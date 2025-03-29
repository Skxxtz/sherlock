use serde::{Deserialize, Serialize};
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
    pub priority: u32,

    #[serde(default)]
    pub r#async: bool,
    #[serde(default)]
    pub home: bool,
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
}

#[derive(Clone, Debug, Default)]
pub struct SherlockFlags {
    pub config: String,
    pub fallback: String,
    pub style: String,
    pub ignore: String,
    pub alias: String,
    pub display_raw: bool,
    pub center_raw: bool,
    pub caching: bool,
    pub cache: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SherlockAlias {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub keywords: Option<String>,
}
#[derive(Debug, Clone)]
pub enum SherlockErrorType {
    EnvVarNotFoundError(String),
    FileExistError(String),
    FileReadError(String),
    FileParseError(String),
    DirReadError(String),
    ResourceParseError,
    ResourceLookupError(String),
    DisplayError,
    ConfigError(Option<String>),
    RegexError(String),
    CommandExecutionError(String),
    ClipboardError,
}

impl SherlockErrorType {
    pub fn get_message(&self) -> (String, String) {
        match self {
            SherlockErrorType::EnvVarNotFoundError(var) => (
                "EnvVarNotFoundError".to_string(),
                format!("Failed to unpack environment variable \"{}\"", var),
            ),
            SherlockErrorType::FileExistError(file) => (
                "FileExistError".to_string(),
                format!("File \"{}\" does not exist", file),
            ),
            SherlockErrorType::FileReadError(file) => (
                "FileReadError".to_string(),
                format!("Failed to read file \"{}\"", file),
            ),
            SherlockErrorType::FileParseError(file) => (
                "FileParseError".to_string(),
                format!("Failed to parse file \"{}\"", file),
            ),
            SherlockErrorType::DirReadError(file) => (
                "DirReadError".to_string(),
                format!("Failed to read/access dir \"{}\"", file),
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
    pub debug: ConfigDebug,
    #[serde(default)]
    pub appearance: ConfigAppearance,
    #[serde(default)]
    pub behavior: ConfigBehavior,
}
impl SherlockConfig {
    pub fn default() -> (Self, Vec<SherlockError>) {
        let mut non_breaking: Vec<SherlockError> = Vec::new();
        (
            SherlockConfig {
                default_apps: ConfigDefaultApps {
                    calendar_client: default_calendar_client(),
                    teams: default_teams(),
                    terminal: get_terminal()
                        .map_err(|e| non_breaking.push(e))
                        .unwrap_or_default(),
                },
                debug: ConfigDebug {
                    try_suppress_errors: false,
                    try_suppress_warnings: false,
                },
                appearance: ConfigAppearance {
                    width: 900,
                    height: 593,
                    gsk_renderer: "cairo".to_string(),
                    recolor_icons: false,
                    icon_paths: Default::default(),
                    icon_size: default_icon_size(),
                },
                behavior: ConfigBehavior {
                    cache: String::from("~/.cache/sherlock_desktop_cache.json"),
                    caching: false,
                    daemonize: false,
                    animate: true,
                },
            },
            non_breaking,
        )
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

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ConfigBehavior {
    #[serde(default = "default_cache")]
    pub cache: String,
    #[serde(default)]
    pub caching: bool,
    #[serde(default)]
    pub daemonize: bool,
    #[serde(default = "default_true")]
    pub animate: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ConfigDebug {
    #[serde(default)]
    pub try_suppress_errors: bool,
    #[serde(default)]
    pub try_suppress_warnings: bool,
}
#[derive(Deserialize, Debug, Clone, Default)]
pub struct ConfigAppearance {
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub gsk_renderer: String,
    #[serde(default)]
    pub recolor_icons: bool,
    #[serde(default)]
    pub icon_paths: Vec<String>,
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
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

pub fn default_terminal() -> String {
    get_terminal().unwrap_or_default()
}
pub fn default_teams() -> String {
    String::from("teams-for-linux --enable-features=UseOzonePlatform --ozone-platform=wayland --url {meeting_url}")
}
pub fn default_calendar_client() -> String {
    String::from("thunderbird")
}
pub fn default_cache() -> String {
    match env::var("HOME") {
        Ok(dir) => format!("{}/.cache/sherlock_desktop_cache.json", dir),
        Err(_) => String::from("~/cache/sherlock_desktop_cache.json"),
    }
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
