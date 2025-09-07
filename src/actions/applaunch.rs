use gio::glib::spawn_command_line_async;

use crate::{
    sher_log, sherlock_error,
    utils::{
        config::ConfigGuard,
        errors::{SherlockError, SherlockErrorType},
    },
};

pub fn applaunch(exec: &str, terminal: bool) -> Result<(), SherlockError> {
    let config = ConfigGuard::read()?;
    let mut parts = Vec::new();

    if let Some(pre) = &config.behavior.global_prefix {
        parts.push(pre.to_string());
    }
    if terminal {
        parts.push(config.default_apps.terminal.clone());
        parts.push("-e".to_string());
    }
    parts.push(exec.to_string());
    if let Some(flag) = &config.behavior.global_flags {
        parts.push(flag.to_string());
    }
    parts.retain(|s| !s.starts_with("%"));

    let cmd = parts.join(" ").trim().to_string();
    match spawn_command_line_async(&cmd) {
        Ok(_) => {
            let _ = sher_log!(format!("Detached process started: {}.", cmd));
            Ok(())
        }
        Err(e) => {
            let _ = sher_log!(format!("Failed to detach command: {}\nError: {}", cmd, e));

            Err(sherlock_error!(
                SherlockErrorType::CommandExecutionError(cmd),
                e.to_string()
            ))
        }
    }
}

pub fn _split_as_command(cmd: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quoting = false;
    let mut prev = '\0';
    let mut double_escape = false;

    for c in cmd.chars() {
        if double_escape {
            // Escape inside quotes in Exec value as specified by
            // https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html
            double_escape = false;
            match c {
                '"' | '`' | '$' | '\\' => {
                    current.pop();
                    current.push(c);
                    prev = '\0';
                    continue;
                }
                _ => current.push('\\'),
            }
        }
        if quoting && c == '\\' && prev == '\\' {
            double_escape = true;
        } else if c == '"' {
            quoting = !quoting;
        } else if !quoting && c.is_whitespace() && !current.is_empty() {
            parts.push(current.clone());
            current.clear();
        } else {
            current.push(c);
        }
        prev = c;
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts.retain(|s| !s.starts_with("%"));

    parts
}
