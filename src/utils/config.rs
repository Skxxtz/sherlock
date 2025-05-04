use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::{
    errors::{SherlockError, SherlockErrorType},
    files::{expand_path, home_dir},
};
use crate::{actions::util::parse_default_browser, loader::Loader};

#[derive(Clone, Debug, Default)]
pub struct SherlockFlags {
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
    pub pipe: ConfigPipe,
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
            pipe: ConfigPipe { method: None },
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
            pipe: ConfigPipe { method: None },
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
                if let Err(error) = fs::write(&alias_path, content).map_err(|e| SherlockError {
                    error: SherlockErrorType::FileWriteError(alias_path),
                    traceback: e.to_string(),
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
        let toml_str = toml::to_string(&config).map_err(|e| SherlockError {
            error: SherlockErrorType::FileWriteError(path.clone()),
            traceback: e.to_string(),
        })?;

        // mkdir -p
        fs::create_dir_all(&path).map_err(|e| SherlockError {
            error: SherlockErrorType::DirCreateError(format!("{:?}", path)),
            traceback: e.to_string(),
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
            .map_err(|e| SherlockError {
                error: SherlockErrorType::ResourceLookupError("fallback.json".to_string()),
                traceback: e.to_string(),
            })?;

            let json_str = std::str::from_utf8(&data).map_err(|e| SherlockError {
                error: SherlockErrorType::FileParseError(PathBuf::from("fallback.json")),
                traceback: e.to_string(),
            })?;
            if let Err(error) = fs::write(&fallback_path, json_str).map_err(|e| SherlockError {
                error: SherlockErrorType::FileWriteError(fallback_path),
                traceback: e.to_string(),
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
    #[sherlock_macro::timing("Loading config")]
    pub fn from_flags(
        sherlock_flags: &SherlockFlags,
    ) -> Result<(SherlockConfig, Vec<SherlockError>), SherlockError> {
        let home = home_dir()?;
        let mut path = match &sherlock_flags.config {
            Some(path) => expand_path(path, &home),
            _ => home.join(".config/sherlock/config.toml"),
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
            return Err(SherlockError {
                error: SherlockErrorType::FileParseError(path.clone()),
                traceback: format!(
                    "The file \"{}\" is not in a valid format.",
                    &path.to_string_lossy()
                ),
            });
        }

        match fs::read_to_string(&path) {
            Ok(config_str) => {
                let config_res: Result<SherlockConfig, SherlockError> = match filetype.as_str() {
                    "json" => {
                        let mut bytes = config_str.into_bytes();
                        simd_json::from_slice(&mut bytes).map_err(|e| SherlockError {
                            error: SherlockErrorType::FileParseError(path.clone()),
                            traceback: e.to_string(),
                        })
                    }
                    "toml" => toml::de::from_str(&config_str).map_err(|e| SherlockError {
                        error: SherlockErrorType::FileParseError(path.clone()),
                        traceback: e.to_string(),
                    }),
                    _ => {
                        return Err(SherlockError {
                            error: SherlockErrorType::FileParseError(path.clone()),
                            traceback: format!(
                                "The file \"{}\" is not in a valid format.",
                                &path.to_string_lossy()
                            ),
                        })
                    }
                };
                match config_res {
                    Ok(mut config) => {
                        config = SherlockConfig::apply_flags(sherlock_flags, config);
                        return Ok((config, vec![]));
                    }
                    Err(e) => {
                        let mut config = SherlockConfig::default();

                        config = SherlockConfig::apply_flags(sherlock_flags, config);
                        Ok((config, vec![e]))
                    }
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    let error = SherlockError {
                        error: SherlockErrorType::FileExistError(path),
                        traceback: e.to_string(),
                    };

                    let mut config = SherlockConfig::default();

                    config = SherlockConfig::apply_flags(sherlock_flags, config);
                    Ok((config, vec![error]))
                }
                _ => Err(SherlockError {
                    error: SherlockErrorType::FileReadError(path),
                    traceback: e.to_string(),
                })?,
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
        config.pipe.method = sherlock_flags.method.clone();
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
    pub _currency: String,
}
impl Default for ConfigUnits {
    fn default() -> Self {
        Self {
            lengths: default_measurements(),
            weights: default_weights(),
            volumes: default_volumes(),
            temperatures: default_temperatures(),
            _currency: default_currency(),
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
    pub icon_paths: Vec<String>,
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
}
impl ConfigAppearance {
    fn with_root(root: &PathBuf) -> Self {
        let mut root = root.clone();
        if root.ends_with("/") {
            root.pop();
        }
        let root = root.to_str();
        fn use_root(root: Option<&str>, path: String) -> Option<String> {
            if let Some(root) = root {
                Some(format!(
                    "{}{}",
                    root,
                    path.trim_start_matches("~/.config/sherlock")
                ))
            } else {
                None
            }
        }
        let icon_paths: Vec<String> = default_icon_paths()
            .into_iter()
            .filter_map(|s| use_root(root, s))
            .collect();
        Self {
            width: 900,
            height: 593, // 617 with, 593 without notification bar
            gsk_renderer: String::from("cairo"),
            icon_paths,
            icon_size: default_icon_size(),
            search_icon: false,
            use_base_css: true,
            status_bar: true,
            opacity: 1.0,
            mod_key_ascii: default_modkey_ascii(),
        }
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
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBehavior {
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
    pub global_prefix: Option<String>,
    pub global_flags: Option<String>,
    pub sub_menu: Option<String>,
}
impl Default for ConfigBehavior {
    fn default() -> Self {
        Self {
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
    #[serde(default)]
    pub modifier: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ConfigPipe {
    #[serde(default)]
    pub method: Option<String>,
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
    PathBuf::from("~/.cache/sherlock/sherlock_desktop_cache.json")
}
pub fn default_config() -> PathBuf {
    PathBuf::from("~/.config/sherlock/config.toml")
}
pub fn default_fallback() -> PathBuf {
    PathBuf::from("~/.config/sherlock/fallback.json")
}
pub fn default_css() -> PathBuf {
    PathBuf::from("~/.config/sherlock/main.css")
}
pub fn default_alias() -> PathBuf {
    PathBuf::from("~/.config/sherlock/sherlock_alias.json")
}
pub fn default_ignore() -> PathBuf {
    PathBuf::from("~/.config/sherlock/sherlockignore")
}
pub fn default_actions() -> PathBuf {
    PathBuf::from("~/.config/sherlock/sherlock_actions.json")
}

pub fn default_true() -> bool {
    true
}
pub fn default_1() -> f64 {
    1.0
}
pub fn default_icon_paths() -> Vec<String> {
    vec![String::from("~/.config/sherlock/icons/")]
}
pub fn default_icon_size() -> i32 {
    22
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
        Err(SherlockError{
                error: SherlockErrorType::ConfigError(Some("Failed to get terminal".to_string())),
                traceback: "Unable to locate or parse a valid terminal app. Ensure that the terminal app is correctly specified in the configuration file or environment variables.".to_string(),
            })
    }
}
fn is_terminal_installed(terminal: &str) -> bool {
    Command::new(terminal)
        .arg("--version") // You can adjust this if the terminal doesn't have a "--version" flag
        .output()
        .is_ok()
}
