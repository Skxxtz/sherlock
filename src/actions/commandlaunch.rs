use std::process::Command;

use crate::actions::applaunch::{launch_detached, split_as_command};
use crate::sher_log;
use crate::utils::config::ConfigGuard;
use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};
pub fn command_launch(exec: &str, keyword: &str) -> Result<(), SherlockError> {
    let config = ConfigGuard::read()?;
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
    let commands = exec.split(" &").map(|s| s.trim()).filter(|s| !s.is_empty());

    for command in commands {
        asynchronous_execution(command, &prefix, &flags)?;
    }
    Ok(())
}

pub fn asynchronous_execution(cmd: &str, prefix: &str, flags: &str) -> Result<(), SherlockError> {
    let raw_command = format!("{}{}{}", prefix, cmd, flags).replace(r#"\""#, "'");
    sher_log!(format!(r#"Spawning command "{}""#, raw_command))?;

    let mut parts = split_as_command(&raw_command).into_iter();
    let mut command = Command::new(parts.next().ok_or(sherlock_error!(
        SherlockErrorType::CommandExecutionError(raw_command.clone()),
        format!("Failed to get first base command")
    ))?);
    command.args(parts);

    match launch_detached(command) {
        Ok(_) => {
            let _ = sher_log!(format!("Detached process started: {}.", raw_command));
            Ok(())
        }
        Err(e) => {
            let _ = sher_log!(format!(
                "Failed to detach command: {}\nError: {}",
                raw_command, e
            ));

            Err(sherlock_error!(
                SherlockErrorType::CommandExecutionError(cmd.to_string()),
                e.to_string()
            ))
        }
    }
}
