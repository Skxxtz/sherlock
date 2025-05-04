use glob::Pattern;
use rayon::prelude::*;
use regex::Regex;
use simd_json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{util, Loader};
use crate::utils::{
    errors::{SherlockError, SherlockErrorType},
    files::{read_file, read_lines},
};
use crate::CONFIG;
use util::{AppData, SherlockAlias};

impl Loader {
    pub fn load_applications_from_disk(
        applications: Option<HashSet<PathBuf>>,
        priority: f32,
        counts: &HashMap<String, f32>,
        decimals: i32,
    ) -> Result<HashSet<AppData>, SherlockError> {
        let config = CONFIG.get().ok_or(SherlockError {
            error: SherlockErrorType::ConfigError(None),
            traceback: format!(""),
        })?;

        // Define required paths for application parsing
        let system_apps = get_applications_dir();

        // Parse needed fields from the '.desktop'
        let (name_re, icon_re, exec_re, display_re, terminal_re, keywords_re) =
            get_regex_patterns().map_err(|e| return e)?;

        let parse_field = |content: &str, regex: &Regex| {
            regex
                .captures(content)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_default()
        };

        // Parse user-specified 'sherlockignore' file
        let ignore_apps: Vec<Pattern> = match read_lines(&config.files.ignore) {
            Ok(lines) => lines
                .map_while(Result::ok)
                .filter_map(|line| Pattern::new(&line.to_lowercase()).ok())
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => Err(SherlockError {
                error: SherlockErrorType::FileReadError(config.files.ignore.clone()),
                traceback: e.to_string(),
            })?,
        };

        // Parse user-specified 'sherlock_alias.json' file
        let aliases: HashMap<String, SherlockAlias> = match File::open(&config.files.alias) {
            Ok(f) => simd_json::from_reader(f).map_err(|e| SherlockError {
                error: SherlockErrorType::FileReadError(config.files.alias.clone()),
                traceback: e.to_string(),
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => Err(SherlockError {
                error: SherlockErrorType::FileReadError(config.files.alias.clone()),
                traceback: e.to_string(),
            })?,
        };

        // Gather '.desktop' files
        let desktop_files: HashSet<PathBuf> = match applications {
            Some(apps) => apps,
            _ => get_desktop_files(system_apps),
        };

        // Parellize opening of all .desktop files and parsing them into AppData
        let apps: HashSet<AppData> = desktop_files
            .into_par_iter()
            .filter_map(|entry| {
                let r_path = entry.to_str()?;
                match read_file(r_path) {
                    Ok(content) => {
                        if parse_field(&content, &display_re) == "true" {
                            return None;
                        }

                        // Extract keywords, icon, and name fields
                        let mut keywords = parse_field(&content, &keywords_re);
                        let mut icon = parse_field(&content, &icon_re);
                        let mut name = parse_field(&content, &name_re);
                        if name.is_empty() || should_ignore(&ignore_apps, &name) {
                            return None; // Skip entries with empty names
                        }

                        // Construct the executable command
                        let mut exec = config
                            .behavior
                            .global_prefix
                            .as_ref()
                            .map_or(String::new(), |pre| format!("{} ", pre));
                        if parse_field(&content, &terminal_re) == "true" {
                            exec.push_str(&config.default_apps.terminal);
                            exec.push(' ');
                        }
                        exec.push_str(&parse_field(&content, &exec_re));
                        if let Some(flag) = &config.behavior.global_flags {
                            exec.push(' ');
                            exec.push_str(&flag);
                        }

                        // apply aliases
                        if let Some(alias) = aliases.get(&name) {
                            if let Some(alias_name) = alias.name.as_ref() {
                                name = alias_name.to_string();
                            }
                            if let Some(alias_icon) = alias.icon.as_ref() {
                                icon = alias_icon.to_string();
                            }
                            if let Some(alias_keywords) = alias.keywords.as_ref() {
                                keywords = alias_keywords.to_string();
                            }
                            if let Some(alias_exec) = alias.exec.as_ref() {
                                exec = alias_exec.to_string();
                            }
                        };
                        let search_string = format!("{};{}", name, keywords);

                        let desktop_file_path = match config.behavior.caching {
                            true => Some(entry),
                            false => None,
                        };

                        // apply counts
                        let count = counts.get(&exec).unwrap_or(&0.0);
                        let priority = parse_priority(priority, *count, decimals);

                        // Return the processed app data
                        Some(AppData {
                            name,
                            icon: Some(icon),
                            icon_class: None,
                            exec,
                            search_string,
                            tag_start: None,
                            tag_end: None,
                            desktop_file: desktop_file_path,
                            priority,
                        })
                    }
                    Err(_) => None,
                }
            })
            .collect();
        Ok(apps)
    }

    fn get_new_applications(
        mut apps: HashSet<AppData>,
        priority: f32,
        counts: &HashMap<String, f32>,
        decimals: i32,
    ) -> Result<HashSet<AppData>, SherlockError> {
        let system_apps = get_applications_dir();

        // get all desktop files
        let mut desktop_files = get_desktop_files(system_apps);

        // remove if cached entry doesnt exist on device anympre
        let mut cached_paths = HashSet::with_capacity(apps.capacity());
        apps.retain(|v| {
            if let Some(path) = &v.desktop_file {
                if desktop_files.contains(path) {
                    cached_paths.insert(path.clone());
                    return true;
                }
            }
            false
        });

        // get files that are not yet cached
        desktop_files.retain(|v| {
            return !cached_paths.contains(v);
        });

        // get information for uncached applications
        match Loader::load_applications_from_disk(Some(desktop_files), priority, counts, decimals) {
            Ok(new_apps) => apps.extend(new_apps),
            _ => {}
        };
        return Ok(apps);
    }

    fn write_cache<T: AsRef<Path>>(apps: &HashSet<AppData>, cache_loc: T) {
        let path = cache_loc.as_ref();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let tmp_path = path.with_extension(".tmp");

        if let Ok(f) = File::create(&tmp_path) {
            if let Ok(_) = simd_json::to_writer(f, &apps) {
                let _ = fs::rename(&tmp_path, &cache_loc);
            } else {
                let _ = fs::remove_file(&tmp_path);
            }
        }
    }

    pub fn load_applications(
        priority: f32,
        counts: &HashMap<String, f32>,
        decimals: i32,
    ) -> Result<HashSet<AppData>, SherlockError> {
        let config = CONFIG.get().ok_or_else(|| SherlockError {
            error: SherlockErrorType::ConfigError(None),
            traceback: String::new(),
        })?;
        // check if sherlock_alias was modified
        let alias_path = Path::new(&config.files.alias);
        let ignore_path = Path::new(&config.files.ignore);
        let config_path = Path::new(&config.files.config);
        let cache_path = Path::new(&config.behavior.cache);
        let changed = file_has_changed(&alias_path, &cache_path)
            || file_has_changed(&ignore_path, &cache_path)
            || file_has_changed(&config_path, &cache_path);

        if !changed {
            let cached_apps: Option<HashSet<AppData>> = File::open(&config.behavior.cache)
                .ok()
                .and_then(|f| simd_json::from_reader(f).ok());

            if let Some(mut apps) = cached_apps {
                // apply the current counts
                apps = apps
                    .drain()
                    .map(|mut v| {
                        let count = counts.get(&v.exec).unwrap_or(&0.0);
                        let new_priority = parse_priority(priority, *count, decimals);
                        v.priority = new_priority;
                        v
                    })
                    .collect();

                // Refresh cache in the background
                let old_apps = apps.clone();
                rayon::spawn_fifo({
                    let counts_clone = counts.clone();
                    move || {
                        if let Ok(new_apps) = Loader::get_new_applications(
                            old_apps,
                            priority,
                            &counts_clone,
                            decimals,
                        ) {
                            Loader::write_cache(&new_apps, &config.behavior.cache);
                        }
                    }
                });
                return Ok(apps);
            }
        }

        let apps = Loader::load_applications_from_disk(None, priority, counts, decimals)?;
        // Write the cache in the background
        let app_clone = apps.clone();
        rayon::spawn_fifo(move || Loader::write_cache(&app_clone, &config.behavior.cache));
        Ok(apps)
    }
}

fn should_ignore(ignore_apps: &Vec<Pattern>, app: &str) -> bool {
    let app_name = app.to_lowercase();
    ignore_apps.iter().any(|pattern| pattern.matches(&app_name))
}
pub fn parse_priority(priority: f32, count: f32, decimals: i32) -> f32 {
    priority + 1.0 - count * 10f32.powi(-decimals)
}

fn get_regex_patterns() -> Result<(Regex, Regex, Regex, Regex, Regex, Regex), SherlockError> {
    fn construct_pattern(key: &str) -> Result<Regex, SherlockError> {
        let pattern = format!(r#"(?im)^{}\s*=\s*[\'\"]?(.*?)[\'\"]?\s*$"#, key);
        Regex::new(&pattern).map_err(|e| SherlockError {
            error: SherlockErrorType::RegexError(key.to_string()),
            traceback: e.to_string(),
        })
    }
    let name = construct_pattern("Name")?;
    let icon = construct_pattern("Icon")?;
    let exec = construct_pattern("Exec")?;
    let display = construct_pattern("NoDisplay")?;
    let terminal = construct_pattern("Terminal")?;
    let keywords = construct_pattern("Keywords")?;
    return Ok((name, icon, exec, display, terminal, keywords));
}

pub fn get_applications_dir() -> HashSet<PathBuf> {
    let xdg_paths = match env::var("XDG_DATA_DIRS").ok() {
        Some(paths) => {
            let app_dirs: HashSet<PathBuf> = paths
                .split(":")
                .map(|p| PathBuf::from(p).join("applications/"))
                .collect();
            app_dirs
        }
        _ => HashSet::new(),
    };
    let home = env::var("HOME").ok().unwrap_or("~".to_string());
    let mut default_paths = vec![
        String::from("/usr/share/applications/"),
        String::from("~/.local/share/applications/"),
    ];
    if let Some(c) = CONFIG.get() {
        default_paths.extend(c.debug.app_paths.clone());
    };

    let mut paths: HashSet<PathBuf> = default_paths
        .iter()
        .map(|path| path.replace("~", &home))
        .map(|path| PathBuf::from(path))
        .collect();
    paths.extend(xdg_paths);
    paths
}

pub fn get_desktop_files(dirs: HashSet<PathBuf>) -> HashSet<PathBuf> {
    dirs.into_par_iter()
        .filter_map(|dir| {
            fs::read_dir(dir).ok().map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry.ok().and_then(|f| {
                            let path = f.path();
                            if path.extension().and_then(|ext| ext.to_str()) == Some("desktop") {
                                Some(path)
                            } else {
                                None
                            }
                        })
                    })
                    .collect::<Vec<PathBuf>>()
            })
        })
        .flatten()
        .collect::<HashSet<PathBuf>>()
}
pub fn file_has_changed(file_path: &Path, compare_to: &Path) -> bool {
    fn modtime(path: &Path) -> Option<SystemTime> {
        fs::metadata(path).ok().and_then(|m| m.modified().ok())
    }
    match (modtime(&file_path), modtime(&compare_to)) {
        (Some(t1), Some(t2)) if t1 >= t2 => return true,
        _ => {}
    }
    return false;
}

