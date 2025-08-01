use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    loader::Loader,
    sherlock_error,
    utils::{
        config::{imp::WithRoot, ConfigAppearance, ConfigFiles, SherlockConfig, SherlockFlags},
        errors::{SherlockError, SherlockErrorType},
        files::{expand_path, home_dir},
    },
};

impl SherlockConfig {
    /// # Arguments
    /// loc: PathBuf
    /// Pathbuf should be a directory **not** a file
    pub fn to_file(loc: PathBuf) -> Result<(), SherlockError> {
        // create config location
        let home = home_dir()?;
        let path = expand_path(&loc, &home);

        fn ensure_dir(path: &Path, label: &str) {
            match std::fs::create_dir(path) {
                Ok(_) => println!("✓ Created '{}' directory", label),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    println!("↷ Skipping '{}' — directory already exists.", label)
                }
                Err(e) => eprintln!("✗ Failed to create '{}' directory: {}", label, e),
            }
        }
        fn created_message(name: &str) {
            println!("✓ Created '{}'", name);
        }
        fn skipped_message(name: &str) {
            println!("↷ Skipping '{}' since file exists already.", name);
        }
        fn error_message(name: &str, reason: SherlockError) {
            eprintln!(
                "✗ Failed to create '{}'. Reason: {}",
                name,
                reason.error.get_message().0
            );
        }
        let write_file = |name: &str, content: &str| {
            let alias_path = path.join(name);
            if !alias_path.exists() {
                if let Err(error) = fs::write(&alias_path, content).map_err(|e| {
                    sherlock_error!(SherlockErrorType::FileWriteError(alias_path), e.to_string())
                }) {
                    error_message(name, error);
                } else {
                    created_message(name);
                }
            } else {
                skipped_message(name);
            }
        };

        // build default config
        let config = SherlockConfig::with_root(&loc);
        let toml_str = toml::to_string(&config).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileWriteError(path.clone()),
                e.to_string()
            )
        })?;

        // mkdir -p
        fs::create_dir_all(&path).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::DirCreateError(format!("{:?}", path)),
                e.to_string()
            )
        })?;
        // create subdirs
        ensure_dir(&path.join("icons/"), "icons");
        ensure_dir(&path.join("scripts/"), "scripts");
        ensure_dir(&path.join("themes/"), "themes");

        // Write basic config files
        write_file("config.toml", &toml_str);
        write_file("sherlockignore", "");
        write_file("sherlock_actions.json", "[]");
        write_file("sherlock_alias.json", "{}");
        write_file("main.css", "");

        // write fallback.json file
        let fallback_path = path.join("fallback.json");
        if !fallback_path.exists() {
            // load resources
            Loader::load_resources()?;
            let data = gio::resources_lookup_data(
                "/dev/skxxtz/sherlock/fallback.json",
                gio::ResourceLookupFlags::NONE,
            )
            .map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::ResourceLookupError("fallback.json".to_string()),
                    e.to_string()
                )
            })?;

            let json_str = std::str::from_utf8(&data).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileParseError(PathBuf::from("fallback.json")),
                    e.to_string()
                )
            })?;
            if let Err(error) = fs::write(&fallback_path, json_str).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileWriteError(fallback_path),
                    e.to_string()
                )
            }) {
                error_message("fallback.json", error);
            } else {
                created_message("fallback.json");
            };
        } else {
            skipped_message("fallback.json");
        }

        if let Some(loc) = loc.to_str() {
            if loc != "~/.config/sherlock/" {
                let loc = loc.trim_end_matches("/");
                println!("\nUse \x1b[32msherlock --config {}/config.toml\x1b[0m to run sherlock with the custom configuration.", loc);
            }
        }

        std::process::exit(0);
    }
    pub fn apply_flags(
        sherlock_flags: &mut SherlockFlags,
        mut config: SherlockConfig,
    ) -> SherlockConfig {
        // Make paths that contain the ~ dir use the correct path
        let home = match home_dir() {
            Ok(h) => h,
            Err(_) => return config,
        };

        // Override config files from flags
        config.files.config = expand_path(
            &sherlock_flags
                .config
                .as_deref()
                .unwrap_or(&config.files.config),
            &home,
        );
        config.files.fallback = expand_path(
            &sherlock_flags
                .fallback
                .as_deref()
                .unwrap_or(&config.files.fallback),
            &home,
        );
        config.files.css = expand_path(
            &sherlock_flags.style.as_deref().unwrap_or(&config.files.css),
            &home,
        );
        config.files.alias = expand_path(
            &sherlock_flags
                .alias
                .as_deref()
                .unwrap_or(&config.files.alias),
            &home,
        );
        config.files.ignore = expand_path(
            &sherlock_flags
                .ignore
                .as_deref()
                .unwrap_or(&config.files.ignore),
            &home,
        );
        config.behavior.cache = expand_path(
            &sherlock_flags
                .cache
                .as_deref()
                .unwrap_or(&config.behavior.cache),
            &home,
        );
        config.runtime.sub_menu = sherlock_flags.sub_menu.take();
        config.runtime.method = sherlock_flags.method.take();
        config.runtime.input = sherlock_flags.input.take();
        config.runtime.center = sherlock_flags.center_raw;
        config.runtime.multi = sherlock_flags.multi;
        config.runtime.display_raw = sherlock_flags.display_raw;
        config.runtime.photo_mode = sherlock_flags.photo_mode;
        config.behavior.field = sherlock_flags.field.take();

        if sherlock_flags.daemonize {
            config.behavior.daemonize = true;
        }
        config
    }
}

impl WithRoot for SherlockConfig {
    fn with_root(root: &PathBuf) -> Self {
        let mut default = SherlockConfig::default();
        default.files = ConfigFiles::with_root(root);
        default.appearance = ConfigAppearance::with_root(root);
        default
    }
}
