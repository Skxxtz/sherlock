use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use super::{
    errors::{SherlockError, SherlockErrorType},
    files::{expand_path, home_dir},
};
use crate::{loader::Loader, sherlock_error, utils::config::defaults::FileDefaults, CONFIG};

mod defaults;
mod flags;
mod imp;

pub use defaults::{BindDefaults, ConstantDefaults, OtherDefaults};
pub use flags::SherlockFlags;

/// Configuration sections:
///
/// - **default_apps**: User-defined default applications (e.g., terminal, calendar).
/// - **units**: Preferred measurement units (e.g., length, temperature).
/// - **debug**: Debugging preferences (e.g., whether to display errors).
/// - **appearance**: UI preferences (e.g., show/hide status bar).
/// - **behavior**: Runtime behavior settings (e.g., daemon mode, caching).
/// - **binds**: Custom key or action bindings (supplementing defaults).
/// - **files**: User-specified overrides for default config file paths.
/// - **pipe** *(internal)*: Internal settings for JSON piping (e.g., default return action).
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct SherlockConfig {
    /// User-defined default applications (e.g., terminal, calendar)
    #[serde(default)]
    pub default_apps: ConfigDefaultApps,

    /// Preferred measurement units (e.g., length, temperature)
    #[serde(default)]
    pub units: ConfigUnits,

    /// Debugging preferences (e.g., whether to display errors)
    #[serde(default)]
    pub debug: ConfigDebug,

    /// UI preferences (e.g., show/hide status bar)
    #[serde(default)]
    pub appearance: ConfigAppearance,

    /// Runtime behavior settings (e.g., daemon mode, caching)
    #[serde(default)]
    pub behavior: ConfigBehavior,

    /// Custom key or action bindings (supplementing defaults)
    #[serde(default)]
    pub binds: ConfigBinds,

    /// User-specified overrides for default config file paths
    #[serde(default)]
    pub files: ConfigFiles,

    /// Internal settings for JSON piping (e.g., default return action)
    #[serde(default)]
    pub runtime: Runtime,

    /// Configures expand feature
    #[serde(default)]
    pub expand: ConfigExpand,

    /// Configures backdrop feature
    #[serde(default)]
    pub backdrop: ConfigBackdrop,

    /// Configures the status bar
    #[serde(default)]
    pub status_bar: StatusBar,

    /// Configures search bar icons
    #[serde(default)]
    pub search_bar_icon: SearchBarIcon,
}
impl SherlockConfig {
    pub fn with_root(root: &PathBuf) -> Self {
        let mut default = SherlockConfig::default();
        default.files = ConfigFiles::with_root(root);
        default.appearance = ConfigAppearance::with_root(root);
        default
    }
    /// # Arguments
    /// loc: PathBuf
    /// Pathbuf should be a directory **not** a file
    pub fn to_file(loc: PathBuf) -> Result<(), SherlockError> {
        // create config location
        let home = home_dir()?;
        let path = expand_path(&loc, &home);

        fn ensure_dir(path: &Path, label: &str) {
            match std::fs::create_dir(path) {
                Ok(_) => println!("✓ Created '{}' directory", label),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    println!("↷ Skipping '{}' — directory already exists.", label)
                }
                Err(e) => eprintln!("✗ Failed to create '{}' directory: {}", label, e),
            }
        }
        fn created_message(name: &str) {
            println!("✓ Created '{}'", name);
        }
        fn skipped_message(name: &str) {
            println!("↷ Skipping '{}' since file exists already.", name);
        }
        fn error_message(name: &str, reason: SherlockError) {
            eprintln!(
                "✗ Failed to create '{}'. Reason: {}",
                name,
                reason.error.get_message().0
            );
        }
        let write_file = |name: &str, content: &str| {
            let alias_path = path.join(name);
            if !alias_path.exists() {
                if let Err(error) = fs::write(&alias_path, content).map_err(|e| {
                    sherlock_error!(SherlockErrorType::FileWriteError(alias_path), e.to_string())
                }) {
                    error_message(name, error);
                } else {
                    created_message(name);
                }
            } else {
                skipped_message(name);
            }
        };

