use std::path::Path;
use std::{fs, env};

use super::Loader;
use super::util::{Config, SherlockError};


impl Loader {
    pub fn load_config() -> Result<(Config, Vec<SherlockError>), SherlockError> {
        let mut non_breaking: Vec<SherlockError> = Vec::new();
        let home_dir = env::var("HOME")
                .map_err(|e| SherlockError {
                    name:format!("Env Var not Found Error"),
                    message: format!("Failed to unpack home directory for user."),
                    traceback: e.to_string(),
                })?;
        let user_config_path = format!("{}/.config/sherlock/config.toml", home_dir);

        let user_config = if Path::new(&user_config_path).exists() {
            let config_str = fs::read_to_string(&user_config_path)
                .map_err(|e| SherlockError {
                    name:format!("File Read Error"),
                    message: format!("Failed to read the user configuration file: {}", user_config_path),
                    traceback: e.to_string(),
                })?;

            toml::de::from_str(&config_str)
                .map_err(|e| SherlockError {
                    name:format!("File Parse Error"),
                    message: format!("Failed to parse the user configuration from the file: {}", user_config_path),
                    traceback: e.to_string(),
                })?
        } else {
            // Unpack non-breaking errors and default config 
            let (config, n) = Config::default();
            non_breaking.extend(n);
            config
        };

        Ok((user_config, non_breaking))
    }
}

