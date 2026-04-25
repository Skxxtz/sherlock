mod fallback_migration;

use std::fs;
use std::path::Path;

use crate::sherlock_msg;
use crate::utils::config::transformer::fallback_migration::LegacyRawLauncher;
use crate::utils::errors::SherlockMessage;
use crate::utils::errors::types::{FileAction, SherlockErrorType};

#[allow(dead_code)]
pub fn migrate_file<P: AsRef<Path>>(path: P) -> Result<(), SherlockMessage> {
    let path_ref = path.as_ref();
    let content = fs::read_to_string(path_ref).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::FileError(FileAction::Read, path_ref.to_path_buf()),
            e
        )
    })?;

    let legacy_configs: Vec<LegacyRawLauncher> = serde_json::from_str(&content).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DeserializationError,
            format!("File is neither modern nor legacy format: {e}")
        )
    })?;

    let mut upgraded_launchers = Vec::new();
    let mut all_logs = Vec::new();

    for legacy in legacy_configs {
        let result = legacy.migrate();
        upgraded_launchers.push(result.launcher);
        all_logs.extend(result.logs);
    }

    // 4. Print migration audit trail
    if !all_logs.is_empty() {
        println!("--- Migration Logs for {} ---", path_ref.display());
        for log in all_logs {
            println!("  • {}", log);
        }
    }

    // 5. Save the upgraded version back to the file
    let new_json = serde_json::to_string_pretty(&upgraded_launchers)
        .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::SerializationError, e))?;
    fs::write(path_ref, new_json).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::FileError(FileAction::Write, path_ref.to_path_buf()),
            e
        )
    })?;

    println!(
        "[{}] Successfully migrated to new format.",
        path_ref.display()
    );

    Ok(())
}
