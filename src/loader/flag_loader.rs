use std::env;

use super::{
    util::{SherlockError, SherlockErrorType, SherlockFlags},
    Loader,
};

impl Loader {
    pub fn load_flags() -> Result<SherlockFlags, SherlockError> {
        let args: Vec<String> = env::args().collect();
        if args.contains(&"--help".to_string()) {
            let _ = print_help();
            std::process::exit(0);
        }
        if args.contains(&"-h".to_string()) {
            let _ = print_help();
            std::process::exit(0);
        }
        if args.contains(&"--version".to_string()) {
            let _ = print_version();
            std::process::exit(0);
        }

        SherlockFlags::new(args)
    }
}
impl SherlockFlags {
    fn new(args: Vec<String>) -> Result<Self, SherlockError> {
        let home_dir = env::var("HOME").map_err(|e| SherlockError {
            error: SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
            traceback: e.to_string(),
        })?;
        let defaults = SherlockFlags::default().map_err(|e| e)?;

        // Helper closure to extract flag values
        let extract_flag_value = |flag: &str, default: String| {
            args.iter()
                .position(|arg| arg == flag)
                .and_then(|index| args.get(index + 1))
                .map_or(default, |f| f.replace("~", &home_dir).to_string())
                .to_string()
        };
        let check_flag_existance = |flag: &str| {
            args.iter()
                .position(|arg| arg == flag)
                .map_or(false, |_| true)
        };

        Ok(SherlockFlags {
            config: extract_flag_value("--config", defaults.config),
            fallback: extract_flag_value("--fallback", defaults.fallback),
            style: extract_flag_value("--style", defaults.style),
            ignore: extract_flag_value("--ignore", defaults.ignore),
            alias: extract_flag_value("--alias", defaults.alias),
            display_raw: check_flag_existance("--display-raw"),
            center_raw: check_flag_existance("--center"),
            caching: check_flag_existance("--cache"),
            cache: extract_flag_value("--cache", defaults.cache),
            daemonize: check_flag_existance("--daemonize"),
        })
    }

    fn default() -> Result<SherlockFlags, SherlockError> {
        let home_dir = env::var("HOME").map_err(|e| SherlockError {
            error: SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
            traceback: e.to_string(),
        })?;
        Ok(SherlockFlags {
            config: format!("{}/.config/sherlock/config.toml", home_dir),
            fallback: format!("{}/.config/sherlock/fallback.json", home_dir),
            style: format!("{}/.config/sherlock/main.css", home_dir),
            ignore: format!("{}/.config/sherlock/sherlockignore", home_dir),
            alias: format!("{}/.config/sherlock/sherlock_alias.json", home_dir),
            display_raw: false,
            center_raw: false,
            caching: false,
            cache: format!("{}/.cache/sherlock_desktop_cache.json", home_dir),
            daemonize: false,
        })
    }
}

pub fn print_version() -> Result<(), SherlockError> {
    let version = env!("CARGO_PKG_VERSION");
    println!("Sherlock v{}", version);
    println!("Developed by Skxxtz");

    Ok(())
}
pub fn print_help() -> Result<(), SherlockError> {
    let allowed_flags: Vec<(&str, &str)> = vec![
        ("--version", "Print the version of the application."),
        ("--help", "Show this help message with allowed flags."),
        ("--config", "Specify the configuration file to load."),
        ("--fallback", "Specify the fallback file to load."),
        ("--style", "Set the style configuration file."),
        ("--ignore", "Specify the sherlock ignore file"),
        ("--alias", "Specify the sherlock alias file (.json)."),
        (
            "--display-raw",
            "Force Sherlock to use a singular tile to display the piped content",
        ),
        ("--cache", "Specify the sherlock cache file (.json)."),
        (
            "--daemonize",
            "If this flag is set, sherlock will run in daemon mode.",
        ),
    ];

    // Print header
    println!("{:<15} {}", "Flag", "Description");

    for (flag, explanation) in allowed_flags {
        println!("{:<15} {}", flag, explanation);
    }

    println!(
        "\n\nFor more help:\nhttps://github.com/Skxxtz/sherlock/blob/main/README.md#Flags\n\n"
    );

    Ok(())
}
