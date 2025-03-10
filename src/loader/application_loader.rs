use glob::Pattern;
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, read_to_string};
use std::path::Path;

use super::util::{SherlockError, SherlockFlags, SherlockErrorType};
use super::{util, Loader};
use crate::CONFIG;
use util::{read_file, AppData, SherlockAlias};

impl Loader {
    pub fn load_applications(
        sherlock_flags: &SherlockFlags,
    ) -> Result<HashMap<String, AppData>, SherlockError> {
        let config = CONFIG.get().ok_or(SherlockError {
            error: SherlockErrorType::ConfigError(None),
            traceback: format!(""),
        })?;
        // Define required paths for application parsing
        let sherlock_ignore_path = sherlock_flags.ignore.clone();
        let sherlock_alias_path = sherlock_flags.alias.clone();
        let system_apps = "/usr/share/applications/";

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
        let mut ignore_apps: Vec<Pattern> = Default::default();
        if Path::new(&sherlock_ignore_path).exists() {
            ignore_apps = read_to_string(&sherlock_ignore_path)
                .map_err(|e| SherlockError {
                    error: SherlockErrorType::FileReadError(sherlock_ignore_path),
                    traceback: e.to_string(),
                })?
                .lines()
                .filter_map(|line| {
                    let line = line.to_lowercase();
                    Pattern::new(&line).ok()
                })
                .collect::<Vec<Pattern>>();
        }

        // Parse user-specified 'sherlock_alias.json' file
        let mut aliases: HashMap<String, SherlockAlias> = Default::default();
        if Path::new(&sherlock_alias_path).exists() {
            let json_data = read_to_string(&sherlock_alias_path).map_err(|e| SherlockError {
                error: SherlockErrorType::FileReadError(sherlock_alias_path.clone()),
                traceback: e.to_string(),
            })?;
            aliases = serde_json::from_str(&json_data).map_err(|e| SherlockError {
                error: SherlockErrorType::FileParseError(sherlock_alias_path),
                traceback: e.to_string(),
            })?
        }

        // Gather '.desktop' files
        let dektop_files: Vec<_> = fs::read_dir(system_apps)
            .expect("Unable to read/access /usr/share/applications directory")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "desktop")
                    .unwrap_or(false)
            })
            .collect();

        // Parellize opening of all files and reading into vector
        let file_contents: Vec<String> = dektop_files
            .into_par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                let rpath = path.to_str()?;
                read_file(rpath).ok()
            })
            .collect();

        // Pararellize parsing of the '.desktop' files contents
        let apps: HashMap<String, AppData> = file_contents
            .into_par_iter()
            .filter_map(|content| {
                // Skip if "NoDisplay" field is set to "true"
                if parse_field(&content, &display_re) == "true" {
                    return None;
                }

                // Extract fields
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

                // Return the processed app data
                Some((
                    name,
                    AppData {
                        icon,
                        exec,
                        search_string,
                        tag_start: None,
                        tag_end: None,
                    },
                ))
            })
            .collect();
        Ok(apps)
    }
}

fn should_ignore(ignore_apps: &Vec<Pattern>, app: &String) -> bool {
    let app_name = app.to_lowercase();
    ignore_apps.iter().any(|pattern| pattern.matches(&app_name))
}

fn get_regex_patterns() -> Result<(Regex, Regex, Regex, Regex, Regex, Regex), SherlockError> {
    fn construct_pattern(key: &str)->Result<Regex, SherlockError>{
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
