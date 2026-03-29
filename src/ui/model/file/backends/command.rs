use std::{borrow::Cow, ffi::OsStr, io::BufRead, path::PathBuf, process::Child};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::ui::model::file::{
    FileResult, FileSearchUtility, ResultHeap, backends::FileSearchProvider,
};

pub struct ChildGuard(pub Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub trait CommandFactory: Send + Sync {
    fn binary_name(&self) -> &'static str;
    fn args<'a>(&self, paths: &'a [PathBuf]) -> impl Iterator<Item = Cow<'a, OsStr>>;
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CommandBackend<F: CommandFactory> {
    factory: F,
}
impl<F: CommandFactory> CommandBackend<F> {
    pub fn new(factory: F) -> Self {
        Self { factory }
    }
}

impl<F: CommandFactory + Default> FileSearchProvider for CommandBackend<F> {
    fn name(&self) -> &'static str {
        self.factory.binary_name()
    }

    fn search(
        &self,
        query: String,
        paths: Vec<PathBuf>,
        heap: &mut ResultHeap,
        mut cancel_rx: mpsc::Receiver<()>,
        tx: &mpsc::Sender<Vec<FileResult>>,
    ) -> bool {
        let child = match std::process::Command::new(self.factory.binary_name())
            .args(self.factory.args(&paths))
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
                // Normalize separators on Windows only — avoid alloc on Unix
                let path_bytes = line.as_bytes();
                if !FileSearchUtility::bytes_contain_ci(path_bytes, query.as_bytes()) {
                    continue;
                }
                FileSearchUtility::score_path(path_bytes, query.as_bytes())
            } else {
                let name_bytes = line.as_bytes();
                // Slice to filename only — don't score against the full path
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

        true
    }
}
