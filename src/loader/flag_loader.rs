use std::{env, path::PathBuf, str::FromStr};

use super::Loader;
use crate::utils::{
    config::{SherlockConfig, SherlockFlags},
    errors::SherlockError,
};

impl Loader {
    #[sherlock_macro::timing(name = "Loading flags", level = "setup")]
    pub fn load_flags() -> Result<SherlockFlags, SherlockError> {
        let args: Vec<String> = env::args().collect();
        if args.contains(&"--help".to_string()) {
            let _ = flag_documentation();
            std::process::exit(0);
        }
        if args.contains(&"-h".to_string()) {
            let _ = flag_documentation();
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
    fn extract_flag_value<T: FromStr>(
        args: &[String],
        flag: &str,
        short: Option<&str>,
    ) -> Option<T> {
        let long = args
            .iter()
            .position(|arg| arg == flag)
            .and_then(|i| args.get(i + 1))
            .and_then(|val| val.parse::<T>().ok());

        match &long {
            None => {
                let flag = short?;
                args.iter()
                    .position(|arg| arg == flag)
                    .and_then(|i| args.get(i + 1))
                    .and_then(|val| val.parse::<T>().ok())
            }
            _ => long,
        }
    }
    fn new(args: Vec<String>) -> Result<Self, SherlockError> {
        // Helper closure to extract flag values
        let extract_path_value =
            |flag: &str| Self::extract_flag_value::<PathBuf>(&args, flag, None);
        let check_flag_existence = |flag: &str| args.iter().any(|arg| arg == flag);

        if check_flag_existence("init") {
            let path = extract_path_value("init").unwrap_or(PathBuf::from("~/.config/sherlock/"));
            let x = SherlockConfig::to_file(path);
            println!("{:?}", x);
        }

        Ok(SherlockFlags {
            config: extract_path_value("--config"),
            fallback: extract_path_value("--fallback"),
            style: extract_path_value("--style"),
            ignore: extract_path_value("--ignore"),
            alias: extract_path_value("--alias"),
            display_raw: check_flag_existence("--display-raw"),
            center_raw: check_flag_existence("--center"),
            cache: extract_path_value("--cache"),
            daemonize: check_flag_existence("--daemonize"),
            sub_menu: Self::extract_flag_value::<String>(&args, "--sub-menu", Some("-sm")),
            method: Self::extract_flag_value::<String>(&args, "--method", None),
            field: Self::extract_flag_value::<String>(&args, "--field", None),
            multi: check_flag_existence("--multi"),
            photo_mode: check_flag_existence("--photo"),
            input: Self::extract_flag_value::<bool>(&args, "--input", None),
        })
    }
}

pub fn print_version() -> Result<(), SherlockError> {
    let version = env!("CARGO_PKG_VERSION");
    println!("Sherlock v{}", version);
    println!("Developed by Skxxtz");

    Ok(())
}
pub fn flag_documentation() -> Result<(), SherlockError> {
    let allowed_flags: Vec<(&str, &str)> = vec![
        ("\nBASICS:", ""),
        ("--version", "Print the version of the application."),
        ("--help", "Show this help message with allowed flags."),
        ("init", "Writes default configs into your config directory."),
        ("\nFILES:", ""),
        ("--config", "Specify the configuration file to load."),
        ("--fallback", "Specify the fallback file to load."),
        ("--style", "Set the style configuration file."),
        ("--ignore", "Specify the sherlock ignore file"),
        ("--alias", "Specify the sherlock alias file (.json)."),
        ("--cache", "Specify the sherlock cache file (.json)."),
        ("\nBEHAVIOR:", ""),
        (
            "--daemonize",
            "If this flag is set, sherlock will run in daemon mode.",
        ),
        (
            "--sub-menu",
            "Start sherlock with an alias active already. For example 'pm' for power menu",
        ),
        (
            "--time-inspect",
            "Show time for loading launchers and from 0 to full content",
        ),
        ("\nPIPE MODE:", ""),
        (
            "--display-raw",
            "Force Sherlock to use a singular tile to display the piped content",
        ),
        (
            "--method",
            "Specifies what to do with the selected data row",
        ),
        (
            "--field",
            "Specifies which of your fields should be printed on return press",
        ),
    ];

    // Print header
    println!("{:<15} {}", "Flag", "Description");

    for (flag, explanation) in allowed_flags {
        println!("{:<15} {}", flag, explanation);
    }

    println!(
        "\n\nFor more help:\nhttps://github.com/Skxxtz/sherlock/blob/documentation/docs/flags.md\n\n"
    );

    Ok(())
}
