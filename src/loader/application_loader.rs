use glob::Pattern;
use gpui::SharedString;
use rayon::prelude::*;
use simd_json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use super::Loader;
use super::utils::ApplicationAction;
use super::utils::{AppData, SherlockAlias};
use crate::launcher::Launcher;
use crate::loader::resolve_icon_path;
use crate::prelude::PathHelpers;
use crate::utils::cache::BinaryCache;
use crate::utils::errors::types::{FileAction, SherlockErrorType};
use crate::utils::{config::ConfigGuard, errors::SherlockMessage, files::read_lines};
use crate::{sher_log, sherlock_msg};

impl Loader {
    pub fn load_applications_from_disk(
        launcher: Arc<Launcher>,
        applications: Option<Vec<PathBuf>>,
        counts: &HashMap<String, u32>,
        decimals: i32,
        use_keywords: bool,
    ) -> Result<Vec<AppData>, SherlockMessage> {
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
            Err(e) => Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, config.files.ignore.clone()),
                e
            ))?,
        };

        // Parse user-specified 'sherlock_alias.json' file
        let aliases: HashMap<String, SherlockAlias> = match File::open(&config.files.alias) {
            Ok(f) => simd_json::from_reader(f)
                .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, config.files.alias.clone()),
                e
            ))?,
        };
        let aliases = Arc::new(RwLock::new(aliases));

        // Gather '.desktop' files
        let desktop_files: Vec<PathBuf> = match applications {
            Some(apps) => apps,
            _ => get_desktop_files(system_apps),
        };

        // Parellize opening of all .desktop files and parsing them into AppData
        let apps: Vec<AppData> = desktop_files
            .into_par_iter()
            .filter_map(|entry| {
                let r_path = entry.to_str()?;
                match read_lines(r_path) {
                    Ok(content) => {
                        let mut buffer = Vec::new();
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
                                    buffer.push(Arc::new(current_action))
                                }
                                current_action = ApplicationAction::new("app_launcher");
                                continue;
                            }

                            let Some(ref section) = current_section else {
                                continue;
                            };

                            if let Some((key, value)) = line.split_once('=') {
                                let key = key.trim().to_ascii_lowercase();
                                let value = value.trim();
                                if section == "Desktop Entry" {
                                    match key.as_ref() {
                                        "name" => {
                                            data.name = {
                                                if should_ignore(&ignore_apps, value) {
                                                    return None;
                                                }
                                                Some(SharedString::from(value.to_string()))
                                            }
                                        }
                                        "icon" => {
                                            data.icon = resolve_icon_path(value);
                                        }
                                        "exec" => data.exec = Some(value.to_string()),
                                        "nodisplay" if value.eq_ignore_ascii_case("true") => {
                                            return None;
                                        }
                                        "hidden" if value.eq_ignore_ascii_case("true") => {
                                            return None;
                                        }
                                        "terminal" => {
                                            data.terminal = value.eq_ignore_ascii_case("true");
                                        }
                                        "keywords" => data.search_string = value.to_lowercase(),
                                        _ => {}
                                    }
                                } else {
                                    // Application Actions
                                    match key.as_ref() {
                                        "name" => {
                                            current_action.name =
                                                Some(SharedString::from(value.to_string()))
                                        }
                                        "exec" => current_action.exec = Some(value.to_string()),
                                        "icon" => current_action.icon = resolve_icon_path(value),
                                        _ => {}
                                    }
                                    if current_action.icon.is_none() {
                                        current_action.icon = data.icon.clone();
                                    }
                                    if current_action.is_full() {
                                        buffer.push(Arc::new(current_action));
                                        current_action = ApplicationAction::new("app_launcher");
                                        current_section = None;
                                    }
                                }
                            }
                        }
                        let alias = {
                            let mut aliases = aliases.write().unwrap();
                            aliases.remove(data.name.as_ref().unwrap().as_str())
                        };
                        data.apply_alias(&launcher, alias, use_keywords, buffer);
                        // apply counts
                        let count = data
                            .exec
                            .as_ref()
                            .and_then(|exec| counts.get(exec))
                            .unwrap_or(&0);
                        let priority = parse_priority(launcher.priority as f32, *count, decimals);
                        data.priority = Some(priority);
                        Some(data)
                    }
                    Err(_) => None,
                }
            })
            .collect();
        Ok(apps)
    }

    fn get_new_applications(
        launcher: Arc<Launcher>,
        mut apps: Vec<AppData>,
        counts: &HashMap<String, u32>,
        decimals: i32,
        last_changed: Option<SystemTime>,
        use_keywords: bool,
    ) -> Result<Vec<AppData>, SherlockMessage> {
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
        desktop_files.retain(|v| !cached_paths.contains(v));

        // get information for uncached applications
        if let Ok(new_apps) = Loader::load_applications_from_disk(
            launcher,
            Some(desktop_files),
            counts,
            decimals,
            use_keywords,
        ) {
            apps.extend(new_apps);
        }

        Ok(apps)
    }

    pub fn load_applications(
        launcher: Arc<Launcher>,
        counts: &HashMap<String, u32>,
        decimals: i32,
        use_keywords: bool,
    ) -> Result<Vec<AppData>, SherlockMessage> {
        let config = ConfigGuard::read()?;
        // check if sherlock_alias was modified
        let changed = file_has_changed(&config.files.alias, &config.caching.cache)
            || file_has_changed(&config.files.ignore, &config.caching.cache)
            || file_has_changed(&config.files.config, &config.caching.cache);

        if !changed {
            let _ = sher_log!("Loading cached apps");
            let cached_apps: Vec<AppData> = BinaryCache::read(&config.caching.cache)?;

            let cleaned_apps: Vec<AppData> = cached_apps
                .into_iter()
                .map(|mut v| {
                    let count = v
                        .exec
                        .as_ref()
                        .and_then(|exec| counts.get(exec))
                        .unwrap_or(&0);
                    let new_priority = parse_priority(launcher.priority as f32, *count, decimals);
                    v.priority = Some(new_priority);
                    v
                })
                .collect();

            // Refresh cache in the background
            let old_apps = cleaned_apps.clone();
            let last_changed = config.caching.cache.modtime();
            let cache = config.caching.cache.clone();
            rayon::spawn_fifo({
                let counts_clone = counts.clone();
                move || {
                    if let Ok(new_apps) = Loader::get_new_applications(
                        launcher,
                        old_apps,
                        &counts_clone,
                        decimals,
                        last_changed,
                        use_keywords,
                    ) {
                        if let Err(e) = BinaryCache::write(cache, &new_apps) {
                            eprintln!("{}", e.error_type);
                        }
                    }
                }
            });
            return Ok(cleaned_apps);
        }

        let _ = sher_log!("Updating cached apps");
        let apps =
            Loader::load_applications_from_disk(launcher, None, counts, decimals, use_keywords)?;
        // Write the cache in the background
        let app_clone = apps.clone();
        let cache = config.caching.cache.clone();
        rayon::spawn_fifo(move || {
            if let Err(e) = BinaryCache::write(cache, &app_clone) {
                eprintln!("{}", e.error_type);
            }
        });
        Ok(apps)
    }
}

