use linicon::lookup_icon;

use crate::loader::assets::Assets;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::utils::files::home_dir;
use crate::utils::paths::get_cache_dir;
use crate::{ICONS, sherlock_error};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
        self.buf.get(name).map(|p| p.clone())
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
                if is_icon {
                    if let Some(stem) = entry_path.file_stem().and_then(|s| s.to_str()) {
                        let stem = stem.to_string();
                        if let Some(arc_path) = render_svg_to_cache(&stem, entry_path) {
                            buf.entry(stem).or_insert(Some(arc_path));
                        }
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

    pub fn add_path<T: AsRef<Path>>(path: T) -> Result<(), SherlockError> {
        let mut inner = Self::get_write()?;
        inner.add_path(path);
        Ok(())
    }

    pub fn lookup_icon(name: &str) -> Result<Option<Option<Arc<Path>>>, SherlockError> {
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

pub fn resolve_icon_path(name: &str) -> Option<Arc<Path>> {
    // 1. Check in-memory HashMap cache
    if let Ok(Some(icon)) = IconThemeGuard::lookup_icon(name) {
        return icon;
    }

    let mut result: Option<Arc<Path>> = None;

    // Check embedded files
    if let Some(asset) = Assets::get(&format!("icons/{name}.svg")) {
        result = render_to_png_cache(name, &asset.data);
    }

    // Fallback to local linicon lookup (~/.local/share/icons)
    if result.is_none() {
        result = (|| {
            let icon_path = lookup_icon(name)
                .with_search_paths(&["~/.local/share/icons/"])
                .ok()?
                .next()?
                .map(|i| i.path)
                .ok()?;
            render_svg_to_cache(name, icon_path)
        })();
    }

    // Fallback to global Freedesktop lookup
    if result.is_none() {
        result = freedesktop_icons::lookup(name)
            .with_size(128)
            .find()
            .and_then(|i| render_svg_to_cache(name, i));
    }

    // Finalize: Write found result back to the Guard buffer
    if let Ok(mut cache) = IconThemeGuard::get_write() {
        cache.buf.insert(name.to_string(), result.clone());
    }

    result
}

/// Renders an svg icon into a high-resolution png version.
fn render_svg_to_cache(key: &str, path: PathBuf) -> Option<Arc<Path>> {
    // Early return if file does not exist
    if !path.exists() {
        return None;
    }

    // Early return if file is not svg
    if path.extension().map_or(true, |s| s.to_str() != Some("svg")) {
        return Some(Arc::from(path.into_boxed_path()));
    }

    // Read svg
    let svg_data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read SVG file {:?}: {}", path, e);
            return None;
        }
    };

    render_to_png_cache(key, &svg_data)
}

fn render_to_png_cache(key: &str, svg_data: &[u8]) -> Option<Arc<Path>> {
    let mut out = get_cache_dir().ok()?.join("icons");

    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("Warning: Failed to create cache directory: {}", e);
        return None;
    }

    out.push(format!("{}.png", key.replace('/', "_")));

    if out.exists() {
        return Some(Arc::from(out.into_boxed_path()));
    }

    // Parse svg
    let opt = usvg::Options::default();
    let tree = match usvg::Tree::from_data(&svg_data, &opt) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to parse SVG {key}: {e}");
            return None;
        }
    };

    // Scale svg
    let target_height = 96.0;
    let zoom = target_height / tree.size().height();

    let width = (tree.size().width() * zoom).round() as u32;
    let height = (tree.size().height() * zoom).round() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();

    let sx = width as f32 / tree.size().width();
    let sy = height as f32 / tree.size().height();

    // Render
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );

    // Save svg to destination
    if let Err(e) = pixmap.save_png(&out) {
        eprintln!("Warning: Failed to cache file: {e}");
        return None;
    }

    Some(Arc::from(out.into_boxed_path()))
}
