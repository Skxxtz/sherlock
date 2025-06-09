use super::Loader;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::{sherlock_error, CONFIG};
use gtk4::{gdk::Display, IconTheme};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

impl Loader {
    pub fn load_icon_theme() -> Vec<SherlockError> {
        let mut non_breaking: Vec<SherlockError> = Vec::new();
        if let Some(config) = CONFIG.get() {
            let icon_paths = config.appearance.icon_paths.clone();
            let icon_theme = IconTheme::for_display(Display::default().as_ref().unwrap());
            let home_dir = env::var("HOME")
                .map_err(|e| {
                    non_breaking.push(sherlock_error!(
                        SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
                        e.to_string()
                    ));
                })
                .ok();

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

            if let Some(h) = home_dir {
                icon_paths
                    .iter()
                    .map(|path| path.replace("~", &h))
                    .for_each(|path| icon_theme.add_search_path(path));
            }
        } else {
            non_breaking.push(sherlock_error!(SherlockErrorType::ConfigError(None), ""));
        }
        non_breaking
    }
}
