use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::CONFIG;

pub fn command_launch(exec: &str, keyword: &str) -> Result<(), SherlockError> {
    let config = CONFIG.get().ok_or(SherlockError {
        error: SherlockErrorType::ConfigError(None),
        traceback: String::new(),
    })?;
    let prefix = config
        .behavior
        .global_prefix
        .as_ref()
        .map_or(String::new(), |p| format!("{} ", p));
    let flags = config
        .behavior
        .global_flags
        .as_ref()
        .map_or(String::new(), |f| format!(" {}", f));

    let exec = exec.replace("{keyword}", &keyword);
    let commands = exec.split("&").map(|s| s.trim()).filter(|s| !s.is_empty());

    for command in commands {
        asynchronous_execution(command, &prefix, &flags)?;
    }
    Ok(())
}

fn asynchronous_execution(cmd: &str, prefix: &str, flags: &str) -> Result<(), SherlockError> {
    let raw_command = format!("{}{}{}", prefix, cmd, flags);
    let mut parts = raw_command
        .split_whitespace()
        .filter(|s| !s.starts_with("%"));

    let mut command = Command::new(parts.next().ok_or_else(|| SherlockError {
        error: SherlockErrorType::CommandExecutionError(String::from(
            "The command list was empty.",
        )),
        traceback: String::from("Location: src/commandlaunch.rs"),
    })?);
    command.args(parts);

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

    command.spawn().map_err(|e| SherlockError {
        error: SherlockErrorType::CommandExecutionError(cmd.to_string()),
        traceback: e.to_string(),
    })?;

    Ok(())
}
