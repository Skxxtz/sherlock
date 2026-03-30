use std::{borrow::Cow, ffi::OsStr, io::BufRead, path::PathBuf, process::Child, sync::Arc};

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
    const HANDLES_FILTERING: bool;
    const BINARY_NAME: &'static str;
    fn args<'a>(
        &self,
        query: &'a str,
        paths: &'a [PathBuf],
    ) -> impl Iterator<Item = Cow<'a, OsStr>>;
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CommandBackend<F: CommandFactory> {
    factory: F,
}
impl<F: CommandFactory> CommandBackend<F> {
    fn handles_filtering(&self) -> bool {
        F::HANDLES_FILTERING
    }
}

impl<F: CommandFactory + Default> FileSearchProvider for CommandBackend<F> {
    fn name(&self) -> &'static str {
        F::BINARY_NAME
    }

    fn search(
        &self,
        query: Arc<str>,
        paths: Arc<Vec<PathBuf>>,
        heap: &mut ResultHeap,
        mut cancel_rx: mpsc::Receiver<()>,
        tx: &mpsc::Sender<Vec<FileResult>>,
    ) -> bool {
        let child = match std::process::Command::new(F::BINARY_NAME)
            .args(self.factory.args(query.as_ref(), &paths))
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

        let mut files_since_send: usize = 0;
        const BATCH: usize = 16;

        for (i, line) in std::io::BufReader::new(stdout).lines().enumerate() {
            if cancel_rx.try_recv().is_ok() || cancel_rx.is_closed() {
                return false;
            }

            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            let score = if !self.handles_filtering() {
                if line.ends_with('/') {
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
                }
            } else {
                i as f32
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
