use std::sync::Arc;

use gpui::{AsyncApp, Entity};

use super::{SherlockConfig, watcher::ConfigFileChange};
use crate::{
    CONFIG, launcher::children::RenderableChild, loader::Loader, ui::launcher::LauncherMode,
    utils::errors::SherlockMessage,
};

pub async fn reload(
    cx: &AsyncApp,
    data: &Entity<Arc<Vec<RenderableChild>>>,
    initial_messages: &mut Vec<SherlockMessage>,
    changes: Vec<ConfigFileChange>,
) -> Option<Arc<[LauncherMode]>> {
    let needs = ReloadNeeds::from_changes(&changes);
    let mut messages: Vec<SherlockMessage> = Vec::new();

    if needs.config {
        let mut flags = Loader::load_flags().ok()?;
        let config = match flags.to_config() {
            Err(e) => {
                messages.push(e);
                SherlockConfig::apply_flags(&mut flags, SherlockConfig::default())
            }
            Ok((cfg, msgs)) => {
                messages.extend(msgs);
                cfg
            }
        };
        // Update global config
        if let Ok(mut guard) = CONFIG.get()?.write() {
            *guard = config;
        }
    }

    // Reload launchers
    let modes = if needs.launchers {
        let result = match cx.update(|cx| Loader::load_launchers(cx, data.clone())) {
            Ok(result) => result,
            Err(e) => {
                messages.push(e);
                return None;
            }
        };
        messages.extend(result.messages);
        Some(result.modes)
    } else {
        None // caller keeps existing modes
    };

    *initial_messages = messages;
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
