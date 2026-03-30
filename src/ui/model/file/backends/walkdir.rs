use std::{path::PathBuf, sync::Arc};

use crate::ui::model::file::{
    FileSearchUtility, MAX_SEARCH_DEPTH,
    backends::FileSearchProvider,
    utils::{FileResult, ResultHeap},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WalkdirBackend;

impl FileSearchProvider for WalkdirBackend {
    fn name(&self) -> &'static str {
        "walkdir"
    }

    fn search(
        &self,
        query: Arc<str>,
        paths: Arc<Vec<PathBuf>>,
        heap: &mut ResultHeap,
        mut cancel_rx: mpsc::Receiver<()>,
        tx: &mpsc::Sender<Vec<FileResult>>,
    ) -> bool {
        let mut files_since_send: usize = 0;
        const BATCH: usize = 16;

        'outer: for path in paths.iter() {
            for entry in walkdir::WalkDir::new(&path)
                .follow_links(false)
                .max_depth(MAX_SEARCH_DEPTH)
                .into_iter()
                .filter_entry(|e| !FileSearchUtility::is_hidden(e))
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if cancel_rx.try_recv().is_ok() || cancel_rx.is_closed() {
                    break 'outer;
                }

                let os_name = entry.file_name();
                let score;
                let matched;

                if query.contains('/') || query.contains(std::path::MAIN_SEPARATOR) {
                    let path_bytes = entry.path().as_os_str().as_encoded_bytes();
                    if !FileSearchUtility::bytes_contain_ci(path_bytes, query.as_bytes()) {
                        continue;
                    }
                    score = FileSearchUtility::score_path(path_bytes, query.as_bytes());
                    matched = true;
                } else {
                    let name_bytes = os_name.as_encoded_bytes();
                    if !FileSearchUtility::bytes_contain_ci(name_bytes, query.as_bytes()) {
                        continue;
                    }
                    score = FileSearchUtility::score_file_ci(&name_bytes, &query);
                    matched = true;
                }

                if !matched {
                    continue;
                }

                if heap.push(FileResult {
                    path: entry.path().to_string_lossy().as_ref().into(),
                    score,
                }) {
                    files_since_send += 1;
                    if files_since_send >= BATCH {
                        let _ = tx.try_send(heap.snapshot());
                        files_since_send = 0;
                    }
                }
            }
        }
        true
    }
}
