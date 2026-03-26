use crate::{
    sherlock_msg,
    utils::{
        config::{ConfigSourceFiles, SherlockConfig},
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        files::{expand_path, home_dir},
        paths,
    },
};
use std::{fs::read_to_string, path::PathBuf};

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
    pub placeholder: Option<String>,
}

impl SherlockFlags {
    pub fn to_config(&mut self) -> Result<(SherlockConfig, Vec<SherlockMessage>), SherlockMessage> {
        // Get location of config file
        let config_dir = self.config_dir.take().unwrap_or(paths::get_config_dir()?);
        let home = home_dir()?;
        let mut path = match &self.config {
            Some(path) => expand_path(path, &home),
            _ => config_dir.join("config.toml"),
        };

        // logic to either use json or toml
        let filetype = if let Some(ext) = path.extension() {
            let mut ext_str = ext.to_string_lossy().to_string();
            if !path.exists() {
                match ext_str.as_str() {
                    "json" => {
                        path.set_extension("toml");
                        ext_str = "toml".into();
                    }
                    "toml" => {
                        path.set_extension("json");
                        ext_str = "json".into();
                    }
                    _ => {
                        return Err(sherlock_msg!(
                            Warning,
                            SherlockErrorType::FileError(FileAction::Parse, path.clone()),
                            format!("unsupported format: '{}'", ext_str)
                        ));
                    }
                }
            }
            ext_str
        } else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Parse, path.clone()),
                "file has no extension and cannot be parsed"
            ));
        };

        match std::fs::read_to_string(&path) {
            Ok(mut config_str) => {
                let config_res: Result<SherlockConfig, SherlockMessage> = match filetype.as_str() {
                    "json" => {
                        let mut bytes = config_str.into_bytes();
                        simd_json::from_slice(&mut bytes).map_err(|e| {
                            sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e)
                        })
                    }
                    "toml" => {
                        // Setup to parse nested configs
                        if let Ok(sources) = toml::de::from_str::<ConfigSourceFiles>(&config_str) {
                            if !sources.source.is_empty() {
                                sources
                                    .source
                                    .into_iter()
                                    .map(|s| {
                                        if s.file.starts_with("~/") {
                                            expand_path(s.file, &home)
                                        } else {
                                            s.file
                                        }
                                    })
                                    .filter(|f| f.is_file())
                                    .filter_map(|f| read_to_string(&f).ok())
                                    .for_each(|content| {
                                        config_str.push('\n');
                                        config_str.push_str(&content);
                                    });
                            }
                        }
                        toml::de::from_str(&config_str).map_err(|e| {
                            sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e)
                        })
                    }
                    _ => {
                        let ext = path
                            .extension()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_else(|| "none".into());

                        return Err(sherlock_msg!(
                            Warning,
                            SherlockErrorType::FileError(FileAction::Parse, path.clone()),
                            format!("the file extension '{}' is not a supported format", ext)
                        ));
                    }
                };
                match config_res {
                    Ok(mut config) => {
                        config = SherlockConfig::apply_flags(self, config);
                        config.initialized = true;
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
                let e = sherlock_msg!(
                    Warning,
                    SherlockErrorType::FileError(FileAction::Read, path),
                    e
                );
                Ok((config, vec![e]))
            }
        }
    }
}
