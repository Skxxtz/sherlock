use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, Debug)]
pub struct CommandConfig {
    pub name: String,
    pub alias: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub display_name: Option<String>,
    pub on_return: Option<String>,
    pub next_content: Option<String>,
    pub r#type: String,
    pub priority: f32,

    #[serde(default = "default_true")]
    pub shortcut: bool,
    #[serde(default = "default_true")]
    pub spawn_focus: bool,
    #[serde(default)]
    pub r#async: bool,
    #[serde(default)]
    pub home: bool,
    #[serde(default)]
    pub only_home: bool,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppData {
    pub icon: String,
    pub exec: String,
    pub search_string: String,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub desktop_file: Option<PathBuf>,
    #[serde(default)]
    pub priority: f32,
}

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
}

#[derive(Deserialize, Clone, Debug)]
pub struct SherlockAlias {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub keywords: Option<String>,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SherlockErrorType {
    EnvVarNotFoundError(String),
    FileExistError(PathBuf),
    FileWriteError(PathBuf),
    FileReadError(PathBuf),
    FileParseError(PathBuf),
    DirReadError(String),
    DirCreateError(String),
    ResourceParseError,
    ResourceLookupError(String),
    DisplayError,
    ConfigError(Option<String>),
    FlagLoadError,
    RegexError(String),
    CommandExecutionError(String),
    ClipboardError,
    DBusConnectionError,
    DBusMessageSendError(String),
    DBusMessageConstructError(String),
    HttpRequestError(String),
    SocketRemoveError(String),
}

impl SherlockErrorType {
    pub fn get_message(&self) -> (String, String) {
        match self {
            SherlockErrorType::EnvVarNotFoundError(var) => (
                "EnvVarNotFoundError".to_string(),
                format!("Failed to unpack environment variable \"{}\"", var),
            ),
            SherlockErrorType::SocketRemoveError(socket) => (
                "SocketRemoveError".to_string(),
                format!("Failed to close socket at location \"{}\"", socket),
            ),
            SherlockErrorType::FileExistError(file) => (
                "FileExistError".to_string(),
                format!("File \"{}\" does not exist", file.to_string_lossy()),
            ),
            SherlockErrorType::FileWriteError(file) => (
                "FileWriteError".to_string(),
                format!("Failed to write file \"{}\"", file.to_string_lossy()),
            ),
            SherlockErrorType::FileReadError(file) => (
                "FileReadError".to_string(),
                format!("Failed to read file \"{}\"", file.to_string_lossy()),
            ),
            SherlockErrorType::FileParseError(file) => (
                "FileParseError".to_string(),
                format!("Failed to parse file \"{}\"", file.to_string_lossy()),
            ),
            SherlockErrorType::DirReadError(file) => (
                "DirReadError".to_string(),
                format!("Failed to read/access dir \"{}\"", file),
            ),
            SherlockErrorType::DirCreateError(file) => (
                "DirCreateError".to_string(),
                format!("Failed to create parent dir \"{}\"", file),
            ),
            SherlockErrorType::ResourceParseError => (
                "ResourceParseError".to_string(),
                format!("Failed to parse resources"),
            ),
            SherlockErrorType::ResourceLookupError(resource) => (
                "ResourceLookupError".to_string(),
                format!("Failed to find resource \"{}\"", resource),
            ),
            SherlockErrorType::DisplayError => (
                "DisplayError".to_string(),
                "Could not connect to a display".to_string(),
            ),
            SherlockErrorType::ConfigError(val) => {
                let message = if let Some(v) = val {
                    format!("{}", v)
                } else {
                    "It should never come to this".to_string()
                };
                ("ConfigError".to_string(), message)
            }
            SherlockErrorType::FlagLoadError => {
                (format!("FlagLoadError"), format!("Failed to load flags"))
            }
            SherlockErrorType::RegexError(key) => (
                format!("RegexError"),
                format!("Failed to compile the regular expression for \"{}\"", key),
            ),
            SherlockErrorType::CommandExecutionError(cmd) => (
                format!("CommandExecutionError"),
                format!("Failed to execute command \"{}\"", cmd),
            ),
            SherlockErrorType::ClipboardError => (
                format!("ClipboardError"),
                format!("Failed to get system clipboard"),
            ),
            SherlockErrorType::DBusConnectionError => (
                format!("DBusConnectionError"),
                format!("Failed to connect to system DBus"),
            ),
            SherlockErrorType::DBusMessageConstructError(message) => (
                format!("DBusMessageConstructError"),
                format!("Failed to construct Dbus message \"{}\"", message),
            ),
            SherlockErrorType::DBusMessageSendError(message) => (
                format!("DBusConnectionError"),
                format!("Failed to send Dbus message \"{}\"", message),
            ),
            SherlockErrorType::HttpRequestError(cmd) => (
                format!("HttpRequestError"),
                format!("Failed to get requested source \"{}\"", cmd),
            ),
        }
    }
}
#[derive(Clone, Debug)]
pub struct SherlockError {
    pub error: SherlockErrorType,
    pub traceback: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct SherlockConfig {
    #[serde(default)]
    pub default_apps: ConfigDefaultApps,
    #[serde(default)]
    pub units: ConfigUnits,
    #[serde(default)]
    pub debug: ConfigDebug,
    #[serde(default)]
    pub appearance: ConfigAppearance,
    #[serde(default)]
    pub behavior: ConfigBehavior,
    #[serde(default)]
    pub binds: ConfigBinds,
    #[serde(default)]
    pub files: ConfigFiles,
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
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigDefaultApps {
    #[serde(default = "default_teams")]
    pub teams: String,
    #[serde(default = "default_calendar_client")]
    pub calendar_client: String,
    #[serde(default = "default_terminal")]
    pub terminal: String,
}
impl Default for ConfigDefaultApps {
    fn default() -> Self {
        Self {
            teams: default_teams(),
            calendar_client: default_calendar_client(),
            terminal: get_terminal().unwrap_or_default(), // Should never get to this...
        }
    }
}
#[derive(Deserialize, Debug, Clone)]
pub struct ConfigUnits {
    #[serde(default = "default_measurements")]
    pub lengths: String,
    #[serde(default = "default_weights")]
    pub weights: String,
    #[serde(default = "default_volumes")]
    pub volumes: String,
    #[serde(default = "default_currency")]
    pub _currency: String,
}
impl Default for ConfigUnits {
    fn default() -> Self {
        Self {
            lengths: default_measurements(),
            weights: default_weights(),
            volumes: default_volumes(),
            _currency: default_currency(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigAppearance {
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub gsk_renderer: String,
    #[serde(default)]
    pub icon_paths: Vec<String>,
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    #[serde(default)]
    pub search_icon: bool,
    #[serde(default = "default_true")]
    pub use_base_css: bool,
}
impl Default for ConfigAppearance {
    fn default() -> Self {
        Self {
            width: 900,
            height: 593,
            gsk_renderer: String::from("cairo"),
            icon_paths: Default::default(),
            icon_size: default_icon_size(),
            search_icon: false,
            use_base_css: true,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigBehavior {
    #[serde(default = "default_cache")]
    pub cache: PathBuf,
    #[serde(default = "default_true")]
    pub caching: bool,
    #[serde(default)]
    pub daemonize: bool,
    #[serde(default = "default_true")]
    pub animate: bool,
    pub field: Option<String>,
    pub launch_prefix: Option<String>,
}
impl Default for ConfigBehavior {
    fn default() -> Self {
        Self {
            cache: default_cache(),
            caching: false,
            daemonize: false,
            animate: true,
            field: None,
            launch_prefix: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
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
}
impl Default for ConfigFiles {
    fn default() -> Self {
        Self {
            config: default_config(),
            css: default_css(),
            fallback: default_fallback(),
            alias: default_alias(),
            ignore: default_ignore(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ConfigBinds {
    #[serde(default)]
    pub prev: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub modifier: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ConfigPipe {
    #[serde(default)]
    pub method: Option<String>,
}

pub fn read_file(file_path: &str) -> std::io::Result<String> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    Ok(content)
}

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn home_dir() -> Result<PathBuf, SherlockError> {
    env::var("HOME")
        .map_err(|e| SherlockError {
            error: SherlockErrorType::EnvVarNotFoundError(String::from("HOME")),
            traceback: e.to_string(),
        })
        .map(|s| PathBuf::from(s))
}
pub fn parse_priority(priority: f32, count: f32, decimals: i32) -> f32 {
    priority + 1.0 - count * 10f32.powi(-decimals)
}

pub fn expand_path(path: &Path, home: &Path) -> PathBuf {
    let mut components = path.components();
    if let Some(std::path::Component::Normal(first)) = components.next() {
        if first == "~" {
            return home.join(components.as_path());
        }
    }
    path.to_path_buf()
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
pub fn default_currency() -> String {
    String::from("eur")
}

pub fn default_cache() -> PathBuf {
    PathBuf::from("~/.cache/sherlock_desktop_cache.json")
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

pub fn default_true() -> bool {
    true
}
pub fn default_icon_size() -> i32 {
    22
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
