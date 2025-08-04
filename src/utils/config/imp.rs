use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::utils::{
    config::{
        defaults::{BindDefaults, ConstantDefaults, FileDefaults, OtherDefaults},
        ConfigAppearance, ConfigBackdrop, ConfigBehavior, ConfigBinds, ConfigCaching, ConfigDebug,
        ConfigDefaultApps, ConfigExpand, ConfigFiles, ConfigUnits, SearchBarIcon, StatusBar,
    },
    files::home_dir,
};

impl Default for ConfigDefaultApps {
    fn default() -> Self {
        Self {
            teams: ConstantDefaults::teams(),
            calendar_client: ConstantDefaults::calendar_client(),
            terminal: ConstantDefaults::get_terminal().unwrap_or_default(), // Should never get to this...
            browser: ConstantDefaults::browser().ok(),
            mpris: None,
        }
    }
}

impl Default for ConfigUnits {
    fn default() -> Self {
        Self {
            lengths: ConstantDefaults::lengths(),
            weights: ConstantDefaults::weights(),
            volumes: ConstantDefaults::volumes(),
            temperatures: ConstantDefaults::temperatures(),
            currency: ConstantDefaults::currency(),
        }
    }
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

impl Default for ConfigAppearance {
    fn default() -> Self {
        Self {
            width: 900,
            height: 593, // 617 with, 593 without notification bar
            gsk_renderer: String::from("cairo"),
            icon_paths: FileDefaults::icon_paths(),
            icon_size: OtherDefaults::icon_size(),
            use_base_css: true,
            opacity: 1.0,
            mod_key_ascii: BindDefaults::modkey_ascii(),
        }
    }
}

impl Default for ConfigBehavior {
    fn default() -> Self {
        Self {
            use_xdg_data_dir_icons: false,
            animate: true,
            global_prefix: None,
            global_flags: None,
            lr_arrow_nav: false,
        }
    }
}

impl Default for ConfigFiles {
    fn default() -> Self {
        Self {
            config: FileDefaults::config(),
            css: FileDefaults::css(),
            fallback: FileDefaults::fallback(),
            alias: FileDefaults::alias(),
            ignore: FileDefaults::ignore(),
            actions: FileDefaults::actions(),
        }
    }
}

impl Default for ConfigBinds {
    fn default() -> Self {
        Self {
            up: BindDefaults::up(),
            down: BindDefaults::down(),
            left: BindDefaults::left(),
            right: BindDefaults::right(),
            context: BindDefaults::context(),
            modifier: BindDefaults::modifier(),
            exec_inplace: BindDefaults::exec_inplace(),
        }
    }
}

impl Default for ConfigCaching {
    fn default() -> Self {
        Self {
            enable: true,
            cache: FileDefaults::cache(),
        }
    }
}

impl Default for ConfigExpand {
    fn default() -> Self {
        Self {
            enable: false,
            edge: OtherDefaults::backdrop_edge(),
            margin: 0,
        }
    }
}

impl Default for ConfigBackdrop {
    fn default() -> Self {
        Self {
            enable: false,
            opacity: OtherDefaults::backdrop_opacity(),
            edge: OtherDefaults::backdrop_edge(),
        }
    }
}

impl Default for SearchBarIcon {
    fn default() -> Self {
        Self {
            enable: true,
            icon: OtherDefaults::search_icon(),
            icon_back: OtherDefaults::search_icon_back(),
            size: OtherDefaults::icon_size(),
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self { enable: true }
    }
}

// With Root Implementations
pub trait WithRoot {
    fn with_root(root: &PathBuf) -> Self;
}
impl WithRoot for ConfigAppearance {
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

impl WithRoot for ConfigFiles {
    fn with_root(root: &PathBuf) -> Self {
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
