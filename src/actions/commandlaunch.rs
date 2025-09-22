use regex::Regex;
use std::process::Command;

use crate::actions::applaunch::{launch_detached, split_as_command};
use crate::api::api::SherlockAPI;
use crate::sher_log;
use crate::utils::config::ConfigGuard;
use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};
pub fn command_launch(exec: &str, keyword: &str) -> Result<(), SherlockError> {
    println!("keyword: {:?}", keyword);
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
                println!("{:?}", exec);
                exec = exec.replace(full_match, &keyword);
                println!("{:?}", exec);
            }
            _ => {}
        }
    }

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
