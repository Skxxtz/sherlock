use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

use crate::utils::config::defaults::FileDefaults;

mod config_impl;
mod defaults;
mod flags;
mod guard;
mod imp;

pub use defaults::{BindDefaults, ConstantDefaults, OtherDefaults};
pub use flags::SherlockFlags;
pub use guard::ConfigGuard;

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

    /// Configures caching feature
    #[serde(default)]
    pub caching: ConfigCaching,

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
    #[serde(default)]
    pub use_system_theme: bool,
    #[serde(default = "OtherDefaults::one")]
    pub opacity: f64,
    #[serde(default = "BindDefaults::modkey_ascii")]
    pub mod_key_ascii: Vec<String>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigBehavior {
    #[serde(default)]
    pub use_xdg_data_dir_icons: bool,
    #[serde(default = "OtherDefaults::bool_true")]
    pub animate: bool,
    #[serde(default)]
    pub global_prefix: Option<String>,
    #[serde(default)]
    pub global_flags: Option<String>,
    #[serde(default = "OtherDefaults::bool_true")]
    pub use_lr_nav: bool,
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
    #[serde(default)]
    pub daemonize: bool,
    #[serde(default)]
    pub field: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigCaching {
    #[serde(default = "OtherDefaults::bool_true")]
    pub enable: bool,
    #[serde(default = "FileDefaults::cache")]
    pub cache: PathBuf,
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigSourceFiles {
    pub source: Vec<ConfigSource>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigSource {
    pub file: PathBuf,
}
