use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    sherlock_error,
    utils::{
        config::SherlockConfig,
        errors::{SherlockError, SherlockErrorType},
    },
    CONFIG,
};

pub struct ConfigGuard;
impl<'g> ConfigGuard {
    fn get_config() -> Result<&'g RwLock<SherlockConfig>, SherlockError> {
        CONFIG.get().ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Config not initialized".to_string()
            )
        })
    }

    fn get_read() -> Result<RwLockReadGuard<'g, SherlockConfig>, SherlockError> {
        Self::get_config()?.read().map_err(|_| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Failed to acquire write lock on config".to_string()
            )
        })
    }

    fn _get_write() -> Result<RwLockWriteGuard<'g, SherlockConfig>, SherlockError> {
        Self::get_config()?.write().map_err(|_| {
            sherlock_error!(
                SherlockErrorType::ConfigError(None),
                "Failed to acquire write lock on config".to_string()
            )
        })
    }

    pub fn read() -> Result<RwLockReadGuard<'g, SherlockConfig>, SherlockError> {
        Self::get_read()
    }

    pub fn write_key<F>(key_fn: F) -> Result<(), SherlockError>
    where
        F: FnOnce(&mut SherlockConfig),
    {
        let mut config = Self::_get_write()?;
        key_fn(&mut config);
        Ok(())
    }
}
