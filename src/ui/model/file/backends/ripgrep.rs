use crate::ui::model::file::{
    FileResult, FileSearchUtility, ResultHeap, backends::FileSearchProvider,
};
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::PathBuf;
use std::process::Child;
use tokio::sync::mpsc;

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RipgrepBackend;

impl FileSearchProvider for RipgrepBackend {
    fn name(&self) -> &'static str {
        "ripgrep"
    }

    fn search(
        &self,
        query: String,
        paths: Vec<PathBuf>,
        heap: &mut ResultHeap,
        mut cancel_rx: mpsc::Receiver<()>,
        tx: &mpsc::Sender<Vec<FileResult>>,
    ) -> bool {
        let child = match std::process::Command::new("rg")
            .args(&paths)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let mut guard = ChildGuard(child);

        let stdout = match guard.0.stdout.take() {
            Some(s) => s,
            None => return false,
        };

        let is_path_query = query.contains('/') || query.contains(std::path::MAIN_SEPARATOR);

        let mut files_since_send: usize = 0;
        const BATCH: usize = 16;

        for line in std::io::BufReader::new(stdout).lines() {
            if cancel_rx.try_recv().is_ok() || cancel_rx.is_closed() {
                return false;
            }

            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            let score = if is_path_query {
                let full_lower = line.to_lowercase();
                if !full_lower.contains(&query) {
                    continue;
                }
                FileSearchUtility::score_path(&full_lower, &query)
            } else {
                let name_bytes = line.as_bytes();
                let name_bytes = FileSearchUtility::memrchr_slash(name_bytes)
                    .map(|i| &name_bytes[i + 1..])
                    .unwrap_or(name_bytes);
                if !FileSearchUtility::bytes_contain_ci(name_bytes, query.as_bytes()) {
                    continue;
                }
                FileSearchUtility::score_file_ci(name_bytes, &query)
            };

            if heap.push(FileResult {
                path: line.as_str().into(),
                score,
            }) {
                files_since_send += 1;
                if files_since_send >= BATCH {
                    let _ = tx.try_send(heap.snapshot());
                    files_since_send = 0;
                }
            }
        }

        // Note to self: ChildGuard drops here — kill is a no-op on an already-exited process,
        // wait() reaps it cleanly either way.
        true
    }
}
