use std::fs::{remove_file, File};
use std::path::Path;

pub fn ensure_single_instance(lock_file: &str) -> Result<LockFile, String> {
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
            Ok(_) => Ok(LockFile {
                path: path.to_string(),
            }),
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