fn should_ignore(ignore_apps: &[Pattern], app: &str) -> bool {
    let app_name = app.to_lowercase();
    ignore_apps.iter().any(|pattern| pattern.matches(&app_name))
}
pub fn parse_priority(priority: f32, count: u32, decimals: i32) -> f32 {
    priority + 0.99 - count as f32 * 10f32.powi(-decimals)
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
    let mut paths: HashSet<PathBuf> = ["/usr/share/applications/", "~/.local/share/applications/"]
        .iter()
        .map(|p| PathBuf::from(p.replace("~", &home)))
        .collect();

    if let Ok(c) = ConfigGuard::read() {
        paths.extend(c.debug.app_paths.iter().map(PathBuf::from));
    }
    paths.extend(xdg_paths);
    paths
}

pub fn get_desktop_files(mut dirs: HashSet<PathBuf>) -> Vec<PathBuf> {
    fn read_desktop_dir(dir: PathBuf) -> Option<HashMap<String, PathBuf>> {
        fs::read_dir(dir).ok().map(|entries| {
            entries
                .filter_map(|entry| {
                    entry.ok().and_then(|f| {
                        let path = f.path();
                        let extension = path.extension().and_then(|ext| ext.to_str())?;
                        if extension == "desktop" {
                            let stem = path.file_stem().and_then(|s| s.to_str())?;
                            Some((stem.to_string(), path))
                        } else {
                            None
                        }
                    })
                })
                .collect::<HashMap<String, PathBuf>>()
        })
    }
    let local_app_dir = PathBuf::from(env::var("HOME").unwrap_or_else(|_| "~".into()))
        .join(".local/share/applications");
    let local_dir = dirs.take(&local_app_dir);

    let mut dirs: HashMap<String, PathBuf> = dirs
        .into_par_iter()
        .filter(|dir| dir.is_dir())
        .filter_map(|dir| read_desktop_dir(dir))
        .flatten()
        .collect();

    if let Some(local_dir) = local_dir.and_then(|d| read_desktop_dir(d)) {
        for (name, path) in local_dir {
            dirs.insert(name, path);
        }
    }

    dirs.into_values().collect()
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
