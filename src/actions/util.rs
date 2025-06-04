use std::{fs, process::Command};

use cli_clipboard::{ClipboardContext, ClipboardProvider};

use crate::{
    loader::application_loader::{get_applications_dir, get_desktop_files},
    utils::{
        errors::{SherlockError, SherlockErrorType},
        files::{home_dir, read_lines},
    },
};
use crate::{sherlock_error, CONFIG};

pub fn copy_to_clipboard(string: &str) -> Result<(), SherlockError> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| sherlock_error!(SherlockErrorType::ClipboardError, e.to_string()))?;

    let _ = ctx.set_contents(string.to_string());
    Ok(())
}
//TODO: takes 2.9ms/1.6ms - how to improve
pub fn read_from_clipboard() -> Result<String, SherlockError> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| sherlock_error!(SherlockErrorType::ClipboardError, e.to_string()))?;
    Ok(ctx.get_contents().unwrap_or_default().trim().to_string())
}

pub fn clear_cached_files() -> Result<(), SherlockError> {
    let config = CONFIG
        .get()
        .ok_or_else(|| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;
    let home = home_dir()?;
    // Clear sherlocks cache
    fs::remove_dir_all(home.join(".cache/sherlock")).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::DirRemoveError(String::from("~/.cache/sherlock")),
            e.to_string()
        )
    })?;

    // Clear app cache
    fs::remove_file(&config.behavior.cache).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::FileRemoveError(config.behavior.cache.clone()),
            e.to_string()
        )
    })?;

    Ok(())
}

pub fn reset_app_counter() -> Result<(), SherlockError> {
    let home = home_dir()?;
    fs::remove_file(home.join(".cache/sherlock/counts.json")).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::FileRemoveError(home.join(".sherlock/counts.json")),
            e.to_string()
        )
    })
}
pub fn parse_default_browser() -> Result<String, SherlockError> {
    // Find default browser desktop file
    let output = Command::new("xdg-settings")
        .arg("get")
        .arg("default-web-browser")
        .output()
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::EnvVarNotFoundError(String::from("default browser")),
                e.to_string()
            )
        })?;

    let desktop_file: String = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        return Err(sherlock_error!(
            SherlockErrorType::EnvVarNotFoundError("default browser".to_string()),
            ""
        ));
    };
    let desktop_dirs = get_applications_dir();
    let desktop_files = get_desktop_files(desktop_dirs);
    let browser_file = desktop_files
        .iter()
        .find(|f| f.ends_with(&desktop_file))
        .ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::EnvVarNotFoundError("default browser".to_string()),
                ""
            )
        })?;
    // read default browser desktop file
    let browser = read_lines(browser_file)
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileReadError(browser_file.clone()),
                e.to_string()
            )
        })?
        .filter_map(Result::ok)
        .find(|line| line.starts_with("Exec="))
        .and_then(|line| line.strip_prefix("Exec=").map(|l| l.to_string()))
        .ok_or_else(|| {
            sherlock_error!(SherlockErrorType::FileParseError(browser_file.clone()), "")
        })?;
    Ok(browser)
}
