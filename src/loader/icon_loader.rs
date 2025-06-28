use super::Loader;
use crate::utils::config::ConfigGuard;
use crate::utils::errors::SherlockError;
use crate::utils::files::{expand_path, home_dir};
use gtk4::{gdk::Display, IconTheme};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

impl Loader {
    #[sherlock_macro::timing(name = "Loading Icon Theme", level = "setup")]
    pub fn load_icon_theme() -> Option<SherlockError> {
        let config = match ConfigGuard::read() {
            Ok(c) => c,
            Err(e) => return Some(e),
        };

        let icon_paths = config.appearance.icon_paths.clone();
        let icon_theme = IconTheme::for_display(Display::default().as_ref().unwrap());

        // Add data dirs to icon paths
        match env::var("XDG_DATA_DIRS").ok() {
            Some(paths) => {
                let app_dirs: HashSet<PathBuf> = paths
                    .split(":")
                    .map(|p| PathBuf::from(p).join("icons/"))
                    .collect();
                app_dirs.into_iter().for_each(|path| {
                    icon_theme.add_search_path(path);
                });
            }
            _ => {}
        };

        if let Ok(home) = home_dir() {
            icon_paths
                .into_iter()
                .map(|path| expand_path(&path, &home))
                .for_each(|path| icon_theme.add_search_path(path));
        }

        None
    }
}
