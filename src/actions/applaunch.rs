use crate::loader::util::{SherlockError, SherlockErrorType};
use crate::CONFIG;
use std::{
    os::unix::process::CommandExt,
    process::{exit, Command, Stdio},
};

pub fn applaunch(exec: &str) -> Result<(), SherlockError> {
    let config = CONFIG.get().ok_or(SherlockError {
        error: SherlockErrorType::ConfigError(None),
        traceback: format!(""),
    })?;

    let parts: Vec<String> = match &config.behavior.launch_prefix {
        Some(prefix) => String::from(prefix) + " " + exec,
        None => String::from(exec),
    }
    .split_whitespace()
    .map(String::from)
    .collect();

    if parts.is_empty() {
        eprintln!("Error: Command is empty");
        exit(1);
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

    // TODO make error handling so that error tile will show up
    let _output = command.spawn();
    Ok(())
}
