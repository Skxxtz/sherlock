use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crate::CONFIG;

pub fn applaunch(exec: &str, terminal: bool) -> Option<()> {
    let config = CONFIG.get()?;
    let mut parts = Vec::new();

    if let Some(pre) = &config.behavior.global_prefix {
        parts.push(pre.to_string());
    }
    if terminal {
        parts.push(config.default_apps.terminal.clone());
    }
    parts.push(exec.to_string());
    if let Some(flag) = &config.behavior.global_flags {
        parts.push(flag.to_string());
    }

    let cmd = parts.join(" ").trim().to_string();
    let mut parts = split_as_command(&cmd).into_iter();
    let mut command = Command::new(parts.next()?);
    command.args(parts);

    #[cfg(target_family = "unix")]
    unsafe {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .pre_exec(|| {
                nix::unistd::setsid().ok();
                Ok(())
            });
    }
    let _ = command
        .spawn()
        .map_err(|e| eprintln!("Error executing command: {}", e));
    None
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
                _ => current.push('\\')
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
