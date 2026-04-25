use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    sherlock_msg,
    utils::{
        config::{ConfigAppearance, ConfigFiles, SherlockConfig, SherlockFlags, imp::WithRoot},
        errors::{
            SherlockMessage,
            types::{DirAction, FileAction, SherlockErrorType},
        },
        files::{expand_path, home_dir},
    },
};

impl SherlockConfig {
    /// # Arguments
    /// loc: PathBuf
    /// Pathbuf should be a directory **not** a file
    pub fn to_file(loc: PathBuf, ext: &str) -> Result<(), SherlockMessage> {
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
        fn error_message(name: &str, reason: SherlockMessage) {
            eprintln!(
                "✗ Failed to create '{}'. Reason: {}",
                name, reason.error_type
            );
        }
        let write_file = |name: &str, content: &str| {
            let alias_path = path.join(name);
            if !alias_path.exists() {
                if let Err(error) = fs::write(&alias_path, content).map_err(|e| {
                    sherlock_msg!(
                        Warning,
                        SherlockErrorType::FileError(FileAction::Write, alias_path),
                        e
                    )
                }) {
                    error_message(name, error);
                } else {
                    created_message(name);
                }
            } else {
                skipped_message(name);
            }
        };

        // mkdir -p
        fs::create_dir_all(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DirError(DirAction::Create, path.clone()),
                e
            )
        })?;
        // create subdirs
        ensure_dir(&path.join("icons/"), "icons");
        ensure_dir(&path.join("scripts/"), "scripts");
        ensure_dir(&path.join("themes/"), "themes");

        // build default config
        let config = SherlockConfig::with_root(&loc);
        match ext {
            "json" => {
                let json_str = serde_json::to_string_pretty(&config).map_err(|e| {
                    sherlock_msg!(Warning, SherlockErrorType::SerializationError, e)
                })?;
                write_file("config.json", &json_str);
            }
            _ => {
                let toml_str = toml::to_string(&config).map_err(|e| {
                    sherlock_msg!(Warning, SherlockErrorType::SerializationError, e)
                })?;
                write_file("config.toml", &toml_str);
            }
        }

        // Write basic config files
        write_file("sherlockignore", "");
        write_file("sherlock_actions.json", "[]");
        write_file("sherlock_alias.json", "{}");
        write_file("fallback.json", "{}");
        write_file("main.css", "");

        if let Some(loc) = loc.to_str()
            && loc != "~/.config/sherlock/"
        {
            let loc = loc.trim_end_matches("/");
            println!(
                "\nUse \x1b[32msherlock --config {}/config.toml\x1b[0m to run sherlock with the custom configuration.",
                loc
            );
        }

        std::process::exit(0);
    }
    pub fn apply_flags(&mut self, sherlock_flags: &mut SherlockFlags) {
        // Make paths that contain the ~ dir use the correct path
        let home = match home_dir() {
            Ok(h) => h,
            Err(_) => return,
        };

        // Override config files from flags
        if let Some(config) = sherlock_flags.config.as_deref() {
            self.files.config = expand_path(config, &home);
        }
        if let Some(fallback) = sherlock_flags.fallback.as_deref() {
            self.files.fallback = expand_path(fallback, &home);
        }
        if let Some(alias) = sherlock_flags.alias.as_deref() {
            self.files.alias = expand_path(alias, &home);
        }
        if let Some(ignore) = sherlock_flags.ignore.as_deref() {
            self.files.ignore = expand_path(ignore, &home);
        }
        if let Some(cache) = sherlock_flags.cache.as_deref() {
            self.caching.cache = expand_path(cache, &home);
        }
        self.runtime.sub_menu = sherlock_flags.sub_menu.take();
        self.runtime.method = sherlock_flags.method.take();
        self.runtime.input = sherlock_flags.input.take();
        self.runtime.center = sherlock_flags.center_raw;
        self.runtime.multi = sherlock_flags.multi;
        self.runtime.display_raw = sherlock_flags.display_raw;
        self.runtime.photo_mode = sherlock_flags.photo_mode;
        self.runtime.field = sherlock_flags.field.take();

        if let Some(placeholder) = sherlock_flags.placeholder.take() {
            self.appearance.placeholder = placeholder;
        }
    }
}

impl WithRoot for SherlockConfig {
    #[inline(always)]
    fn with_root(root: &Path) -> Self {
        Self {
            files: ConfigFiles::with_root(root),
            appearance: ConfigAppearance::with_root(root),
            ..Default::default()
        }
    }
}
