use crate::loader::icon::render::render_svg_to_cache;
use crate::utils::errors::SherlockMessage;
use crate::utils::errors::types::SherlockErrorType;
use crate::utils::files::home_dir;
use crate::{ICONS, sherlock_msg};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct CustomIconTheme {
    pub buf: HashMap<String, Option<Arc<Path>>>,
}
impl CustomIconTheme {
    pub fn new() -> Self {
        Self {
            buf: HashMap::new(),
        }
    }
    pub fn add_path<T: AsRef<Path>>(&mut self, path: T) {
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
    pub fn lookup_icon(&self, name: &str) -> Option<Option<Arc<Path>>> {
        self.buf.get(name).cloned()
    }
    fn scan_path(path: &Path, buf: &mut HashMap<String, Option<Arc<Path>>>) {
        // Early return if its not a scannable directory
        if !path.exists() || !path.is_dir() {
            return;
        }

        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                Self::scan_path(&entry_path, buf);
            } else if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                let is_icon = matches!(ext.to_ascii_lowercase().as_str(), "png" | "svg");
                if is_icon && let Some(stem) = entry_path.file_stem().and_then(|s| s.to_str()) {
                    let stem = stem.to_string();
                    if let Some(arc_path) = render_svg_to_cache(&stem, entry_path) {
                        buf.entry(stem).or_insert(Some(arc_path));
                    }
                }
            }
        }
    }
}

pub struct IconThemeGuard;
impl<'g> IconThemeGuard {
    fn get_theme() -> Result<&'g RwLock<CustomIconTheme>, SherlockMessage> {
        ICONS.get().ok_or_else(|| {
            sherlock_msg!(
                Error,
                SherlockErrorType::ConfigError("Failed to get ICONS singleton".into()),
                "Config not initialized"
            )
        })
    }

    fn get_read() -> Result<RwLockReadGuard<'g, CustomIconTheme>, SherlockMessage> {
        Self::get_theme()?.read().map_err(|_| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Failed to read icon theme".into()),
                "Failed to acquire write lock on config"
            )
        })
    }

    pub fn get_write() -> Result<RwLockWriteGuard<'g, CustomIconTheme>, SherlockMessage> {
        Self::get_theme()?.write().map_err(|_| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Failed to get mutable icon theme".into()),
                "Failed to acquire write lock on config"
            )
        })
    }

    pub fn _read() -> Result<RwLockReadGuard<'g, CustomIconTheme>, SherlockMessage> {
        Self::get_read()
    }

    pub fn add_path<T: AsRef<Path>>(path: T) -> Result<(), SherlockMessage> {
        let mut inner = Self::get_write()?;
        inner.add_path(path);
        Ok(())
    }

    pub fn lookup_icon(name: &str) -> Result<Option<Option<Arc<Path>>>, SherlockMessage> {
        let inner = Self::get_read()?;
        Ok(inner.lookup_icon(name))
    }

    pub fn _write_key<F>(key_fn: F) -> Result<(), SherlockMessage>
    where
        F: FnOnce(&mut CustomIconTheme),
    {
        let mut config = Self::get_write()?;
        key_fn(&mut config);
        Ok(())
    }
}
