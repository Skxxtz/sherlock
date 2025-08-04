use std::fs::write;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use crate::loader::util::AppData;
use crate::loader::Loader;
use crate::sherlock_error;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::utils::paths;

use super::LauncherType;

#[derive(Clone, Debug)]
pub struct ThemePicker {
    pub location: PathBuf,
    pub themes: Vec<AppData>,
}
impl ThemePicker {
    pub fn new<T: AsRef<Path>>(loc: T, prio: f32) -> LauncherType {
        let absolute = loc.as_ref();
        if !absolute.is_dir() {
            return LauncherType::Empty;
        }
        let mut themes: Vec<AppData> = absolute
            .read_dir()
            .ok()
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path.is_file() || path.is_symlink())
                    .filter_map(|path| {
                        if path.extension()?.to_str()? == "css" {
                            let name = path.file_name()?.to_str()?;
                            Some(AppData::new_for_theme(name, path.to_str(), prio))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        themes.push(AppData::new_for_theme("Unset", Some(""), prio));

        if themes.is_empty() {
            return LauncherType::Empty;
        }

        LauncherType::Theme(ThemePicker {
            location: absolute.to_path_buf(),
            themes,
        })
    }
    pub fn select_theme<T>(theme: T, exit: bool) -> Result<(), SherlockError>
    where
        T: AsRef<[u8]>,
    {
        let absolute = Self::get_cached()?;
        write(&absolute, theme).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileWriteError(absolute.clone()),
                e.to_string()
            )
        })?;
        if !exit {
            if let Err(error) = Loader::load_css(false) {
                let _result = error.insert(false);
            }
        }
        Ok(())
    }

    pub fn get_cached() -> Result<PathBuf, SherlockError> {
        let config_dir = paths::get_config_dir()?;
        let absolute = config_dir.join("theme.txt");
        if let Some(parents) = absolute.parent() {
            std::fs::create_dir_all(parents).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::DirCreateError(parents.to_string_lossy().to_string()),
                    e.to_string()
                )
            })?;
        }
        if !absolute.is_file() {
            File::create(&absolute).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileWriteError(absolute.clone()),
                    e.to_string()
                )
            })?;
        }
        Ok(absolute)
    }
}
