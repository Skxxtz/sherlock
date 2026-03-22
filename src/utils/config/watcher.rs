use chrono::{DateTime, Local};
use std::path::Path;

use crate::{
    sherlock_error,
    utils::{
        config::ConfigGuard,
        errors::{SherlockError, SherlockErrorType},
    },
};

/// **Unfinished**
/// This struct aims at providing an audit function to check for config file changes and
/// application data changes. This should be run on every startup.
///
/// TODO:
/// Add functionality for .desktop files.
/// Add audit file that contains last audit time.
///
pub struct ConfigWatcher {
    latest_audit: DateTime<Local>,
    root_dir: Box<Path>,
}

impl ConfigWatcher {
    pub fn new(root_dir: Box<Path>) -> Self {
        Self {
            latest_audit: Local::now(),
            root_dir,
        }
    }

    pub fn audit(&mut self) -> Result<Vec<ConfigFileChange>, SherlockError> {
        let current_audit_time = Local::now();
        let since = self.latest_audit;

        // get entries
        let entries = std::fs::read_dir(&self.root_dir).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::DirReadError(self.root_dir.to_string_lossy().to_string()),
                e.to_string()
            )
        })?;

        let files = ConfigGuard::read()
            .map(|c| c.files.clone())
            .unwrap_or_default();

        // collect out-of-date entries
        let changed = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .map(|modified| {
                        let modified: DateTime<Local> = modified.into();
                        entry.path().is_file() && modified > since
                    })
                    .unwrap_or(false)
            })
            .map(|entry| {
                let path_buf = entry.path().to_path_buf();
                match path_buf {
                    _ if path_buf == files.config => ConfigFileChange::Config,
                    _ if path_buf == files.fallback => ConfigFileChange::Fallback,
                    _ if path_buf == files.alias => ConfigFileChange::Alias,
                    _ if path_buf == files.ignore => ConfigFileChange::Ignore,
                    _ if path_buf == files.actions => ConfigFileChange::Actions,
                    _ => ConfigFileChange::Other,
                }
            })
            .collect();

        self.latest_audit = current_audit_time;
        Ok(changed)
    }
}

pub enum ConfigFileChange {
    Fallback,
    Config,
    Alias,
    Ignore,
    Actions,
    Other,
}
