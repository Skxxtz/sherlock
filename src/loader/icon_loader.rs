use super::Loader;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::CONFIG;
use gtk4::{gdk::Display, IconTheme};
use std::env;

impl Loader {
    pub fn load_icon_theme() -> Vec<SherlockError> {
        let mut non_breaking: Vec<SherlockError> = Vec::new();
        if let Some(config) = CONFIG.get() {
            let icon_paths = config.appearance.icon_paths.clone();
            let icon_theme = IconTheme::for_display(Display::default().as_ref().unwrap());
            let home_dir = env::var("HOME")
                .map_err(|e| {
                    non_breaking.push(SherlockError {
                        error: SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
                        traceback: e.to_string(),
                    });
                })
                .ok();

            if let Some(h) = home_dir {
                icon_paths
                    .iter()
                    .map(|path| path.replace("~", &h))
                    .for_each(|path| icon_theme.add_search_path(path));
            }
        } else {
            non_breaking.push(SherlockError {
                error: SherlockErrorType::ConfigError(None),
                traceback: format!(""),
            });
        }
        non_breaking
    }
}
