use std::io::Write;
use std::{fs, fs::OpenOptions, sync::Mutex};

use chrono::Local;
use once_cell::sync::Lazy;

use crate::sherlock_error;
use crate::utils::{errors::SherlockError, errors::SherlockErrorType, paths};

static LOG_FILE: Lazy<Result<Mutex<std::fs::File>, SherlockError>> = Lazy::new(|| {
    let cache_dir = paths::get_cache_dir()?;
    fs::create_dir_all(&cache_dir).map_err(|e| {
        sherlock_error!(
            SherlockErrorType::DirCreateError(cache_dir.display().to_string()),
            e.to_string()
        )
    })?;

    let location = cache_dir.join("sherlock.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&location)
        .map_err(|e| sherlock_error!(SherlockErrorType::FileWriteError(location), e.to_string()))?;

    Ok(Mutex::new(file))
});

pub fn write_log<T: AsRef<str>>(message: T, file: &str, line: u32) -> Result<(), SherlockError> {
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
    ($message:expr) => {{
        $crate::utils::logging::write_log($message, file!(), line!())
    }};
}
