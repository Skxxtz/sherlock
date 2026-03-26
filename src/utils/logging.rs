use std::io::Write;
use std::{fs, fs::OpenOptions, sync::Mutex};

use chrono::Local;
use once_cell::sync::Lazy;

use crate::sherlock_msg;
use crate::utils::errors::types::{DirAction, FileAction, SherlockErrorType};
use crate::utils::{errors::SherlockMessage, paths};

static LOG_FILE: Lazy<Result<Mutex<std::fs::File>, SherlockMessage>> = Lazy::new(|| {
    let cache_dir = paths::get_cache_dir()?;
    fs::create_dir_all(&cache_dir).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DirError(DirAction::Create, cache_dir.clone()),
            e
        )
    })?;

    let location = cache_dir.join("sherlock.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&location)
        .map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, location),
                e
            )
        })?;

    Ok(Mutex::new(file))
});

pub fn write_log<T: AsRef<str>>(message: T, file: &str, line: u32) -> Result<(), SherlockMessage> {
    let message = message.as_ref();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut log_file = LOG_FILE
        .as_ref()
        .map_err(|e| e.clone())?
        .lock()
        .expect("Failed to lock LOG_FILE..");

    message
        .split("\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .for_each(|msg| {
            writeln!(log_file, "[{}] {}:{} - {}", now, file, line, msg)
                .expect("Failed to write to log file");
        });

    Ok(())
}

#[macro_export]
macro_rules! sher_log {
    // With message only — uses current file!() and line!()
    ($message:expr) => {
        $crate::utils::logging::write_log($message, file!(), line!())
    };

    // With message, file and line explicitly passed
    ($message:expr, $file:expr, $line:expr) => {
        $crate::utils::logging::write_log($message, $file, $line)
    };
}
