use std::sync::Arc;

use gpui::{AsyncApp, Entity};

use super::{SherlockConfig, watcher::ConfigFileChange};
use crate::{
    CONFIG, launcher::children::RenderableChild, loader::Loader, ui::launcher::LauncherMode,
    utils::errors::SherlockError,
};

pub async fn reload(
    cx: &AsyncApp,
    data: &Entity<Arc<Vec<RenderableChild>>>,
    initial_errors: &mut Vec<SherlockError>,
    initial_warnings: &mut Vec<SherlockError>,
    changes: Vec<ConfigFileChange>,
) -> Option<Arc<[LauncherMode]>> {
    let needs = ReloadNeeds::from_changes(&changes);
    let mut warnings: Vec<SherlockError> = Vec::new();
    let mut errors: Vec<SherlockError> = Vec::new();

    if needs.config {
        let mut flags = Loader::load_flags().ok()?;
        let config = flags.to_config().map_or_else(
            |e| {
                errors.push(e);
                SherlockConfig::apply_flags(&mut flags, SherlockConfig::default())
            },
            |(cfg, non_crit)| {
                warnings.extend(non_crit);
                cfg
            },
        );
        // Update global config
        if let Ok(mut guard) = CONFIG.get()?.write() {
            *guard = config;
        }
    }

    // Reload launchers
    let modes = if needs.launchers {
        let result = match cx
            .update(|cx| Loader::load_launchers(cx, data.clone()))
            .ok()?
        {
            Ok(result) => result,
            Err(e) => {
                errors.push(e);
                return None;
            }
        };
        warnings.extend(result.warnings);
        Some(result.modes)
    } else {
        None // caller keeps existing modes
    };

    *initial_errors = errors;
    *initial_warnings = warnings;
    modes
}

struct ReloadNeeds {
    config: bool,
    launchers: bool,
}

impl ReloadNeeds {
    fn from_changes(changes: &[ConfigFileChange]) -> Self {
        let mut needs = Self {
            config: false,
            launchers: false,
        };
        for change in changes {
            match change {
                ConfigFileChange::Config => needs.config = true,
                ConfigFileChange::Fallback
                | ConfigFileChange::Alias
                | ConfigFileChange::Actions
                | ConfigFileChange::Ignore => needs.launchers = true,
                ConfigFileChange::Other => {}
            }
        }
        needs
    }
}
