use std::process::Command;
use std::{collections::HashMap, os::fd::AsRawFd};

use regex::Regex;

use crate::{
    sher_log, sherlock_error,
    utils::{
        config::ConfigGuard,
        errors::{SherlockError, SherlockErrorType},
    },
};

pub fn applaunch(
    exec: &str,
    terminal: bool,
    variables: HashMap<String, String>,
) -> Result<(), SherlockError> {
    let config = ConfigGuard::read()?;
    let mut parts = Vec::new();
    let mut exec = exec.to_string();

    // Insert prefixes
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

    // Insert variables
    for (k, v) in variables {
        exec = exec.replace(&format!("{{variable:{}}}", k), &v);
    }

    if let Some(pre) = &config.behavior.global_prefix {
        parts.push(pre.to_string());
    }
    if terminal {
        parts.push(config.default_apps.terminal.clone());
        parts.push("-e".to_string());
    }
    parts.push(exec.to_string());
    if let Some(flag) = &config.behavior.global_flags {
        parts.push(flag.to_string());
    }

    let cmd = parts.join(" ").trim().to_string();
    let mut parts = split_as_command(&cmd).into_iter();

    let mut command = Command::new(parts.next().ok_or(sherlock_error!(
        SherlockErrorType::CommandExecutionError(cmd.clone()),
        format!("Failed to get first base command")
    ))?);
    command.args(parts);

    match launch_detached(command) {
        Ok(_) => {
            let _ = sher_log!(format!("Detached process started: {}.", cmd));
            Ok(())
        }
        Err(e) => {
            let _ = sher_log!(format!("Failed to detach command: {}\nError: {}", cmd, e));

            Err(sherlock_error!(
                SherlockErrorType::CommandExecutionError(cmd),
                e.to_string()
            ))
        }
    }
}

pub fn launch_detached(mut command: Command) -> std::io::Result<()> {
    unsafe {
        match libc::fork() {
            -1 => return Err(std::io::Error::last_os_error()),
            0 => {
                // Child process
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                // Fork again to prevent from acquiring a controlling terminal
                match libc::fork() {
                    -1 => return Err(std::io::Error::last_os_error()),
                    0 => {
                        // Now fully detached
                        // Redirect stdio
                        let devnull = std::fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open("/dev/null")?;
                        let fd = devnull.as_raw_fd();
                        libc::dup2(fd, libc::STDIN_FILENO);
                        libc::dup2(fd, libc::STDOUT_FILENO);
                        libc::dup2(fd, libc::STDERR_FILENO);

                        command.spawn().expect("Failed to spawn command");
                        std::process::exit(0);
                    }
                    _ => std::process::exit(0),
                }
            }
            _ => Ok(()),
        }
    }
}

pub fn launch_detached_child(mut child: std::process::Child) -> Result<(), SherlockError> {
    unsafe {
        // Tells OS the process is not attatched to sherlock anymore
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }

    // Make sure stdin/out/err dont hand or capture output
    if let Some(stdin) = child.stdin.take() {
        let _ = nix::fcntl::fcntl(
            stdin.as_raw_fd(),
            nix::fcntl::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC),
        );
    }
    if let Some(stdin) = child.stdin.take() {
        let _ = nix::fcntl::fcntl(
            stdin.as_raw_fd(),
            nix::fcntl::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC),
        );
    }
    if let Some(stdin) = child.stdin.take() {
        let _ = nix::fcntl::fcntl(
            stdin.as_raw_fd(),
            nix::fcntl::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC),
        );
    }

    std::mem::drop(child);
    Ok(())
}

pub fn split_as_command(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();

    // State machine
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    let chars: Vec<char> = cmd.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];

        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        match c {
            // Handle backslash escaping
            '\\' => {
                current.push('\\');
                // Only mark as escaped if there's a character following it
                if i + 1 < chars.len() {
                    escaped = true;
                }
            }
            // Single quote toggle
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push('\'');
            }
            // Double quote toggle
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push('"');
            }
            // Whitespace split
            c if c.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            // Literal characters
            _ => {
                current.push(c);
            }
        }
    }

    // Push the final buffer
    if !current.is_empty() {
        args.push(current);
    }

    args.retain(|s| !s.starts_with('%'));

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_as_command() {
        assert_eq!(
            split_as_command(
                "flatpak run --command=bottles-cli com.usebottles.bottles run -p 'FL Studio 2025' -b 'Apps' -- %u"
            ),
            vec![
                "flatpak",
                "run",
                "--command=bottles-cli",
                "com.usebottles.bottles",
                "run",
                "-p",
                "'FL Studio 2025'",
                "-b",
                "'Apps'",
                "--"
            ]
        );
        // 1. Basic splitting
        assert_eq!(split_as_command("ls -la /home"), vec!["ls", "-la", "/home"]);

        // 2. Double quotes with spaces
        assert_eq!(
            split_as_command("echo \"hello world\""),
            vec!["echo", "\"hello world\""]
        );

        // 3. Single quotes
        assert_eq!(
            split_as_command("grep 'pattern with spaces' file.txt"),
            vec!["grep", "'pattern with spaces'", "file.txt"]
        );

        // 4. Nested quotes (ignored)
        assert_eq!(
            split_as_command("echo \"it's a trap\""),
            vec!["echo", "\"it's a trap\""]
        );

        // 5. Escaped characters inside double quotes
        assert_eq!(
            split_as_command("echo \"shout \\\"hello\\\"\""),
            vec!["echo", "\"shout \\\"hello\\\"\""]
        );

        // 6. Filtering variables starting with %
        assert_eq!(
            split_as_command("mpv %file --fullscreen"),
            vec!["mpv", "--fullscreen"]
        );

        // 7. Multiple spaces between arguments
        assert_eq!(
            split_as_command("rsync    -avz   source/   dest/"),
            vec!["rsync", "-avz", "source/", "dest/"]
        );

        // 8. Empty input
        let empty: Vec<String> = Vec::new();
        assert_eq!(split_as_command(""), empty);
        assert_eq!(split_as_command("   "), empty);
    }
}
