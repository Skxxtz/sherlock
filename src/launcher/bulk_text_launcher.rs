use serde::{Deserialize, Serialize};
use std::env::home_dir;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

use crate::loader::util::ApplicationAction;

#[derive(Clone, Debug)]
pub struct BulkTextLauncher {
    pub icon: String,
    pub exec: String,
    pub args: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AsyncCommandResponse {
    pub title: Option<String>,
    pub content: Option<String>,
    pub next_content: Option<String>,
    pub result: Option<String>,
    pub actions: Option<Vec<ApplicationAction>>,
}
impl AsyncCommandResponse {
    fn new() -> Self {
        AsyncCommandResponse {
            title: None,
            content: None,
            next_content: None,
            actions: None,
            result: None,
        }
    }
}

impl BulkTextLauncher {
    pub async fn get_result(&self, keyword: &str) -> Option<AsyncCommandResponse> {
        if self.args.contains("{keyword}") && keyword.trim().is_empty() {
            return None;
        };

        let a = self.args.replace("{keyword}", keyword);
        let args = a.split(" ");

        // build execution command
        let home = home_dir()?;
        let exec = self.exec.clone();
        let relative_exec = exec.strip_prefix("~/").unwrap_or(&exec);
        let absolute_exec = home.join(relative_exec);
        let exec_str = absolute_exec.to_str()?;

        let mut cmd = Command::new(exec_str);

        cmd.args(args);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn the child process
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let mut response = AsyncCommandResponse::new();
                response.title = Some(String::from("Failed to execute script."));
                response.content = Some(format!("Error: {}", e));
                return Some(response);
            }
        };

        // Set up a timeout for the process
        let result = timeout(Duration::from_secs(2), async {
            let stdout = child.stdout.take();
            let mut stdout_content = String::new();
            let stderr_content = String::new();

            // If stdout is present, read asynchronously
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout);
                reader.read_to_string(&mut stdout_content).await.ok();
            }

            // Wait for the child process to finish
            let status = child.wait().await;
            (status, stdout_content, stderr_content)
        })
        .await;

        // Handle the result
        match result {
            Ok((Ok(status), stdout, _stderr)) => {
                if status.success() {
                    let mut output = stdout.into_bytes();
                    let response: AsyncCommandResponse =
                        simd_json::from_slice(&mut output).unwrap_or(AsyncCommandResponse::new());
                    Some(response)
                } else {
                    let mut response = AsyncCommandResponse::new();
                    response.title = Some(String::from("Script returned an error."));
                    response.content = Some(format!("Status: {:?}", status));
                    Some(response)
                }
            }
            Ok((Err(_), _, _)) => {
                let _ = child.kill().await; // Kill the process if it fails
                let mut response = AsyncCommandResponse::new();
                response.title = Some(String::from("Failed to execute script."));
                response.content = Some(String::from(
                    "Error occurred while running the process: {:?}",
                ));
                Some(response)
            }
            Err(_) => {
                let _ = child.kill().await; // Kill the process on timeout
                let mut response = AsyncCommandResponse::new();
                response.title = Some(String::from("Failed to execute script."));
                response.content = Some(String::from("Timeout exceeded."));
                Some(response)
            }
        }
    }
}
