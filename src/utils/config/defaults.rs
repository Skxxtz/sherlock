use std::{path::PathBuf, process::Command};

use crate::{
    loader::application_loader::{get_applications_dir, get_desktop_files},
    sherlock_error,
    utils::{
        errors::{SherlockError, SherlockErrorType},
        files::read_lines,
        paths,
    },
};

pub struct ConstantDefaults {}
impl ConstantDefaults {
    pub fn terminal() -> String {
        Self::get_terminal().unwrap_or_default()
    }
    pub fn get_terminal() -> Result<String, SherlockError> {
        let mut terminal = None;

        //Check if $TERMAINAL is set
        if let Ok(term) = std::env::var("TERMINAL") {
            if Self::is_terminal_installed(&term) {
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
                if Self::is_terminal_installed(t) {
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
    pub fn browser() -> Result<String, SherlockError> {
        // Find default browser desktop file
        let output = Command::new("xdg-settings")
            .arg("get")
            .arg("default-web-browser")
            .output()
            .map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::EnvVarNotFoundError(String::from("default browser")),
                    e.to_string()
                )
            })?;

        let desktop_file: String = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            return Err(sherlock_error!(
                SherlockErrorType::EnvVarNotFoundError("default browser".to_string()),
                ""
            ));
        };
        let desktop_dirs = get_applications_dir();
        let desktop_files = get_desktop_files(desktop_dirs);
        let browser_file = desktop_files
            .iter()
            .find(|f| f.ends_with(&desktop_file))
            .ok_or_else(|| {
                sherlock_error!(
                    SherlockErrorType::EnvVarNotFoundError("default browser".to_string()),
                    ""
                )
            })?;
        // read default browser desktop file
        let browser = read_lines(browser_file)
            .map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileReadError(browser_file.clone()),
                    e.to_string()
                )
            })?
            .filter_map(Result::ok)
            .find(|line| line.starts_with("Exec="))
            .and_then(|line| line.strip_prefix("Exec=").map(|l| l.to_string()))
            .ok_or_else(|| {
                sherlock_error!(SherlockErrorType::FileParseError(browser_file.clone()), "")
            })?;
        Ok(browser)
    }
    pub fn teams() -> String {
        String::from("teams-for-linux --enable-features=UseOzonePlatform --ozone-platform=wayland --url {meeting_url}")
    }
    pub fn calendar_client() -> String {
        String::from("thunderbird")
    }
    pub fn lengths() -> String {
        String::from("meters")
    }
    pub fn weights() -> String {
        String::from("kg")
    }
    pub fn volumes() -> String {
        String::from("l")
    }
    pub fn temperatures() -> String {
        String::from("C")
    }
    pub fn currency() -> String {
        String::from("eur")
    }
}

pub struct BindDefaults {}
impl BindDefaults {
    pub fn modkey_ascii() -> Vec<String> {
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
    pub fn up() -> Option<String> {
        Some(String::from("control-k"))
    }
    pub fn down() -> Option<String> {
        Some(String::from("control-j"))
    }
    pub fn left() -> Option<String> {
        Some(String::from("control-ih"))
    }
    pub fn right() -> Option<String> {
        Some(String::from("control-l"))
    }
    pub fn context() -> Option<String> {
        Some(String::from("control-i"))
    }
    pub fn modifier() -> Option<String> {
        Some(String::from("control"))
    }
    pub fn exec_inplace() -> Option<String> {
        Some(String::from("control-return"))
    }
}

pub struct FileDefaults {}
impl FileDefaults {
    pub fn cache() -> PathBuf {
        paths::get_cache_dir()
            .unwrap()
            .join("desktop_files.bin")
    }
    pub fn config() -> PathBuf {
        paths::get_config_dir().unwrap().join("config.toml")
    }
    pub fn fallback() -> PathBuf {
        paths::get_config_dir().unwrap().join("fallback.json")
    }
    pub fn css() -> PathBuf {
        paths::get_config_dir().unwrap().join("main.css")
    }
    pub fn alias() -> PathBuf {
        paths::get_config_dir().unwrap().join("sherlock_alias.json")
    }
    pub fn ignore() -> PathBuf {
        paths::get_config_dir().unwrap().join("sherlockignore")
    }
    pub fn actions() -> PathBuf {
        paths::get_config_dir()
            .unwrap()
            .join("sherlock_actions.json")
    }
    pub fn icon_paths() -> Vec<PathBuf> {
        vec![paths::get_config_dir()
            .unwrap()
            .join("icons/")
            .to_path_buf()]
    }
}

pub struct OtherDefaults {}
impl OtherDefaults {
    pub fn bool_true() -> bool {
        true
    }
    pub fn one() -> f64 {
        1.0
    }
    pub fn five() -> u8 {
        5
    }
    pub fn backdrop_opacity() -> f64 {
        0.6
    }
    pub fn backdrop_edge() -> String {
        String::from("top")
    }
    pub fn icon_size() -> i32 {
        22
    }
    pub fn search_icon() -> String {
        String::from("system-search-symbolic")
    }
    pub fn search_icon_back() -> String {
        String::from("sherlock-back")
    }
}
