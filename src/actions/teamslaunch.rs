use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crate::CONFIG;
use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};

pub fn teamslaunch(meeting_url: &str) -> Result<(), SherlockError> {
    if let Some(c) = CONFIG.get() {
        let teams_command = c.default_apps.teams.clone();
        let exec = teams_command.replace("{meeting_url}", meeting_url);

        let parts: Vec<String> = exec.split_whitespace().map(String::from).collect();

        if parts.is_empty() {
            return Err(sherlock_error!(
                SherlockErrorType::CommandExecutionError(String::from("Teams Start")),
                "Command is empty"
            ));
        }

        let mut command = Command::new(&parts[0]);
        for arg in &parts[1..] {
            if !arg.starts_with("%") {
                command.arg(arg);
            }
        }

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

        let _output = command.spawn().map_err(|e| {
            sherlock_error!(
                SherlockErrorType::CommandExecutionError(String::from("Teams Start")),
                e.to_string()
            )
        })?;

        Ok(())
    } else {
        Err(sherlock_error!(
            SherlockErrorType::ConfigError(None),
            "It should never get to this"
        ))
    }
}
