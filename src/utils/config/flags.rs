use std::path::PathBuf;

use crate::{
    sherlock_error,
    utils::{
        config::SherlockConfig,
        errors::{SherlockError, SherlockErrorType},
        files::{expand_path, home_dir},
        paths,
    },
};

#[derive(Clone, Debug, Default)]
pub struct SherlockFlags {
    pub config_dir: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub fallback: Option<PathBuf>,
    pub style: Option<PathBuf>,
    pub ignore: Option<PathBuf>,
    pub alias: Option<PathBuf>,
    pub display_raw: bool,
    pub center_raw: bool,
    pub cache: Option<PathBuf>,
    pub daemonize: bool,
    pub method: Option<String>,
    pub field: Option<String>,
    pub sub_menu: Option<String>,
    pub multi: bool,
    pub photo_mode: bool,
    pub input: Option<bool>,
}

impl SherlockFlags {
    pub fn to_config(&mut self) -> Result<(SherlockConfig, Vec<SherlockError>), SherlockError> {
        // Get location of config file
        let config_dir = self.config_dir.take().unwrap_or(paths::get_config_dir()?);
        let mut path = match &self.config {
            Some(path) => {
                let home = home_dir()?;
                expand_path(path, &home)
            }
            _ => config_dir.join("config.toml"),
        };

        // logic to either use json or toml
        let mut filetype: String = String::new();
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            match ext.as_ref() {
                "json" => {
                    if !path.exists() {
                        path.set_extension("toml");
                        filetype = "toml".to_string();
                    } else {
                        filetype = "json".to_string();
                    }
                }
                "toml" => {
                    if !path.exists() {
                        path.set_extension("json");
                        filetype = "json".to_string();
                    } else {
                        filetype = "toml".to_string();
                    }
                }
                _ => {}
            }
        } else {
            return Err(sherlock_error!(
                SherlockErrorType::FileParseError(path.clone()),
                format!(
                    "The file \"{}\" is not in a valid format.",
                    &path.to_string_lossy()
                )
            ));
        }

        match std::fs::read_to_string(&path) {
            Ok(config_str) => {
                let config_res: Result<SherlockConfig, SherlockError> = match filetype.as_str() {
                    "json" => {
                        let mut bytes = config_str.into_bytes();
                        simd_json::from_slice(&mut bytes).map_err(|e| {
                            sherlock_error!(
                                SherlockErrorType::FileParseError(path.clone()),
                                e.to_string()
                            )
                        })
                    }
                    "toml" => toml::de::from_str(&config_str).map_err(|e| {
                        sherlock_error!(
                            SherlockErrorType::FileParseError(path.clone()),
                            e.to_string()
                        )
                    }),
                    _ => {
                        return Err(sherlock_error!(
                            SherlockErrorType::FileParseError(path.clone()),
                            format!(
                                "The file \"{}\" is not in a valid format.",
                                &path.to_string_lossy()
                            )
                        ))
                    }
                };
                match config_res {
                    Ok(mut config) => {
                        config = SherlockConfig::apply_flags(self, config);
                        Ok((config, vec![]))
                    }
                    Err(e) => {
                        let mut config = SherlockConfig::default();

                        config = SherlockConfig::apply_flags(self, config);
                        Ok((config, vec![e]))
                    }
                }
            }
            Err(e) => {
                let mut config = SherlockConfig::default();
                config = SherlockConfig::apply_flags(self, config);
                let e = sherlock_error!(SherlockErrorType::FileReadError(path), e.to_string());
                Ok((config, vec![e]))
            }
        }
    }
}
