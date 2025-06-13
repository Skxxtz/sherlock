use std::process::{Command, Stdio};

use crate::{sher_log, CONFIG};
use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};
pub fn command_launch(exec: &str, keyword: &str) -> Result<(), SherlockError> {
    let config = CONFIG
        .get()
        .ok_or(sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;
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
    command.arg("-c").arg(raw_command.clone());

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Move command string into task
    let child = command.spawn().map_err(|e| {
        let _ = sher_log!(format!(
            "Failed to spawn command: {}\nError: {}",
            raw_command, e
        ));
        sherlock_error!(
            SherlockErrorType::CommandExecutionError(cmd.to_string()),
            e.to_string()
        )
    })?;

    tokio::spawn(async move {
        let result = match child.wait_with_output() {
            Ok(output) => {
                if output.status.success() {
                    let _ = sher_log!(format!("Command succeeded: {}", raw_command));
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let _ = sher_log!(format!(
                        "Command failed: {}\nStderr: {}",
                        raw_command, stderr
                    ));
                    Err(sherlock_error!(
                        SherlockErrorType::CommandExecutionError(raw_command.to_string()),
                        stderr.to_string()
                    ))
                }
            }
            Err(e) => {
                let _ = sher_log!(format!(
                    "Failed to wait for command: {}\nError: {}",
                    raw_command, e
                ));
                Err(sherlock_error!(
                    SherlockErrorType::CommandExecutionError(raw_command.to_string()),
                    e.to_string()
                ))
            }
        };
        if let Err(err) = result {
            let _result = err.insert(false);
        }
    });
    Ok(())
}
