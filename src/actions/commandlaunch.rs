use std::process::{Command, Stdio};

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
    let commands = exec.split("&").map(|s| s.trim()).filter(|s| !s.is_empty());

    for command in commands {
        asynchronous_execution(command, &prefix, &flags)?;
    }
    Ok(())
}

pub fn asynchronous_execution(cmd: &str, prefix: &str, flags: &str) -> Result<(), SherlockError> {
    let raw_command = format!("{}{}{}", prefix, cmd, flags).replace(r#"\""#, "'");
    sher_log!(format!(r#"Spawning command "{}""#, raw_command))?;

    let mut command = Command::new("sh");

    command
        .arg("-c")
        .arg(raw_command.clone())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    match command.spawn() {
        Ok(mut _child) => {
            let _ = sher_log!(format!("Detached process started: {}.", raw_command));
            // if let Some(err) = child.stderr.take() {
            // sher_log!(format!(
            //     r#"Detached process {} erred: {:?}"#,
            //     raw_command, err
            // ));
            // }
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
