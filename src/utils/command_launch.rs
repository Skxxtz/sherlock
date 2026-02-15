use std::{
    io::Write,
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::LazyLock,
};

use gpui::SharedString;
use regex::{Captures, Regex};

use crate::{
    sherlock_error,
    utils::{
        config::{ConfigGuard, SherlockConfig},
        errors::{SherlockError, SherlockErrorType},
    },
};

/// Spawnes a command completely detatched from the current process.
///
/// This function uses a "double-fork" strategy to ensure that the spawned process is adopted by
/// the system init process (PID 1). This prevents empty "zombie" process from cluttering the
/// process table and ensures the child survives even if the daemon exits.
///
/// # Safety
/// This function uses `unsafe` and `pre_exec`. `pre_exec` runs in a restricted environment between
/// `fork` and `exec`. It is generally safe here as it only performs a single syscall and exit, but
/// complex logic (like memory allocation or locking) shuold be avoided inside the `pre_exec`
/// block!
///
/// # Arguments
/// * `cmd` -  A string containing the program name followed by its arguments (e.g, `foot -e`).
pub fn spawn_detached(
    cmd: &str,
    keyword: &str,
    variables: &[(SharedString, SharedString)],
) -> Result<(), SherlockError> {
    let config = ConfigGuard::read().unwrap();
    let cmd = parse_variables(cmd, keyword, variables, &config);

    drop(config);

    let mut parts = split_as_command(&cmd);
    if parts.is_empty() {
        return Ok(());
    }

    // if sudo, insert -S flag to read sudo from stdin
    let mut sudo_used = false;
    if let Some(idx) = parts.iter().position(|p| *p == "sudo") {
        parts.insert(idx + 1, "-S".into());
        sudo_used = true;
    }

    let program = &parts[0];
    let args = &parts[1..];

    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if sudo_used {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    unsafe {
        command.pre_exec(|| {
            // Fork again inside the child
            match libc::fork() {
                -1 => return Err(std::io::Error::last_os_error()),
                0 => {
                    // detatch grandchild
                    libc::setsid();
                    Ok(())
                }
                _ => {
                    // exit child immediately
                    // this orphans the grandchild, will get adopted by PID 1.
                    libc::_exit(0);
                }
            }
        });
    }

    let mut child = command.spawn().map_err(|e| {
        sherlock_error!(
            SherlockErrorType::CommandExecutionError(cmd.to_string()),
            e.to_string()
        )
    })?;

    // pass sudo password
    if sudo_used {
        if let Some((_, password)) = variables
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("sudo"))
        {
            send_sudo(&mut child, password.as_str()).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::CommandExecutionError(cmd.into()),
                    e.to_string()
                )
            })?;
        }
    }

    let _ = child.wait();

    Ok(())
}

pub fn split_as_command(cmd: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut double_quoting = false;
    let mut single_quoting = false;
    let mut escaped = false;

    let mut it = cmd.chars().peekable();

    while let Some(c) = it.next() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' if !single_quoting => {
                escaped = true;
            }
            '"' if !single_quoting => {
                double_quoting = !double_quoting;
            }
            '\'' if !double_quoting => {
                single_quoting = !single_quoting;
            }
            c if c.is_whitespace() && !double_quoting && !single_quoting => {
                if !current.is_empty() {
                    parts.push(current.split_off(0));
                }
            }
            c => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts.retain(|s| !s.starts_with('%'));
    parts
}

/// Regex pattern used for parsing variables from a command string
///
/// # Groups
/// Group 1 & 2: `{key:value}`
/// Group 3 & 4: `{prefix[key]:value}`
static VAR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\{([a-zA-Z_]+)(?::(.*?))?\}|\{prefix\[(.*?)\]:(.*?)\}"#).unwrap()
});

/// Replaces variables like `{variable:variable name}` and applies prefixes such as `{prefix[variable name]:string}` with
/// the corresponding values.
///
/// This function is designed to efficiently find and replace occurences of variables and prefixes
/// using regex while folling the `Sherlock variable input scheme`.
pub fn parse_variables(
    exec_input: &str,
    keyword: &str,
    variables: &[(SharedString, SharedString)],
    config: &SherlockConfig,
) -> String {
    VAR_REGEX
        .replace_all(exec_input, |caps: &Captures| {
            // Handle prefixes `prefix[for]:text`
            if let (Some(prefix_for), Some(prefix_text)) = (caps.get(3), caps.get(4)) {
                let key = prefix_for.as_str();
                let has_value = variables
                    .iter()
                    .any(|(k, v)| k.as_ref() == key && !v.is_empty());

                return if has_value {
                    prefix_text.as_str().to_string()
                } else {
                    String::new()
                };
            }

            // Handle vars `{terminal, keyword, or variable:text}`
            let key = &caps[1];
            match key {
                "terminal" => format!("{} -e", config.default_apps.terminal),
                "keyword" => keyword.to_string(),
                "variable" => {
                    let var_name = caps.get(2).map(|m| m.as_str());
                    variables
                        .iter()
                        .find(|(k, _)| Some(k.as_ref()) == var_name)
                        .map(|v| v.1.to_string())
                        .unwrap_or_else(|| caps[0].to_string())
                }
                _ => caps[0].to_string(),
            }
        })
        .into_owned()
}

/// Send the sudo password to a child process's stdin.
///
/// This function is designed for commands prefixed with `sudo -S`. It writes the provided password
/// followed by a newlines and flushes the buffer to ensure the child process reveices the
/// credentials immediately.
///
/// # Errors
/// * The child process doesn't have a piped stdin (e.g., was not spawned with `Stdio::piped()`)
/// * The pipe is broken or the write operation failes
fn send_sudo(child: &mut Child, sudo: &str) -> Result<(), std::io::Error> {
    let mut stdin = child.stdin.take().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Stdin pipe not available.")
    })?;

    {
        stdin.write_all(sudo.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
    }

    Ok(())
}
