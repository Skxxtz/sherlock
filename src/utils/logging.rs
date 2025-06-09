use std::io::Write;
use std::{env::home_dir, fs::OpenOptions, sync::Mutex};

use chrono::Local;
use once_cell::sync::Lazy;

static LOG_FILE: Lazy<Mutex<std::fs::File>> = Lazy::new(|| {
    let home = home_dir().expect(&format!(
        "{}:{} - Failed to find home directory.",
        file!(),
        line!()
    ));
    let location = home.join(".sherlock/sherlock.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(location)
        .expect(&format!("{}:{} - Failed to open logfile", file!(), line!()));
    Mutex::new(file)
});

pub fn write_log<T: AsRef<str>>(message: T, file: &str, line: u32) {
    let message = message.as_ref();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut log_file = LOG_FILE.lock().expect("Failed to lock LOG_FILE..");
    message
        .split("\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .for_each(|msg| {
            writeln!(log_file, "[{}] {}:{} - {}", now, file, line, msg)
                .expect("Failed to write to log file");
        });
}

#[macro_export]
macro_rules! sher_log {
    ($message:expr) => {{
        $crate::utils::logging::write_log($message, file!(), line!())
    }};
}
