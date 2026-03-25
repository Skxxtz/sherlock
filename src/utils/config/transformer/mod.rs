mod fallback_migration;

use std::fs;
use std::path::Path;

use crate::utils::config::transformer::fallback_migration::LegacyRawLauncher;

pub fn migrate_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let path_ref = path.as_ref();
    let content = fs::read_to_string(path_ref)?;

    let legacy_configs: Vec<LegacyRawLauncher> = serde_json::from_str(&content)
        .map_err(|e| format!("File is neither modern nor legacy format: {}", e))?;

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
    let new_json = serde_json::to_string_pretty(&upgraded_launchers)?;
    fs::write(path_ref, new_json)?;

    println!(
        "[{}] Successfully migrated to new format.",
        path_ref.display()
    );

    Ok(())
}
