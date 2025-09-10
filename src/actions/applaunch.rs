use std::os::fd::AsRawFd;
use std::process::Command;

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

    let cmd = parts.join(" ").trim().to_string();
    let mut parts = split_as_command(&cmd).into_iter();

    let mut command = Command::new(parts.next().ok_or(sherlock_error!(
        SherlockErrorType::CommandExecutionError(cmd.clone()),
        format!("Failed to get first base command")
    ))?);
    command.args(parts);

    match launch_detached(command) {
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

pub fn launch_detached(mut command: Command) -> std::io::Result<()> {
    unsafe {
        match libc::fork() {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {
                // Child process
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                // Fork again to prevent from acquiring a controlling terminal
                match libc::fork() {
                    -1 => return Err(std::io::Error::last_os_error()),
                    0 => {
                        // Now fully detached
                        // Redirect stdio
                        let devnull = std::fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open("/dev/null")?;
                        let fd = devnull.as_raw_fd();
                        libc::dup2(fd, libc::STDIN_FILENO);
                        libc::dup2(fd, libc::STDOUT_FILENO);
                        libc::dup2(fd, libc::STDERR_FILENO);

                        command.spawn().expect("Failed to spawn command");
                        std::process::exit(0);
                    }
                    _ => std::process::exit(0),
                }
            }
            _ => Ok(()),
        }
    }
}

pub fn split_as_command(cmd: &str) -> Vec<String> {
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
