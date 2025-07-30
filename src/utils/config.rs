use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use super::{
    errors::{SherlockError, SherlockErrorType},
    files::{expand_path, home_dir},
    paths,
};
use crate::{actions::util::parse_default_browser, loader::Loader, sherlock_error, CONFIG};

#[derive(Clone, Debug, Default)]
pub struct SherlockFlags {
    pub config_dir: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub fallback: Option<PathBuf>,
    pub style: Option<PathBuf>,
    pub ignore: Option<PathBuf>,
    pub alias: Option<PathBuf>,
    pub display_raw: bool,
    pub center_raw: bool,
    pub cache: Option<PathBuf>,
    pub daemonize: bool,
    pub method: Option<String>,
    pub field: Option<String>,
    pub sub_menu: Option<String>,
    pub multi: bool,
    pub photo_mode: bool,
    pub input: Option<bool>,
}
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
}
impl SherlockConfig {
    pub fn default() -> Self {
        SherlockConfig {
            default_apps: ConfigDefaultApps::default(),
            units: ConfigUnits::default(),
            debug: ConfigDebug::default(),
            appearance: ConfigAppearance::default(),
            behavior: ConfigBehavior::default(),
            binds: ConfigBinds::default(),
            files: ConfigFiles::default(),
            runtime: Runtime {
                method: None,
                multi: false,
                center: false,
                photo_mode: false,
                display_raw: false,
                input: None,
            },
            expand: ConfigExpand::default(),
            backdrop: ConfigBackdrop::default(),
        }
    }
    pub fn with_root(root: &PathBuf) -> Self {
        SherlockConfig {
            default_apps: ConfigDefaultApps::default(),
            units: ConfigUnits::default(),
            debug: ConfigDebug::default(),
            appearance: ConfigAppearance::with_root(root),
            behavior: ConfigBehavior::default(),
            binds: ConfigBinds::default(),
            files: ConfigFiles::with_root(root),
            runtime: Runtime {
                method: None,
                multi: false,
                center: false,
                photo_mode: false,
                display_raw: false,
                input: None,
            },
            expand: ConfigExpand::default(),
            backdrop: ConfigBackdrop::default(),
        }
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
    #[sherlock_macro::timing(name = "Loading config")]
    pub fn from_flags(
        sherlock_flags: &SherlockFlags,
    ) -> Result<(SherlockConfig, Vec<SherlockError>), SherlockError> {
        // Get location of config file
        let config_dir = sherlock_flags
            .config_dir
            .clone()
            .unwrap_or(paths::get_config_dir()?);
        let mut path = match &sherlock_flags.config {
            Some(path) => {
                let home = home_dir()?;
                expand_path(path, &home)
            }
            _ => config_dir.join("config.toml"),
        };

        // logic to either use json or toml
        let mut filetype: String = String::new();
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            match ext.as_ref() {
                "json" => {
                    if !path.exists() {
                        path.set_extension("toml");
                        filetype = "toml".to_string();
                    } else {
                        filetype = "json".to_string();
                    }
                }
                "toml" => {
                    if !path.exists() {
                        path.set_extension("json");
                        filetype = "json".to_string();
                    } else {
                        filetype = "toml".to_string();
                    }
                }
                _ => {}
            }
        } else {
            return Err(sherlock_error!(
                SherlockErrorType::FileParseError(path.clone()),
                format!(
                    "The file \"{}\" is not in a valid format.",
                    &path.to_string_lossy()
                )
            ));
        }

        match fs::read_to_string(&path) {
            Ok(config_str) => {
                let config_res: Result<SherlockConfig, SherlockError> = match filetype.as_str() {
                    "json" => {
                        let mut bytes = config_str.into_bytes();
                        simd_json::from_slice(&mut bytes).map_err(|e| {
                            sherlock_error!(
                                SherlockErrorType::FileParseError(path.clone()),
                                e.to_string()
                            )
                        })
                    }
                    "toml" => toml::de::from_str(&config_str).map_err(|e| {
                        sherlock_error!(
                            SherlockErrorType::FileParseError(path.clone()),
                            e.to_string()
                        )
                    }),
                    _ => {
                        return Err(sherlock_error!(
                            SherlockErrorType::FileParseError(path.clone()),
                            format!(
                                "The file \"{}\" is not in a valid format.",
                                &path.to_string_lossy()
                            )
                        ))
                    }
                };
                match config_res {
                    Ok(mut config) => {
                        config = SherlockConfig::apply_flags(sherlock_flags, config);
                        Ok((config, vec![]))
                    }
                    Err(e) => {
                        let mut config = SherlockConfig::default();

                        config = SherlockConfig::apply_flags(sherlock_flags, config);
                        Ok((config, vec![e]))
                    }
                }
            }
            Err(e) => {
                let mut config = SherlockConfig::default();
                config = SherlockConfig::apply_flags(sherlock_flags, config);
                let e = sherlock_error!(
                    SherlockErrorType::FileReadError(path),
                    e.to_string()
                );
                Ok((config, vec![e]))
            },
        }
    }
    pub fn apply_flags(
        sherlock_flags: &SherlockFlags,
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
        config.behavior.sub_menu = sherlock_flags.sub_menu.clone();
        config.runtime.method = sherlock_flags.method.clone();
        config.runtime.input = sherlock_flags.input.clone();
        config.runtime.center = sherlock_flags.center_raw.clone();
        config.runtime.multi = sherlock_flags.multi;
        config.runtime.display_raw = sherlock_flags.display_raw;
        config.runtime.photo_mode = sherlock_flags.photo_mode;
        config.behavior.field = sherlock_flags.field.clone();

        if sherlock_flags.daemonize {
            config.behavior.daemonize = true;
        }
        config
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigDefaultApps {
    #[serde(default = "default_teams")]
    pub teams: String,
    #[serde(default = "default_calendar_client")]
    pub calendar_client: String,
    #[serde(default = "default_terminal")]
    pub terminal: String,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub mpris: Option<String>,
}
impl Default for ConfigDefaultApps {
    fn default() -> Self {
        Self {
            teams: default_teams(),
            calendar_client: default_calendar_client(),
            terminal: get_terminal().unwrap_or_default(), // Should never get to this...
            browser: parse_default_browser().ok(),
            mpris: None,
        }
    }
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigUnits {
    #[serde(default = "default_measurements")]
    pub lengths: String,
    #[serde(default = "default_weights")]
    pub weights: String,
    #[serde(default = "default_volumes")]
    pub volumes: String,
    #[serde(default = "default_temperatures")]
    pub temperatures: String,
    #[serde(default = "default_currency")]
    pub currency: String,
}
impl Default for ConfigUnits {
    fn default() -> Self {
        Self {
            lengths: default_measurements(),
            weights: default_weights(),
            volumes: default_volumes(),
            temperatures: default_temperatures(),
            currency: default_currency(),
        }
    }
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
impl Default for ConfigDebug {
    fn default() -> Self {
        Self {
            try_suppress_errors: false,
            try_suppress_warnings: false,
            app_paths: HashSet::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigAppearance {
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub gsk_renderer: String,
    #[serde(default = "default_icon_paths")]
    pub icon_paths: Vec<PathBuf>,
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    #[serde(default)]
    pub search_icon: bool,
    #[serde(default = "default_true")]
    pub use_base_css: bool,
    #[serde(default = "default_true")]
    pub status_bar: bool,
    #[serde(default = "default_1")]
    pub opacity: f64,
    #[serde(default = "default_modkey_ascii")]
    pub mod_key_ascii: Vec<String>,
    #[serde(default = "default_search_icon")]
    pub search_bar_icon: String,
    #[serde(default = "default_search_icon_back")]
    pub search_bar_icon_back: String,
    #[serde(default = "default_icon_size")]
    pub search_icon_size: i32,
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
        let icon_paths: Vec<PathBuf> = default_icon_paths()
            .into_iter()
            .filter_map(|s| use_root(root, s))
            .collect();
        let mut default = Self::default();
        default.icon_paths = icon_paths;
        default
    }
}
impl Default for ConfigAppearance {
    fn default() -> Self {
        Self {
            width: 900,
            height: 593, // 617 with, 593 without notification bar
            gsk_renderer: String::from("cairo"),
            icon_paths: default_icon_paths(),
            icon_size: default_icon_size(),
            search_icon: false,
            use_base_css: true,
            status_bar: true,
            opacity: 1.0,
            mod_key_ascii: default_modkey_ascii(),
            search_bar_icon: default_search_icon(),
            search_bar_icon_back: default_search_icon_back(),
            search_icon_size: default_icon_size(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBehavior {
    #[serde(default)]
    pub use_xdg_data_dir_icons: bool,
    #[serde(default = "default_cache")]
    pub cache: PathBuf,
    #[serde(default = "default_true")]
    pub caching: bool,
    #[serde(default)]
    pub daemonize: bool,
    #[serde(default = "default_true")]
    pub animate: bool,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub global_prefix: Option<String>,
    #[serde(default)]
    pub global_flags: Option<String>,
    #[serde(default)]
    pub sub_menu: Option<String>,
}
impl Default for ConfigBehavior {
    fn default() -> Self {
        Self {
            use_xdg_data_dir_icons: false,
            cache: default_cache(),
            caching: false,
            daemonize: false,
            animate: true,
            field: None,
            global_prefix: None,
            global_flags: None,
            sub_menu: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigFiles {
    #[serde(default = "default_config")]
    pub config: PathBuf,
    #[serde(default = "default_css")]
    pub css: PathBuf,
    #[serde(default = "default_fallback")]
    pub fallback: PathBuf,
    #[serde(default = "default_alias")]
    pub alias: PathBuf,
    #[serde(default = "default_ignore")]
    pub ignore: PathBuf,
    #[serde(default = "default_actions")]
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
            config: use_root(&root, default_config()),
            css: use_root(&root, default_css()),
            fallback: use_root(&root, default_fallback()),
            alias: use_root(&root, default_alias()),
            ignore: use_root(&root, default_ignore()),
            actions: use_root(&root, default_actions()),
        }
    }
}
impl Default for ConfigFiles {
    fn default() -> Self {
        Self {
            config: default_config(),
            css: default_css(),
            fallback: default_fallback(),
            alias: default_alias(),
            ignore: default_ignore(),
            actions: default_actions(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ConfigBinds {
    #[serde(default)]
    pub prev: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default = "default_context")]
    pub context: Option<String>,
    #[serde(default)]
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
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigExpand {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_backdrop_edge")]
    pub edge: String,
    #[serde(default)]
    pub margin: i32,
}
impl Default for ConfigExpand {
    fn default() -> Self {
        Self {
            enable: false,
            edge: default_backdrop_edge(),
            margin: 0,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBackdrop {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_backdrop_opacity")]
    pub opacity: f64,
    #[serde(default = "default_backdrop_edge")]
    pub edge: String,
}
impl Default for ConfigBackdrop {
    fn default() -> Self {
        Self {
            enable: false,
            opacity: default_backdrop_opacity(),
            edge: default_backdrop_edge(),
        }
    }
}

// ====================
// SECTION: DEFAULT GETTERS
// ====================
pub fn default_terminal() -> String {
    get_terminal().unwrap_or_default()
}
pub fn default_teams() -> String {
    String::from("teams-for-linux --enable-features=UseOzonePlatform --ozone-platform=wayland --url {meeting_url}")
}
pub fn default_calendar_client() -> String {
    String::from("thunderbird")
}
pub fn default_measurements() -> String {
    String::from("meters")
}
pub fn default_weights() -> String {
    String::from("kg")
}
pub fn default_volumes() -> String {
    String::from("l")
}
pub fn default_temperatures() -> String {
    String::from("C")
}
pub fn default_currency() -> String {
    String::from("eur")
}

pub fn default_cache() -> PathBuf {
    paths::get_cache_dir()
        .unwrap()
        .join("sherlock_desktop_cache.json")
}
pub fn default_config() -> PathBuf {
    paths::get_config_dir().unwrap().join("config.toml")
}
pub fn default_fallback() -> PathBuf {
    paths::get_config_dir().unwrap().join("fallback.json")
}
pub fn default_css() -> PathBuf {
    paths::get_config_dir().unwrap().join("main.css")
}
pub fn default_alias() -> PathBuf {
    paths::get_config_dir().unwrap().join("sherlock_alias.json")
}
pub fn default_ignore() -> PathBuf {
    paths::get_config_dir().unwrap().join("sherlockignore")
}
pub fn default_actions() -> PathBuf {
    paths::get_config_dir()
        .unwrap()
        .join("sherlock_actions.json")
}

pub fn default_true() -> bool {
    true
}
pub fn default_1() -> f64 {
    1.0
}
pub fn default_backdrop_opacity() -> f64 {
    0.6
}
pub fn default_backdrop_edge() -> String {
    String::from("top")
}
pub fn default_icon_paths() -> Vec<PathBuf> {
    vec![paths::get_config_dir()
        .unwrap()
        .join("icons/")
        .to_path_buf()]
}
pub fn default_icon_size() -> i32 {
    22
}
pub fn default_search_icon() -> String {
    String::from("system-search-symbolic")
}
pub fn default_search_icon_back() -> String {
    String::from("go-previous-symbolic")
}
pub fn default_modkey_ascii() -> Vec<String> {
    vec![
        String::from("⇧"), // shift
        String::from("⇧"), // lock
        String::from("⌘"), // control
        String::from("⌘"), // meta
        String::from("⎇"), // alt
        String::from("✦"), // super
        String::from("✦"), // hyper
        String::from("⌘"), // default
    ]
}
pub fn default_context() -> Option<String> {
    Some(String::from("control-l"))
}
pub fn get_terminal() -> Result<String, SherlockError> {
    let mut terminal = None;

    //Check if $TERMAINAL is set
    if let Ok(term) = env::var("TERMINAL") {
        if is_terminal_installed(&term) {
            terminal = Some(term);
        }
    }
    // Try other terminals
    if terminal.is_none() {
        let terminals = [
            "kitty",
            "gnome-terminal",
            "xterm",
            "konsole",
            "alacritty",
            "urxvt",
            "mate-terminal",
            "terminator",
            "sakura",
            "terminology",
            "st",
            "xfce4-terminal",
            "guake",
            "x11-terminal",
            "macos-terminal",
            "iterm2",
            "lxterminal",
            "foot",
            "wezterm",
            "tilix",
        ];
        for t in terminals {
            if is_terminal_installed(t) {
                terminal = Some(t.to_string());
                break;
            }
        }
    }
    if let Some(t) = terminal {
        Ok(t)
    } else {
        Err(sherlock_error!(
                SherlockErrorType::ConfigError(Some("Failed to get terminal".to_string())),
                "Unable to locate or parse a valid terminal app. Ensure that the terminal app is correctly specified in the configuration file or environment variables."
        ))
    }
}
fn is_terminal_installed(terminal: &str) -> bool {
    Command::new(terminal).arg("--version").output().is_ok()
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
