use regex::Regex;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::actions::applaunch::{launch_detached, launch_detached_child, split_as_command};
use crate::api::api::SherlockAPI;
use crate::sher_log;
use crate::utils::config::ConfigGuard;
use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};
pub fn command_launch(
    exec: &str,
    keyword: &str,
    variables: HashMap<String, String>,
) -> Result<(), SherlockError> {
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

    let mut exec = exec.to_string();

    let pattern = r#"\{([a-zA-Z_]+)(?::(.*?))?\}"#;
    let re = Regex::new(pattern).unwrap();
    for cap in re.captures_iter(&exec.clone()) {
        let full_match = &cap[0];
        let key = &cap[1];
        let value = cap.get(2).map(|m| m.as_str());

        match key {
            "terminal" => {
                exec = exec.replace(full_match, &format!("{} -e", config.default_apps.terminal));
            }
            "password" => {
                let password = gio::glib::MainContext::default()
                    .block_on(async { SherlockAPI::input_field(true, value).await })?;
                exec = exec.replace(full_match, &password);
            }
            "custom_text" => {
                let text = gio::glib::MainContext::default()
                    .block_on(async { SherlockAPI::input_field(false, value).await })?;
                exec = exec.replace(full_match, &text);
            }
            "keyword" => {
                exec = exec.replace(full_match, &keyword);
            }
            "variable" => {
                if let Some(val) = variables.get(value.unwrap()) {
                    exec = exec.replace(full_match, val)
                }
            }
            _ => {}
        }
    }

    let pattern = r#"\{prefix\[(.*?)\]:(.*?)\}"#;
    let re = Regex::new(pattern).unwrap();
    for cap in re.captures_iter(&exec.clone()) {
        let full_match = &cap[0];
        let prefix_for = &cap[1];
        let prefix = &cap[2];
        if !variables.get(prefix_for).map_or(true, |v| v.is_empty()) {
            exec = exec.replace(full_match, prefix);
        } else {
            exec = exec.replace(full_match, "");
        }
    }

    let commands = exec.split(" &").map(|s| s.trim()).filter(|s| !s.is_empty());

    for command in commands {
        let mut sudo = None;
        // Query sudo password if its not specified yet
        if command.contains("sudo ") {
            sudo = variables.get("sudo");
        }
        asynchronous_execution(command, &prefix, &flags, sudo)?;
    }
    Ok(())
}

pub fn asynchronous_execution(
    cmd: &str,
    prefix: &str,
    flags: &str,
    sudo_password: Option<&String>,
) -> Result<(), SherlockError> {
    let raw_command = format!("{}{}{}", prefix, cmd, flags).replace(r#"\""#, "'");

    sher_log!(format!(r#"Spawning command "{}""#, raw_command))?;

    let mut parts = split_as_command(&raw_command).into_iter();
    let base = parts.next().ok_or_else(|| {
        sherlock_error!(
            SherlockErrorType::CommandExecutionError(raw_command.clone()),
            "Failed to get first base command"
        )
    })?;

    // If sudo is requested, wrap the command
    let mut command = if let Some(ref _pw) = sudo_password {
        let mut c = Command::new("sudo");

        // -S makes sudo read password from stdin
        c.arg("-S");

        c.arg(base);
        c.args(parts);

        c.stdin(Stdio::piped());
        c
    } else {
        let mut c = Command::new(base);
        c.args(parts);
        c
    };

    // If sudo: write password into stdin *before* detaching
    if let Some(ref pw) = sudo_password {
        let mut child = command.spawn().map_err(|e| {
            sherlock_error!(
                SherlockErrorType::CommandExecutionError(raw_command.clone()),
                e.to_string()
            )
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            // write password securely
            stdin
                .write_all(format!("{}\n", pw).as_bytes())
                .map_err(|e| {
                    sherlock_error!(
                        SherlockErrorType::CommandExecutionError(raw_command.clone()),
                        e.to_string()
                    )
                })?;
        }

        launch_detached_child(child)?;

        let _ = sher_log!(format!("Detached sudo process for: {}.", raw_command));
        return Ok(());
    }

    // No sudo path
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
