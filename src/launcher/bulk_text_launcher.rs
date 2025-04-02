use serde::{Deserialize, Serialize};
use serde_json;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Clone, Debug)]
pub struct BulkText {
    pub icon: String,
    pub exec: String,
    pub args: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AsyncCommandResponse {
    title: Option<String>,
    content: Option<String>,
    next_content: Option<String>,
}
impl AsyncCommandResponse {
    fn new() -> Self {
        AsyncCommandResponse {
            title: None,
            content: None,
            next_content: None,
        }
    }
}

impl BulkText {
    pub async fn get_result(&self, keyword: &str) -> Option<(String, String, Option<String>)> {
        if self.args.contains("{keyword}") && keyword.trim().is_empty() {
            return None;
        };

        let a = self.args.replace("{keyword}", &keyword);
        let args = a.split(" ");
        let mut cmd = Command::new(&self.exec);
        cmd.args(args);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn the child process
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return Some((
                    "Failed to execute script.".to_string(),
                    format!("Error: {}", e),
                    None,
                ));
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
                    let output = String::from_utf8_lossy(stdout.as_bytes());
                    let response: AsyncCommandResponse =
                        serde_json::from_str(&output).unwrap_or(AsyncCommandResponse::new());

                    let title = response.title.unwrap_or(keyword.to_string());
                    let content = response.content.unwrap_or_default();
                    Some((title, content, response.next_content))
                } else {
                    Some((
                        "Script executed but returned an error.".to_string(),
                        format!("Status: {:?}", status),
                        None,
                    ))
                }
            }
            Ok((Err(_), _, _)) => {
                let _ = child.kill().await; // Kill the process if it fails
                Some((
                    "Failed to execute script.".to_string(),
                    "Error occurred while running the process.".to_string(),
                    None,
                ))
            }
            Err(_) => {
                let _ = child.kill().await; // Kill the process on timeout
                Some((
                    "Failed to execute script.".to_string(),
                    "Timeout exceeded.".to_string(),
                    None,
                ))
            }
        }
    }
}
