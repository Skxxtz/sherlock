use std::fs;

use crate::FLAGS;

use super::util::{SherlockConfig, SherlockError, SherlockErrorType};
use super::Loader;

impl Loader {
    pub fn load_config() -> Result<(SherlockConfig, Vec<SherlockError>), SherlockError> {
        let sherlock_flags = FLAGS.get().ok_or_else(|| SherlockError {
            error: SherlockErrorType::ConfigError(None),
            traceback: String::new(),
        })?;
        match fs::read_to_string(&sherlock_flags.config) {
            Ok(config_str) => {
                let mut config: SherlockConfig = match toml::de::from_str(&config_str) {
                    Ok(config) => config,
                    Err(e) => {
                        return Err(SherlockError {
                            error: SherlockErrorType::FileParseError(
                                sherlock_flags.config.to_string(),
                            ),
                            traceback: e.to_string(),
                        })
                    }
                };

                // Override from flags
                if sherlock_flags.caching {
                    config.behavior.caching = true;
                    config.behavior.cache = sherlock_flags.cache.clone();
                }
                if sherlock_flags.daemonize {
                    config.behavior.daemonize = true;
                }
                Ok((config, vec![]))
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    let mut non_breaking = vec![SherlockError {
                        error: SherlockErrorType::FileExistError(sherlock_flags.config.to_string()),
                        traceback: Default::default(),
                    }];

                    // Unpack non-breaking errors and default config
                    let (mut config, n) = SherlockConfig::default();
                    non_breaking.extend(n);

                    // Override from flags
                    if sherlock_flags.caching {
                        config.behavior.caching = true;
                        config.behavior.cache = sherlock_flags.cache.clone();
                    }
                    if sherlock_flags.daemonize {
                        config.behavior.daemonize = true;
                    }
                    Ok((config, non_breaking))
                }
                _ => Err(SherlockError {
                    error: SherlockErrorType::FileReadError(sherlock_flags.config.to_string()),
                    traceback: e.to_string(),
                })?,
            },
        }
    }
}
