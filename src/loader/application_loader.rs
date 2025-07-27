use async_std::sync::Mutex;
use glob::Pattern;
use rayon::prelude::*;
use simd_json;
use simd_json::prelude::ArrayTrait;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use super::util::ApplicationAction;
use super::{util, Loader};
use crate::prelude::PathHelpers;
use crate::utils::config::ConfigGuard;
use crate::utils::{
    errors::{SherlockError, SherlockErrorType},
    files::read_lines,
};
use crate::{sher_log, sherlock_error};
use util::{AppData, SherlockAlias};

impl Loader {
    pub fn load_applications_from_disk(
        applications: Option<HashSet<PathBuf>>,
        priority: f32,
        counts: &HashMap<String, f32>,
        decimals: i32,
    ) -> Result<HashSet<AppData>, SherlockError> {
        let config = ConfigGuard::read()?;

        // Define required paths for application parsing
        let system_apps = get_applications_dir();

        // Parse user-specified 'sherlockignore' file
        let ignore_apps: Vec<Pattern> = match read_lines(&config.files.ignore) {
            Ok(lines) => lines
                .map_while(Result::ok)
                .filter_map(|line| Pattern::new(&line.to_lowercase()).ok())
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => Err(sherlock_error!(
                SherlockErrorType::FileReadError(config.files.ignore.clone()),
                e.to_string()
            ))?,
        };

        // Parse user-specified 'sherlock_alias.json' file
        let aliases: HashMap<String, SherlockAlias> = match File::open(&config.files.alias) {
            Ok(f) => simd_json::from_reader(f).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileReadError(config.files.alias.clone()),
                    e.to_string()
                )
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => Err(sherlock_error!(
                SherlockErrorType::FileReadError(config.files.alias.clone()),
                e.to_string()
            ))?,
        };
        let aliases = Arc::new(Mutex::new(aliases));

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
                match read_lines(r_path) {
                    Ok(content) => {
                        let mut data = AppData::new();
                        let mut current_section = None;
                        let mut current_action = ApplicationAction::new("app_launcher");
                        data.desktop_file = Some(entry);
                        for line in content.flatten() {
                            let line = line.trim();
                            // Skip useless lines
                            if line.is_empty() || line.starts_with('#') {
                                continue;
                            }
                            if line.starts_with('[') && line.ends_with(']') {
                                current_section = Some(line[1..line.len() - 1].to_string());
                                if current_action.is_valid() {
                                    data.actions.push(current_action.clone())
                                }
                                current_action = ApplicationAction::new("app_launcher");
                                continue;
                            }
                            if current_section.is_none() {
                                continue;
                            }
                            if let Some((key, value)) = line.split_once('=') {
                                let key = key.trim().to_ascii_lowercase();
                                let value = value.trim();
                                if current_section.as_deref().unwrap() == "Desktop Entry" {
                                    match key.as_ref() {
                                        "name" => {
                                            data.name = {
                                                if should_ignore(&ignore_apps, value) {
                                                    return None;
                                                }
                                                value.to_string()
                                            }
                                        }
                                        "icon" => data.icon = Some(value.to_string()),
                                        "exec" => data.exec = Some(value.to_string()),
                                        "nodisplay" if value.eq_ignore_ascii_case("true") => {
                                            return None
                                        }
                                        "terminal" => {
                                            data.terminal = value.eq_ignore_ascii_case("true");
                                        }
                                        "keywords" => data.search_string = value.to_string(),
                                        _ => {}
                                    }
                                } else {
                                    // Application Actions
                                    match key.as_ref() {
                                        "name" => current_action.name = Some(value.to_string()),
                                        "exec" => current_action.exec = Some(value.to_string()),
                                        "icon" => current_action.icon = Some(value.to_string()),
                                        _ => {}
                                    }
                                    if current_action.is_full() {
                                        data.actions.push(current_action.clone());
                                        current_action = ApplicationAction::new("app_launcher");
                                        current_section = None;
                                    }
                                }
                            }
                        }
                        data.actions
                            .iter_mut()
                            .filter(|action| action.icon.is_none())
                            .for_each(|action| action.icon = data.icon.clone());
                        let alias = {
                            let mut aliases = aliases.lock_blocking();
                            aliases.remove(&data.name)
                        };
                        data.apply_alias(alias);
                        // apply counts
                        let count = data
                            .exec
                            .as_ref()
                            .and_then(|exec| counts.get(exec))
                            .unwrap_or(&0.0);
                        println!("{:?}", decimals);
                        let priority = parse_priority(priority, *count, decimals);
                        data.priority = priority;
                        Some(data)
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
        last_changed: Option<SystemTime>,
    ) -> Result<HashSet<AppData>, SherlockError> {
        let system_apps = get_applications_dir();

        // get all desktop files
        let mut desktop_files = get_desktop_files(system_apps);

        // remove if cached entry doesnt exist on device anympre
        let mut cached_paths = HashSet::with_capacity(apps.capacity());
        apps.retain(|v| {
            if let Some(path) = &v.desktop_file {
                if desktop_files.contains(path) {
                    // Do not flag files as cached that have been modified after the cache has last been
                    // modified
                    if let (Some(modtime), Some(last_changed)) = (path.modtime(), last_changed) {
                        if modtime < last_changed {
                            cached_paths.insert(path.clone());
                        } else {
                            return false;
                        }
                    }
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
        let config = ConfigGuard::read()?;
        // check if sherlock_alias was modified
        let changed = file_has_changed(&config.files.alias, &config.behavior.cache)
            || file_has_changed(&config.files.ignore, &config.behavior.cache)
            || file_has_changed(&config.files.config, &config.behavior.cache);

        if !changed {
            let _ = sher_log!("Loading cached apps");
            let cached_apps: Option<HashSet<AppData>> = File::open(&config.behavior.cache)
                .ok()
                .and_then(|f| simd_json::from_reader(f).ok());

            if let Some(mut apps) = cached_apps {
                // apply the current counts
                apps = apps
                    .drain()
                    .map(|mut v| {
                        let count = v
                            .exec
                            .as_ref()
                            .and_then(|exec| counts.get(exec))
                            .unwrap_or(&0.0);
                        let new_priority = parse_priority(priority, *count, decimals);
                        v.priority = new_priority;
                        v
                    })
                    .collect();

                // Refresh cache in the background
                let old_apps = apps.clone();
                let last_changed = config.behavior.cache.modtime();
                let cache = config.behavior.cache.clone();
                rayon::spawn_fifo({
                    let counts_clone = counts.clone();
                    move || {
                        if let Ok(new_apps) = Loader::get_new_applications(
                            old_apps,
                            priority,
                            &counts_clone,
                            decimals,
                            last_changed,
                        ) {
                            Loader::write_cache(&new_apps, cache);
                        }
                    }
                });
                return Ok(apps);
            }
        }

        let _ = sher_log!("Updating cached apps");
        let apps = Loader::load_applications_from_disk(None, priority, counts, decimals)?;
        // Write the cache in the background
        let app_clone = apps.clone();
        let cache = config.behavior.cache.clone();
        rayon::spawn_fifo(move || Loader::write_cache(&app_clone, cache));
        Ok(apps)
    }
}

fn should_ignore(ignore_apps: &Vec<Pattern>, app: &str) -> bool {
    let app_name = app.to_lowercase();
    ignore_apps.iter().any(|pattern| pattern.matches(&app_name))
}
pub fn parse_priority(priority: f32, count: f32, decimals: i32) -> f32 {
    if count == 0.0 {
        priority + 0.99
    } else {
        priority + 0.99 - count * 10f32.powi(-decimals)
    }
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
    if let Ok(c) = ConfigGuard::read() {
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
        .filter(|dir| dir.is_dir())
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
    match (&file_path.modtime(), &compare_to.modtime()) {
        (Some(t1), Some(t2)) if t1 > t2 => true, // t1 is newer than t2
        (Some(t1), Some(t2)) if t1 < t2 => false, // t1 is older than t2
        _ => true,                               // if there is a modtime missing
    }
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

impl PathHelpers for Path {
    fn modtime(&self) -> Option<SystemTime> {
        self.metadata().ok().and_then(|m| m.modified().ok())
    }
}
