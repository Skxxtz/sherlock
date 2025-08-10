use std::env;
use std::fs::{self, remove_file, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use nix::sys::signal::kill;
use nix::sys::signal::Signal::SIGKILL;
use nix::unistd::Pid;
use procfs::process::Process;

use crate::daemon::daemon::SherlockDaemon;
use crate::sherlock_error;
use crate::utils::errors::{SherlockError, SherlockErrorType};


pub struct LockFile {
    path: PathBuf,
}
impl LockFile {
    #[sherlock_macro::timing(name = "Ensuring single instance", level = "setup")]
    pub fn single_instance(lock_file: &str) -> Result<Self, SherlockError> {
        let path = PathBuf::from(lock_file);
        let take_over = env::args().find(|s| s == "--take-over");
        if path.exists() {
            let content = fs::read_to_string(&path).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileReadError(path.clone()),
                    e.to_string()
                )
            })?;
            let pid = content.parse::<i32>().map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileParseError(path.clone()),
                    e.to_string()
                )
            })?;
            match Process::new(pid) {
                Ok(_) => {
                    if take_over.is_some() {
                        let pid = Pid::from_raw(pid);
                        let _ = kill(pid, SIGKILL);
                        let _ = fs::remove_file(lock_file);
                    } else {
                        let _ = SherlockDaemon::instance();
                    }
                }
                Err(_) => {
                    let _ = fs::remove_file(lock_file);
                }
            }
        }
        LockFile::new(lock_file)
    }
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, SherlockError> {
        let path = path.as_ref();
        if path.exists() {
            return Err(sherlock_error!(
                SherlockErrorType::LockfileExistsError,
                "".to_string()
            ));
        }

        match File::create(&path) {
            Ok(mut f) => {
                write!(f, "{}", std::process::id()).map_err(|e| {
                    sherlock_error!(
                        SherlockErrorType::FileWriteError(path.to_path_buf()),
                        e.to_string()
                    )
                })?;
                Ok(LockFile {
                    path: path.to_path_buf(),
                })
            }
            Err(e) => Err(sherlock_error!(
                SherlockErrorType::FileWriteError(path.to_path_buf()),
                e.to_string()
            )),
        }
    }

    pub fn remove(&self) -> Result<(), SherlockError> {
        remove_file(&self.path).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileRemoveError(self.path.clone()),
                e.to_string()
            )
        })
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
