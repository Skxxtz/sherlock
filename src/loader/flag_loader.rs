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
        if args.contains(&"-v".to_string()) {
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
            let extension = Self::extract_flag_value::<String>(&args, "--file-type", Some("-f"))
                .unwrap_or(String::from("toml"));
            let x = SherlockConfig::to_file(path, &extension);
            eprintln!("{:?}", x);
        }

        Ok(SherlockFlags {
            config_dir: extract_path_value("--config-dir"),
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
            toggle: check_flag_existence("--toggle"),
            placeholder: Self::extract_flag_value::<String>(&args, "--placeholder", Some("-p")),
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
        ("-v, --version", "Print the version of the application."),
        ("-h, --help", "Show this help message with allowed flags."),
        ("init", "Writes default configs into your config directory."),
        ("\nFILES:", ""),
        ("--config", "Specify the configuration file to load."),
        ("--fallback", "Specify the fallback file to load."),
        ("--style", "Set the style configuration file."),
        ("--ignore", "Specify the Sherlock ignore file"),
        ("--alias", "Specify the Sherlock alias file (.json)."),
        ("--cache", "Specify the Sherlock cache file (.json)."),
        (
            "--config-dir",
            "Specify the directly Sherlock will look for its configuration in.",
        ),
        ("\nBEHAVIOR:", ""),
        (
            "-p, --placeholder",
            "Overwrite the placeholder text of the search bar.",
        ),
        (
            "--daemonize",
            "If this flag is set, Sherlock will run in daemon mode.",
        ),
        (
            "-sm, --sub-menu",
            "Start Sherlock with an alias active already. For example 'pm' for power menu",
        ),
        (
            "--multi",
            "Start Sherlock in \"multi mode\". This mode allows to select and execute multiple entries.",
        ),
        (
            "--photo",
            "Start Sherlock in \"photo mode\". This mode temporarily disables Sherlock from closing on focus loss.",
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
    let longest = allowed_flags
        .iter()
        .max_by_key(|item| item.0.len())
        .map_or(20, |i| i.0.len() + 5);

    for (flag, explanation) in allowed_flags {
        println!("{:<width$} {}", flag, explanation, width = longest);
    }

    println!(
        "\n\nFor more help:\nhttps://github.com/Skxxtz/sherlock/blob/documentation/docs/flags.md\n\n"
    );

    Ok(())
}
