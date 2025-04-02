use glob::Pattern;
use rayon::prelude::*;
use regex::Regex;
use simd_json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::util::{SherlockError, SherlockErrorType, SherlockFlags};
use super::{util, Loader};
use crate::CONFIG;
use util::{parse_priority, read_file, read_lines, AppData, SherlockAlias};

impl Loader {
    pub fn load_applications_from_disk(
        sherlock_flags: &SherlockFlags,
        applications: Option<HashSet<PathBuf>>,
        priority: f32,
        counts: HashMap<String, f32>,
        decimals: i32,
    ) -> Result<HashMap<String, AppData>, SherlockError> {
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
        let ignore_apps: Vec<Pattern> = match read_lines(&sherlock_flags.ignore) {
            Ok(lines) => lines
                .map_while(Result::ok)
                .filter_map(|line| Pattern::new(&line.to_lowercase()).ok())
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => Err(SherlockError {
                error: SherlockErrorType::FileReadError(sherlock_flags.ignore.to_string()),
                traceback: e.to_string(),
            })?,
        };

        // Parse user-specified 'sherlock_alias.json' file
        let aliases: HashMap<String, SherlockAlias> = match File::open(&sherlock_flags.alias) {
            Ok(f) => simd_json::from_reader(f).map_err(|e| SherlockError {
                error: SherlockErrorType::FileReadError(sherlock_flags.alias.to_string()),
                traceback: e.to_string(),
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => Err(SherlockError {
                error: SherlockErrorType::FileReadError(sherlock_flags.alias.to_string()),
                traceback: e.to_string(),
            })?,
        };

        // Gather '.desktop' files
        let desktop_files: HashSet<PathBuf> = match applications {
            Some(apps) => apps,
            _ => get_desktop_files(system_apps),
        };

        // Parellize opening of all .desktop files and parsing them into AppData
        let apps: HashMap<String, AppData> = desktop_files
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
                        let exec_path = parse_field(&content, &exec_re);
                        let mut exec = if parse_field(&content, &terminal_re) == "true" {
                            format!("{} {}", &config.default_apps.terminal, exec_path)
                        } else {
                            exec_path.to_string()
                        };

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
                        Some((
                            name,
                            AppData {
                                icon,
                                exec,
                                search_string,
                                tag_start: None,
                                tag_end: None,
                                desktop_file: desktop_file_path,
                                priority,
                            },
                        ))
                    }
                    Err(_) => None,
                }
            })
            .collect();
        Ok(apps)
    }

    fn get_new_applications(
        mut apps: HashMap<String, AppData>,
        flags: &SherlockFlags,
        priority: f32,
        counts: HashMap<String, f32>,
        decimals: i32,
    ) -> Result<HashMap<String, AppData>, SherlockError> {
        let system_apps = get_applications_dir();

        // check if sherlock_alias was modified
        let alias_path = Path::new(&flags.alias);
        let cache_path = Path::new(&flags.cache);

        fn modtime(path: &Path) -> Option<SystemTime> {
            match fs::metadata(path) {
                Ok(metadata) => metadata.modified().ok(),
                Err(_) => None,
            }
        }
        match (modtime(&alias_path), modtime(&cache_path)) {
            (Some(t1), Some(t2)) => {
                if t1 >= t2 {
                    return Loader::load_applications_from_disk(
                        flags, None, priority, counts, decimals,
                    );
                }
            }
            _ => {}
        }

        // get all desktop files
        let mut desktop_files = get_desktop_files(system_apps);

        // remove if cached entry doesnt exist on device anympre
        let mut cached_paths = HashSet::with_capacity(apps.capacity());
        apps.retain(|_, v| {
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
        match Loader::load_applications_from_disk(
            flags,
            Some(desktop_files),
            priority,
            counts,
            decimals,
        ) {
            Ok(new_apps) => apps.extend(new_apps),
            _ => {}
        };
        return Ok(apps);
    }

    fn write_cache<T: AsRef<Path>>(apps: &HashMap<String, AppData>, cache_loc: T) {
        let tmp_path = cache_loc.as_ref().with_extension(".tmp");
        if let Ok(f) = File::create(&tmp_path) {
            if let Ok(_) = simd_json::to_writer(f, &apps) {
                let _ = fs::rename(&tmp_path, &cache_loc);
            } else {
                let _ = fs::remove_file(&tmp_path);
            }
        }
    }

    pub fn load_applications(
        sherlock_flags: &SherlockFlags,
        priority: f32,
        counts: HashMap<String, f32>,
        decimals: i32,
    ) -> Result<HashMap<String, AppData>, SherlockError> {
        let cache_loc = sherlock_flags.cache.to_string();
        let flags = sherlock_flags.clone();
        let cached_apps: Option<HashMap<String, AppData>> = File::open(&cache_loc)
            .ok()
            .and_then(|f| simd_json::from_reader(f).ok());

        if let Some(mut apps) = cached_apps {
            // apply the current counts
            for (_, v) in apps.iter_mut() {
                let count = counts.get(&v.exec).unwrap_or(&0.0);
                let priority = parse_priority(priority, *count, decimals);
                v.priority = priority
            }

            // Refresh cache in the background
            let old_apps = apps.clone();
            rayon::spawn_fifo(move || {
                if let Ok(new_apps) =
                    Loader::get_new_applications(old_apps, &flags, priority, counts, decimals)
                {
                    Loader::write_cache(&new_apps, &cache_loc);
                }
            });
            return Ok(apps);
        }

        let apps =
            Loader::load_applications_from_disk(sherlock_flags, None, priority, counts, decimals)?;
        // Write the cache in the background
        let app_clone = apps.clone();
        rayon::spawn_fifo(move || Loader::write_cache(&app_clone, &cache_loc));
        Ok(apps)
    }
}

fn should_ignore(ignore_apps: &Vec<Pattern>, app: &str) -> bool {
    let app_name = app.to_lowercase();
    ignore_apps.iter().any(|pattern| pattern.matches(&app_name))
}

fn get_regex_patterns() -> Result<(Regex, Regex, Regex, Regex, Regex, Regex), SherlockError> {
    fn construct_pattern(key: &str) -> Result<Regex, SherlockError> {
        let pattern = format!(r"(?i){}\s*=\s*(.*)\n", key);
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

fn get_applications_dir() -> HashSet<PathBuf> {
    match env::var("XDG_DATA_DIRS").ok() {
        Some(paths) => {
            let app_dirs: HashSet<PathBuf> = paths
                .split(":")
                .map(|p| PathBuf::from(p).join("applications/"))
                .collect();
            app_dirs
        }
        _ => {
            let home = env::var("HOME").ok().unwrap_or("~".to_string());
            let mut default_paths = vec![
                String::from("/usr/share/applications/"),
                String::from("~/.local/share/applications/"),
            ];
            if let Some(c) = CONFIG.get() {
                default_paths.extend(c.debug.app_paths.clone());
            };

            let paths: HashSet<PathBuf> = default_paths
                .iter()
                .map(|path| path.replace("~", &home))
                .map(|path| PathBuf::from(path))
                .collect();
            paths
        }
    }
}

fn get_desktop_files(dirs: HashSet<PathBuf>) -> HashSet<PathBuf> {
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
