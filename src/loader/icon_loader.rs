use super::Loader;
use crate::utils::config::ConfigGuard;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::utils::files::{expand_path, home_dir};
use crate::{sherlock_error, ICONS};
use gtk4::{gdk::Display, IconTheme};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

impl Loader {
    pub async fn load_icon_theme() -> Option<SherlockError> {
        let icon_theme = IconTheme::for_display(Display::default().as_ref().unwrap());

        // Build icon paths off main thread
        let icon_paths = tokio::task::spawn_blocking(move || {
            let mut paths = Vec::new();

            if let Ok(paths_var) = env::var("XDG_DATA_DIRS") {
                paths.extend(
                    paths_var
                    .split(':')
                    .map(|p| PathBuf::from(p).join("icons"))
                    .filter(|p| p.exists()),
                );
            }

            let config = match ConfigGuard::read() {
                Ok(c) => c,
                Err(e) => return Err(e)
            };
            if let Ok(home) = home_dir() {
                paths.extend(
                    config
                    .appearance
                    .icon_paths
                    .iter()
                    .map(|p| expand_path(p, &home))
                    .filter(|p| p.exists()),
                );
            }

            Ok(paths)
        })
        .await.ok()?;

        match icon_paths {
            Ok(p) => add_paths_incrementally(&icon_theme, p),
            Err(e) => return Some(e)
        }

        None
    }
}

fn add_paths_incrementally(icon_theme: &IconTheme, paths: Vec<PathBuf>) {
    let icon_theme = icon_theme.clone();
    let mut paths_iter = paths.into_iter();

    gtk4::glib::idle_add_local(move || {
        if let Some(path) = paths_iter.next() {
            icon_theme.add_search_path(path);
            true.into()
        } else {
            false.into()
        }
    });
}

pub struct CustomIconTheme {
    pub buf: HashMap<String, PathBuf>,
}
impl CustomIconTheme {
    pub fn new() -> Self {
        Self {
            buf: HashMap::new()
        }
    }
    pub fn add_path<T: AsRef<Path>>(&mut self, path: T){
        let path_ref = path.as_ref();

        let path = if let Some(str_path) = path_ref.to_str() {
            if let Some(stripped) = str_path.strip_prefix("~/") {
                if let Ok(home) = home_dir() {
                    home.join(stripped)
                } else {
                    return;
                }
            } else {
                path_ref.to_path_buf()
            }
        } else {
            path_ref.to_path_buf()
        };
        Self::scan_path(&path, &mut self.buf);
    }
    pub fn lookup_icon(&self, name: &str) -> Option<PathBuf> {
        self.buf.get(name).cloned()
    }
    fn scan_path(path: &Path, buf: &mut HashMap<String, PathBuf>){
        // Early return if its not a scannable directory
        if !path.exists() ||!path.is_dir() {
            return
        }

        let Ok(entries) = std::fs::read_dir(path) else { return };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                Self::scan_path(&entry_path, buf);
            } else if let Some(ext) = entry_path.extension().and_then(|e|e.to_str()){
                let is_icon = matches!(ext.to_ascii_lowercase().as_str(), "png" | "svg");
                if is_icon {
                    if let Some(stem) = entry_path.file_stem().and_then(|s| s.to_str()){
                        buf.entry(stem.to_string()).or_insert(entry_path);
                    }
                }
            }
        }

    }
}

pub struct IconThemeGuard;
impl<'g> IconThemeGuard {
    fn get_theme() -> Result<&'g RwLock<CustomIconTheme>, SherlockError> {
        ICONS.get().ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Config not initialized".to_string()
            )
        })
    }

    fn get_read() -> Result<RwLockReadGuard<'g, CustomIconTheme>, SherlockError> {
        Self::get_theme()?.read().map_err(|_| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Failed to acquire write lock on config".to_string()
            )
        })
    }

    fn get_write() -> Result<RwLockWriteGuard<'g, CustomIconTheme>, SherlockError> {
        Self::get_theme()?.write().map_err(|_| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Failed to acquire write lock on config".to_string()
            )
        })
    }

    pub fn _read() -> Result<RwLockReadGuard<'g, CustomIconTheme>, SherlockError> {
        Self::get_read()
    }

    pub fn add_path<T: AsRef<Path>>(path: T) -> Result<(), SherlockError>{
        let mut inner = Self::get_write()?;
        inner.add_path(path);
        Ok(())
    }

    pub fn lookup_icon(name: &str) -> Result<Option<PathBuf>, SherlockError> {
        let inner = Self::get_read()?;
        Ok(inner.lookup_icon(name))
    }

    pub fn _write_key<F>(key_fn: F) -> Result<(), SherlockError>
    where
        F: FnOnce(&mut CustomIconTheme),
    {
        let mut config = Self::get_write()?;
        key_fn(&mut config);
        Ok(())
    }
}