        // build default config
        let config = SherlockConfig::with_root(&loc);
        let toml_str = toml::to_string(&config).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileWriteError(path.clone()),
                e.to_string()
            )
        })?;

        // mkdir -p
        fs::create_dir_all(&path).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::DirCreateError(format!("{:?}", path)),
                e.to_string()
            )
        })?;
        // create subdirs
        ensure_dir(&path.join("icons/"), "icons");
        ensure_dir(&path.join("scripts/"), "scripts");
        ensure_dir(&path.join("themes/"), "themes");

        // write config.toml file
        write_file("config.toml", &toml_str);

        // write sherlockignore file
        write_file("sherlockignore", "");

        // write sherlock_actions file
        write_file("sherlock_actions.json", "[]");

        // write sherlock_alias file
        write_file("sherlock_alias.json", "{}");

        // write main.css file
        write_file("main.css", "");

        // write fallback.json file
        let fallback_path = path.join("fallback.json");
        if !fallback_path.exists() {
            // load resources
            Loader::load_resources()?;
            let data = gio::resources_lookup_data(
                "/dev/skxxtz/sherlock/fallback.json",
                gio::ResourceLookupFlags::NONE,
            )
            .map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::ResourceLookupError("fallback.json".to_string()),
                    e.to_string()
                )
            })?;

            let json_str = std::str::from_utf8(&data).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileParseError(PathBuf::from("fallback.json")),
                    e.to_string()
                )
            })?;
            if let Err(error) = fs::write(&fallback_path, json_str).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileWriteError(fallback_path),
                    e.to_string()
                )
            }) {
                error_message("fallback.json", error);
            } else {
                created_message("fallback.json");
            };
        } else {
            skipped_message("fallback.json");
        }

        if let Some(loc) = loc.to_str() {
            if loc != "~/.config/sherlock/" {
                let loc = loc.trim_end_matches("/");
                println!("\nUse \x1b[32msherlock --config {}/config.toml\x1b[0m to run sherlock with the custom configuration.", loc);
            }
        }

        std::process::exit(0);
    }
    pub fn apply_flags(
        sherlock_flags: &mut SherlockFlags,
        mut config: SherlockConfig,
    ) -> SherlockConfig {
        // Make paths that contain the ~ dir use the correct path
        let home = match home_dir() {
            Ok(h) => h,
            Err(_) => return config,
        };

        // Override config files from flags
        config.files.config = expand_path(
            &sherlock_flags
                .config
                .as_deref()
                .unwrap_or(&config.files.config),
            &home,
        );
        config.files.fallback = expand_path(
            &sherlock_flags
                .fallback
                .as_deref()
                .unwrap_or(&config.files.fallback),
            &home,
        );
        config.files.css = expand_path(
            &sherlock_flags.style.as_deref().unwrap_or(&config.files.css),
            &home,
        );
        config.files.alias = expand_path(
            &sherlock_flags
                .alias
                .as_deref()
                .unwrap_or(&config.files.alias),
            &home,
        );
        config.files.ignore = expand_path(
            &sherlock_flags
                .ignore
                .as_deref()
                .unwrap_or(&config.files.ignore),
            &home,
        );
        config.behavior.cache = expand_path(
            &sherlock_flags
                .cache
                .as_deref()
                .unwrap_or(&config.behavior.cache),
            &home,
        );
        config.runtime.sub_menu = sherlock_flags.sub_menu.take();
        config.runtime.method = sherlock_flags.method.take();
        config.runtime.input = sherlock_flags.input.take();
        config.runtime.center = sherlock_flags.center_raw;
        config.runtime.multi = sherlock_flags.multi;
        config.runtime.display_raw = sherlock_flags.display_raw;
        config.runtime.photo_mode = sherlock_flags.photo_mode;
        config.behavior.field = sherlock_flags.field.take();

        if sherlock_flags.daemonize {
            config.behavior.daemonize = true;
        }
        config
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigDefaultApps {
    #[serde(default = "ConstantDefaults::teams")]
    pub teams: String,
    #[serde(default = "ConstantDefaults::calendar_client")]
    pub calendar_client: String,
    #[serde(default = "ConstantDefaults::terminal")]
    pub terminal: String,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub mpris: Option<String>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigUnits {
    #[serde(default = "ConstantDefaults::lengths")]
    pub lengths: String,
    #[serde(default = "ConstantDefaults::weights")]
    pub weights: String,
    #[serde(default = "ConstantDefaults::volumes")]
    pub volumes: String,
    #[serde(default = "ConstantDefaults::temperatures")]
    pub temperatures: String,
    #[serde(default = "ConstantDefaults::currency")]
    pub currency: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigDebug {
    #[serde(default)]
    pub try_suppress_errors: bool,
    #[serde(default)]
    pub try_suppress_warnings: bool,
    #[serde(default)]
    pub app_paths: HashSet<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigAppearance {
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub gsk_renderer: String,
    #[serde(default = "FileDefaults::icon_paths")]
    pub icon_paths: Vec<PathBuf>,
    #[serde(default = "OtherDefaults::icon_size")]
    pub icon_size: i32,
    #[serde(default = "OtherDefaults::bool_true")]
    pub use_base_css: bool,
    #[serde(default = "OtherDefaults::one")]
    pub opacity: f64,
    #[serde(default = "BindDefaults::modkey_ascii")]
    pub mod_key_ascii: Vec<String>,
}
impl ConfigAppearance {
    fn with_root(root: &PathBuf) -> Self {
        let mut root = root.clone();
        if root.ends_with("/") {
            root.pop();
        }
        let root = root.to_str();
        fn use_root(root: Option<&str>, path: PathBuf) -> Option<PathBuf> {
            let root = root?;
            let home = home_dir().ok()?;
            let base = home.join(".config/sherlock");

            if let Ok(suffix) = path.strip_prefix(&base) {
                Some(Path::new(root).join(suffix))
            } else {
                None
            }
        }
        let icon_paths: Vec<PathBuf> = FileDefaults::icon_paths()
            .into_iter()
            .filter_map(|s| use_root(root, s))
            .collect();
        let mut default = Self::default();
        default.icon_paths = icon_paths;
        default
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBehavior {
    #[serde(default)]
    pub use_xdg_data_dir_icons: bool,
    #[serde(default = "FileDefaults::cache")]
    pub cache: PathBuf,
    #[serde(default = "OtherDefaults::bool_true")]
    pub caching: bool,
    #[serde(default)]
    pub daemonize: bool,
    #[serde(default = "OtherDefaults::bool_true")]
    pub animate: bool,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub global_prefix: Option<String>,
    #[serde(default)]
    pub global_flags: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigFiles {
    #[serde(default = "FileDefaults::config")]
    pub config: PathBuf,
    #[serde(default = "FileDefaults::css")]
    pub css: PathBuf,
    #[serde(default = "FileDefaults::fallback")]
    pub fallback: PathBuf,
    #[serde(default = "FileDefaults::alias")]
    pub alias: PathBuf,
    #[serde(default = "FileDefaults::ignore")]
    pub ignore: PathBuf,
    #[serde(default = "FileDefaults::actions")]
    pub actions: PathBuf,
}
impl ConfigFiles {
    pub fn with_root(root: &PathBuf) -> Self {
        let mut root = root.clone();
        if root.ends_with("/") {
            root.pop();
        }
        fn use_root(root: &PathBuf, path: PathBuf) -> PathBuf {
            if let Ok(stripped) = path.strip_prefix("~/.config/sherlock") {
                root.join(stripped)
            } else {
                path
            }
        }

        Self {
            config: use_root(&root, FileDefaults::config()),
            css: use_root(&root, FileDefaults::css()),
            fallback: use_root(&root, FileDefaults::fallback()),
            alias: use_root(&root, FileDefaults::alias()),
            ignore: use_root(&root, FileDefaults::ignore()),
            actions: use_root(&root, FileDefaults::actions()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBinds {
    #[serde(default)]
    pub up: Option<String>,
    #[serde(default)]
    pub down: Option<String>,
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    #[serde(default = "BindDefaults::context")]
    pub context: Option<String>,
    #[serde(default = "BindDefaults::modifier")]
    pub modifier: Option<String>,
    #[serde(default)]
    pub exec_inplace: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Runtime {
    #[serde(default)]
    pub method: Option<String>,

    #[serde(default)]
    pub multi: bool,

    #[serde(default)]
    pub center: bool,

    #[serde(default)]
    pub photo_mode: bool,

    #[serde(default)]
    pub display_raw: bool,

    #[serde(default)]
    pub input: Option<bool>,

    #[serde(default)]
    pub sub_menu: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigExpand {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "OtherDefaults::backdrop_edge")]
    pub edge: String,
    #[serde(default)]
    pub margin: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBackdrop {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "OtherDefaults::backdrop_opacity")]
    pub opacity: f64,
    #[serde(default = "OtherDefaults::backdrop_edge")]
    pub edge: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchBarIcon {
    #[serde(default = "OtherDefaults::bool_true")]
    pub enable: bool,

    #[serde(default = "OtherDefaults::search_icon")]
    pub icon: String,

    #[serde(default = "OtherDefaults::search_icon_back")]
    pub icon_back: String,

    #[serde(default = "OtherDefaults::icon_size")]
    pub size: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StatusBar {
    #[serde(default = "OtherDefaults::bool_true")]
    pub enable: bool,
}

pub struct ConfigGuard;
impl<'g> ConfigGuard {
    fn get_config() -> Result<&'g RwLock<SherlockConfig>, SherlockError> {
        CONFIG.get().ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Config not initialized".to_string()
            )
        })
    }

    fn get_read() -> Result<RwLockReadGuard<'g, SherlockConfig>, SherlockError> {
        Self::get_config()?.read().map_err(|_| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Failed to acquire write lock on config".to_string()
            )
        })
    }

    fn _get_write() -> Result<RwLockWriteGuard<'g, SherlockConfig>, SherlockError> {
        Self::get_config()?.write().map_err(|_| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Failed to acquire write lock on config".to_string()
            )
        })
    }

    pub fn read() -> Result<RwLockReadGuard<'g, SherlockConfig>, SherlockError> {
        Self::get_read()
    }

    pub fn write_key<F>(key_fn: F) -> Result<(), SherlockError>
    where
        F: FnOnce(&mut SherlockConfig),
    {
        let mut config = Self::_get_write()?;
        key_fn(&mut config);
        Ok(())
    }
}
