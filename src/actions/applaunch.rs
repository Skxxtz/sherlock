use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crate::utils::config::get_config;

pub fn applaunch(exec: &str, terminal: bool) -> Option<()> {
    let config = get_config().ok()?;
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

    let cmd = parts.join(" ").replace('"', "");

    let mut parts = cmd
        .trim()
        .split_whitespace()
        .filter(|s| !s.starts_with("%"));

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
