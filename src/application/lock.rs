use std::fs::{self, remove_file, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use procfs::process::Process;

use crate::daemon::daemon::SherlockDaemon;

#[sherlock_macro::timing("Ensuring single instance")]
pub fn ensure_single_instance(lock_file: &str) -> Result<LockFile, String> {
    let path = PathBuf::from(lock_file);
    if path.exists() {
        if let Some(content) = fs::read_to_string(&path).ok() {
            if let Some(pid) = content.parse::<i32>().ok() {
                match Process::new(pid) {
                    Ok(_) => {
                        let _ = SherlockDaemon::instance();
                    }
                    Err(_) => {
                        let _ = fs::remove_file(lock_file);
                    }
                }
            }
        }
    }
    LockFile::new(lock_file)
}

pub struct LockFile {
    path: String,
}

impl LockFile {
    pub fn new(path: &str) -> Result<Self, String> {
        if Path::new(path).exists() {
            return Err("Lockfile already exists. Aborting...".to_string());
        }

        match File::create(path) {
            Ok(mut f) => {
                write!(f, "{}", std::process::id()).map_err(|e| e.to_string())?;
                Ok(LockFile {
                    path: path.to_string(),
                })
            }
            Err(e) => Err(format!("Failed to create lock file: {}", e)),
        }
    }

    pub fn remove(&self) -> Result<(), String> {
        if let Err(e) = remove_file(&self.path) {
            return Err(format!("Failed to remove lock file: {}", e));
        }
        Ok(())
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
