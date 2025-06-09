use gtk4::gdk::Display;
use gtk4::CssProvider;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use super::Loader;
use crate::launcher::theme_picker::ThemePicker;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::{sherlock_error, CONFIG};

impl Loader {
    pub fn load_css() -> Result<Vec<SherlockError>, SherlockError> {
        let mut non_breaking: Vec<SherlockError> = Vec::new();
        let provider = CssProvider::new();

        let config = CONFIG
            .get()
            .ok_or_else(|| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;
        let display = Display::default().ok_or_else(|| {
            sherlock_error!(SherlockErrorType::DisplayError, "No display available")
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
        let theme = match ThemePicker::get_cached() {
            Ok(loc) => read_to_string(loc).map(|s| PathBuf::from(s.trim())).ok(),
            _ => None,
        }
        .unwrap_or(config.files.css.clone());
        if Path::new(&theme).exists() {
            let usr_provider = CssProvider::new();
            usr_provider.load_from_path(&theme);
            gtk4::style_context_add_provider_for_display(
                &display,
                &usr_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        } else {
            non_breaking.push(sherlock_error!(
                SherlockErrorType::FileExistError(config.files.css.clone()),
                "Using default css"
            ));
        }

        drop(provider);
        Ok(non_breaking)
    }
}
