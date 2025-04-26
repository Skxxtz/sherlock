use std::fs;

use cli_clipboard::{ClipboardContext, ClipboardProvider};

use crate::utils::{
    errors::{SherlockError, SherlockErrorType},
    files::home_dir,
};
use crate::CONFIG;

pub fn copy_to_clipboard(string: &str) -> Result<(), SherlockError> {
    let mut ctx = ClipboardContext::new().map_err(|e| SherlockError {
        error: SherlockErrorType::ClipboardError,
        traceback: e.to_string(),
    })?;

    let _ = ctx.set_contents(string.to_string());
    Ok(())
}
pub fn read_from_clipboard() -> Result<String, SherlockError> {
    let mut ctx = ClipboardContext::new().map_err(|e| SherlockError {
        error: SherlockErrorType::ClipboardError,
        traceback: e.to_string(),
    })?;
    Ok(ctx.get_contents().unwrap_or_default().trim().to_string())
}

pub fn clear_cached_files() -> Result<(), SherlockError> {
    let config = CONFIG.get().ok_or_else(|| SherlockError {
        error: SherlockErrorType::ConfigError(None),
        traceback: String::from("Location: src/actions/util.rs"),
    })?;
    let home = home_dir()?;
    // Clear sherlocks cache
    fs::remove_dir_all(home.join(".cache/sherlock")).map_err(|e| SherlockError {
        error: SherlockErrorType::DirRemoveError(String::from("~/.cache/sherlock")),
        traceback: e.to_string(),
    })?;

    // Clear app cache
    fs::remove_file(&config.behavior.cache).map_err(|e| SherlockError {
        error: SherlockErrorType::FileRemoveError(config.behavior.cache.clone()),
        traceback: e.to_string(),
    })?;

    Ok(())
}

pub fn reset_app_counter() -> Result<(), SherlockError> {
    let home = home_dir()?;
    fs::remove_file(home.join(".sherlock/counts.json")).map_err(|e| SherlockError {
        error: SherlockErrorType::FileRemoveError(home.join(".sherlock/counts.json")),
        traceback: e.to_string(),
    })
}
