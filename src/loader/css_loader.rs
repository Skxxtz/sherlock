use gtk4::gdk::Display;
use gtk4::CssProvider;
use std::path::Path;

use super::Loader;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::CONFIG;

impl Loader {
    pub fn load_css() -> Result<Vec<SherlockError>, SherlockError> {
        let mut non_breaking: Vec<SherlockError> = Vec::new();
        let provider = CssProvider::new();

        let config = CONFIG.get().ok_or_else(|| SherlockError {
            error: SherlockErrorType::ConfigError(None),
            traceback: String::new(),
        })?;
        let display = Display::default().ok_or_else(|| SherlockError {
            error: SherlockErrorType::DisplayError,
            traceback: "No display available".to_string(),
        })?;

        // Load the base line css
        if config.appearance.use_base_css {
            provider.load_from_resource("/dev/skxxtz/sherlock/main.css");
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        // Load the user css
        if Path::new(&config.files.css).exists() {
            let usr_provider = CssProvider::new();
            usr_provider.load_from_path(&config.files.css);
            gtk4::style_context_add_provider_for_display(
                &display,
                &usr_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        } else {
            non_breaking.push(SherlockError {
                error: SherlockErrorType::FileExistError(config.files.css.clone()),
                traceback: String::from("Using default css"),
            });
        }

        drop(provider);
        Ok(non_breaking)
    }
}
