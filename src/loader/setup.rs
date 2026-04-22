use std::{
    path::{Path, PathBuf},
    sync::RwLock,
};

use crate::{
    CONFIG, ICONS,
    loader::{CustomIconTheme, IconThemeGuard},
    sherlock_msg,
    utils::{
        config::SherlockConfig,
        errors::{
            SherlockMessage,
            types::{DirAction, SherlockErrorType},
        },
        paths::get_config_dir,
    },
};

use super::Loader;

pub struct SetupResult {
    pub config_dir: Box<Path>,
    pub messages: Vec<SherlockMessage>,
}
impl Loader {
    /// Initializes the application by loading flags, configuration, and icon themes.
    ///
    /// This function is infallible — all errors are collected into the returned
    /// `SetupResult` rather than aborting. The caller decides how to handle them.
    ///
    /// Steps performed:
    /// - Load CLI flags, falling back to defaults on failure
    /// - Resolve user configuration, falling back to `SherlockConfig::default`
    /// - Initialize the global icon cache and register custom icon paths
    /// - Set the global CONFIG static
    /// - Resolve the config root directory, falling back to XDG config dir or `/tmp/sherlock`
    pub fn setup() -> SetupResult {
        let mut messages: Vec<SherlockMessage> = Vec::new();
        let mut flags = Self::load_flags()
            .map_err(|e| messages.push(e))
            .unwrap_or_default();

        let config = match flags.get_config() {
            Err(e) => {
                messages.push(e);
                let defaults = SherlockConfig::default();
                SherlockConfig::apply_flags(&mut flags, defaults)
            }
            Ok((cfg, non_crit)) => {
                messages.extend(non_crit);
                cfg
            }
        };

        let _ = ICONS.set(RwLock::new(CustomIconTheme::new()));
        config.appearance.icon_paths.iter().for_each(|path| {
            if let Err(e) = IconThemeGuard::add_path(path) {
                messages.push(e);
            }
        });

        if CONFIG.set(RwLock::new(config.clone())).is_err() {
            messages.push(sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Failed to set global config singleton.".into()),
                ""
            ));
        }

        let config_dir: Box<Path> = match config.files.config.parent() {
            Some(p) => p.into(),
            None => {
                messages.push(sherlock_msg!(
                    Warning,
                    SherlockErrorType::DirError(DirAction::Find, "Config Root Dir".into()),
                    "Failed to read config root dir."
                ));
                get_config_dir()
                    .unwrap_or(PathBuf::from("/tmp"))
                    .join("sherlock")
                    .into()
            }
        };

        SetupResult {
            config_dir,
            messages,
        }
    }
}
