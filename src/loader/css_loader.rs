use gio::glib::object::ObjectExt;
use gio::glib::WeakRef;
use gtk4::gdk::Display;
use gtk4::CssProvider;
use std::cell::RefCell;
use std::fs;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use super::Loader;
use crate::launcher::theme_picker::ThemePicker;
use crate::utils::config::ConfigGuard;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::{sher_log, sherlock_error};

thread_local! {
    static CURRENT_PROVIDER: RefCell<Option<WeakRef<CssProvider>>> = RefCell::new(None);
}

fn get_provider() -> Option<CssProvider> {
    CURRENT_PROVIDER.with(|cell| cell.borrow().as_ref().and_then(|weak| weak.upgrade()))
}
fn set_provider(provider: WeakRef<CssProvider>) {
    CURRENT_PROVIDER.with(|cell| *cell.borrow_mut() = Some(provider))
}

impl Loader {
    pub async fn load_css(apply_base: bool) -> Result<(), SherlockError> {
        let provider = CssProvider::new();

        let config = ConfigGuard::read()?;
        let display = Display::default().ok_or_else(|| {
            sherlock_error!(SherlockErrorType::DisplayError, "No display available")
        })?;

        if let Some(current_provider) = get_provider() {
            sher_log!("Removed current style provider")?;
            gtk4::style_context_remove_provider_for_display(&display, &current_provider);
        }

        // Load the base line css
        if apply_base && config.appearance.use_base_css {
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
            set_provider(usr_provider.downgrade());
            sher_log!("Added new user style provider")?;
        } else {
            fs::write(&theme, "").map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileWriteError(theme.clone()),
                    e.to_string()
                )
            })?;
        }

        drop(provider);
        Ok(())
    }
}
