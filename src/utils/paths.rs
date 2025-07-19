use crate::utils::files;
use std::{fs, path::PathBuf};

fn get_xdg_dirs() -> xdg::BaseDirectories {
    xdg::BaseDirectories::with_prefix("sherlock")
}

fn legacy_path() -> Result<PathBuf, crate::utils::errors::SherlockError> {
    let home_dir = files::home_dir()?;
    Ok(home_dir.join(".sherlock"))
}

/// Returns the configuration directory.
///
/// It first checks for the legacy `~/.sherlock` directory. If it exists, it returns that path.
/// Otherwise, it returns the XDG standard configuration path, `$XDG_CONFIG_HOME/sherlock`.
/// If the directory does not exist, it will be created.
pub fn get_config_dir() -> Result<PathBuf, crate::utils::errors::SherlockError> {
    let legacy_path = legacy_path()?;
    if legacy_path.exists() {
        return Ok(legacy_path);
    }
    let xdg_dirs = get_xdg_dirs();
    let dir = xdg_dirs.get_config_home().ok_or_else(|| {
        crate::sherlock_error!(
            crate::utils::errors::SherlockErrorType::DirReadError(
                "Could not find config directory".to_string()
            ),
            ""
        )
    })?;
    fs::create_dir_all(&dir).map_err(|_| {
        crate::sherlock_error!(
            crate::utils::errors::SherlockErrorType::DirCreateError(
                "Could not create config directory".to_string()
            ),
            ""
        )
    })?;
    Ok(dir)
}

/// Returns the data directory.
///
/// It first checks for the legacy `~/.sherlock` directory. If it exists, it returns that path.
/// Otherwise, it returns the XDG standard data path, `$XDG_DATA_HOME/sherlock`.
/// If the directory does not exist, it will be created.
pub fn get_data_dir() -> Result<PathBuf, crate::utils::errors::SherlockError> {
    let legacy_path = legacy_path()?;
    if legacy_path.exists() {
        return Ok(legacy_path);
    }
    let xdg_dirs = get_xdg_dirs();
    let dir = xdg_dirs.get_data_home().ok_or_else(|| {
        crate::sherlock_error!(
            crate::utils::errors::SherlockErrorType::DirReadError(
                "Could not find data directory".to_string()
            ),
            ""
        )
    })?;
    fs::create_dir_all(&dir).map_err(|_| {
        crate::sherlock_error!(
            crate::utils::errors::SherlockErrorType::DirCreateError(
                "Could not create data directory".to_string()
            ),
            ""
        )
    })?;
    Ok(dir)
}

/// Returns the cache directory.
///
/// This function returns the XDG standard cache path, `$XDG_CACHE_HOME/sherlock`.
/// If the directory does not exist, it will be created.
pub fn get_cache_dir() -> Result<PathBuf, crate::utils::errors::SherlockError> {
    let xdg_dirs = get_xdg_dirs();
    let dir = xdg_dirs.get_cache_home().ok_or_else(|| {
        crate::sherlock_error!(
            crate::utils::errors::SherlockErrorType::DirReadError(
                "Could not find cache directory".to_string()
            ),
            ""
        )
    })?;
    fs::create_dir_all(&dir).map_err(|_| {
        crate::sherlock_error!(
            crate::utils::errors::SherlockErrorType::DirCreateError(
                "Could not create cache directory".to_string()
            ),
            ""
        )
    })?;
    Ok(dir)
}
