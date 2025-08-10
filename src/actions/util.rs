use std::fs;

use cli_clipboard::{ClipboardContext, ClipboardProvider};

use crate::sherlock_error;
use crate::utils::config::ConfigGuard;
use crate::utils::{
    errors::{SherlockError, SherlockErrorType},
    paths,
};

pub fn copy_to_clipboard(string: &str) -> Result<(), SherlockError> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| sherlock_error!(SherlockErrorType::ClipboardError, e.to_string()))?;

    let _ = ctx.set_contents(string.to_string());
    Ok(())
}
//TODO: takes 2.9ms/1.6ms - how to improve
#[sherlock_macro::timing(level = "launchers")]
pub fn read_from_clipboard() -> Result<String, SherlockError> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| sherlock_error!(SherlockErrorType::ClipboardError, e.to_string()))?;
    Ok(ctx.get_contents().unwrap_or_default().trim().to_string())
}

pub fn clear_cached_files() -> Result<(), SherlockError> {
    let cache_dir = paths::get_cache_dir()?;

    let config = ConfigGuard::read()?;
    // Clear sherlocks cache
    fs::remove_dir_all(&cache_dir).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::DirRemoveError(cache_dir.to_string_lossy().to_string()),
            e.to_string()
        )
    })?;

    // Clear app cache
    fs::remove_file(&config.caching.cache).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::FileRemoveError(config.caching.cache.clone()),
            e.to_string()
        )
    })?;

    Ok(())
}

pub fn reset_app_counter() -> Result<(), SherlockError> {
    let data_dir = paths::get_data_dir()?;
    let counts_path = data_dir.join("counts.json");
    fs::remove_file(&counts_path).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::FileRemoveError(counts_path),
            e.to_string()
        )
    })
}
