use std::{env, fs, path::PathBuf, process::Command}; // Added env and PathBuf for XDG paths

use cli_clipboard::{ClipboardContext, ClipboardProvider};

use crate::{
    loader::application_loader::{get_applications_dir, get_desktop_files},
    utils::{
        errors::{SherlockError, SherlockErrorType},
        files::{home_dir, read_lines},
    },
};
use crate::{sherlock_error, CONFIG};

fn get_sherlock_cache_dir() -> Result<PathBuf, SherlockError> {
    let cache_home = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            // Fallback to ~/.cache if XDG_CACHE_HOME is not set
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"))
        })
        .ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::EnvVarNotFoundError("HOME or XDG_CACHE_HOME".to_string()),
                "Neither HOME nor XDG_CACHE_HOME environment variable found".to_string()
            )
        })?;
    Ok(cache_home.join("sherlock"))
}

pub fn copy_to_clipboard(string: &str) -> Result<(), SherlockError> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| sherlock_error!(SherlockErrorType::ClipboardError, e.to_string()))?;

    let _ = ctx.set_contents(string.to_string());
    Ok(())
}

pub fn read_from_clipboard() -> Result<String, SherlockError> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| sherlock_error!(SherlockErrorType::ClipboardError, e.to_string()))?;
    Ok(ctx.get_contents().unwrap_or_default().trim().to_string())
}

pub fn clear_cached_files() -> Result<(), SherlockError> {
    let config = CONFIG
        .get()
        .ok_or_else(|| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;

    let sherlock_cache_dir = get_sherlock_cache_dir()?;

    fs::remove_dir_all(&sherlock_cache_dir).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::DirRemoveError(sherlock_cache_dir.display().to_string()),
            e.to_string()
        )
    })?;


    Ok(())
}

pub fn reset_app_counter() -> Result<(), SherlockError> {
    let sherlock_cache_dir = get_sherlock_cache_dir()?;
    let counts_file_path = sherlock_cache_dir.join("counts.json");

    fs::remove_file(&counts_file_path).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::FileRemoveError(counts_file_path.clone()),
            e.to_string()
        )
    })
}

pub fn parse_default_browser() -> Result<String, SherlockError> {
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