#[test]
fn test_get_applications_dir() {
    // Test input path
    let test_path = Some("/home/cinnamon/.local/share/flatpak/exports/share:/var/lib/flatpak/exports/share:/home/cinnamon/.nix-profile/share:/nix/profile/share:/home/cinnamon/.local/state/nix/profile/share:/etc/profiles/per-user/cinnamon/share:/nix/var/nix/profiles/default/share:/run/current-system/sw/share".to_string());

    // Compute result based on input path
    let res: HashSet<PathBuf> = match test_path {
        Some(path) => path
            .split(":")
            .map(|p| PathBuf::from(p).join("applications/"))
            .collect(),
        _ => HashSet::from([PathBuf::from("/usr/share/applications/")]),
    };

    // Manually insert the paths into HashSet for expected result
    let expected_app_dirs: HashSet<PathBuf> = HashSet::from([
        PathBuf::from("/home/cinnamon/.local/share/flatpak/exports/share/applications/"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications/"),
        PathBuf::from("/home/cinnamon/.nix-profile/share/applications/"),
        PathBuf::from("/nix/profile/share/applications/"),
        PathBuf::from("/home/cinnamon/.local/state/nix/profile/share/applications/"),
        PathBuf::from("/etc/profiles/per-user/cinnamon/share/applications/"),
        PathBuf::from("/nix/var/nix/profiles/default/share/applications/"),
        PathBuf::from("/run/current-system/sw/share/applications/"),
    ]);

    // Assert that the result matches the expected HashSet
    assert_eq!(res, expected_app_dirs);
}
#[test]
fn test_desktop_file_entries() {
    let test_cases = vec![
        String::from("\ntest=1.0"),
        String::from("\ntest='Application'"),
        String::from("\ntest=Example App"),
        String::from("\ntest=\"Sample Utility\""),
        String::from("\ntest='This is an example application'"),
        String::from("\ntest=\"/usr/bin/example-app --example-flag\""),
        String::from("\ntest='/usr/bin/example-app'"),
        String::from("\ntest=example-icon"),
        String::from("\ntest=\"false\""),
        String::from("\ntest='true'"),
        String::from("\ntest=application/x-example;"),
        String::from("\ntest=false"),
        String::from("\ntest='false'"),
        String::from("\ntest='/opt/example'"),
        String::from("\nTest=example-app"),
        String::from(
            "[Desktop Entry]
test=/usr/bin/bssh
        ",
        ),
    ];

    let expected_values: Vec<String> = vec![
        String::from("1.0"),
        String::from("Application"),
        String::from("Example App"),
        String::from("Sample Utility"),
        String::from("This is an example application"),
        String::from("/usr/bin/example-app --example-flag"),
        String::from("/usr/bin/example-app"),
        String::from("example-icon"),
        String::from("false"),
        String::from("true"),
        String::from("application/x-example;"),
        String::from("false"),
        String::from("false"),
        String::from("/opt/example"),
        String::from("example-app"),
        String::from("/usr/bin/bssh"),
    ];

    // Fixed regex pattern: simpler and correctly matching optional quotes
    let pattern = format!(r#"(?im)^{}\s*=\s*[\'\"]?(.*?)[\'\"]?\s*$"#, "test");
    let re = Regex::new(&pattern).expect("Failed to construct regex pattern");

    // Iterate over the test cases and expected results
    test_cases
        .iter()
        .zip(expected_values.iter())
        .for_each(|(case, res)| {
            let catch = re
                .captures(case)
                .expect(&format!("Didn't match the pattern. String: {}", case));
            let group = catch
                .get(1)
                .expect("Group 1 is non-existent")
                .as_str()
                .to_string();
            assert_eq!(group, *res);
        });
}
