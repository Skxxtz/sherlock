use chrono::{DateTime, Local};
use std::path::Path;

use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};

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

    pub fn audit(&mut self) -> Result<Vec<Box<Path>>, SherlockError> {
        let current_audit_time = Local::now();

        // get entries
        let entries = std::fs::read_dir(&self.root_dir).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::DirReadError(self.root_dir.to_string_lossy().to_string()),
                e.to_string()
            )
        })?;

        // collect out-of-date entries
        let mut changed = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| sherlock_error!(SherlockErrorType::IO, e.to_string()))?;
            let metadata = entry
                .metadata()
                .map_err(|e| sherlock_error!(SherlockErrorType::IO, e.to_string()))?;

            if metadata.is_file() {
                if let Ok(modified_system_time) = metadata.modified() {
                    let modified_chrono: DateTime<Local> = modified_system_time.into();

                    if modified_chrono > self.latest_audit {
                        changed.push(entry.path().into_boxed_path())
                    }
                }
            }
        }

        self.latest_audit = current_audit_time;
        Ok(changed)
    }
}
