use crate::launcher::Launcher;
use crate::launcher::children::RenderableChild;
use crate::launcher::children::file_data::FileData;
use crate::ui::launcher::LauncherView;
use crate::ui::model::file::backends::FileSearchBackend;
use crate::ui::model::file::backends::command::CommandBackend;
use gpui::{App, Task, WeakEntity};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

mod backends;

#[derive(Clone)]
pub struct FileResult {
    pub path: Arc<str>,
    pub score: f32,
}
impl FileResult {
    pub fn extension(&self) -> &str {
        self.path.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("")
    }

    pub fn get_icon_name(&self) -> &'static str {
        if self.path.ends_with('/') {
            return "folder";
        }

        let filename = self
            .path
            .rsplit_once(['/', '\\'])
            .map(|(_, n)| n)
            .unwrap_or(&self.path);

        let filename_lower = filename.to_lowercase();

        // 1. High-level System Files
        match filename_lower.as_str() {
            "license" => return "license",
            "vmlinuz" | "zimage" => return "system-run", // Kernel
            "fstab" | "mtab" => return "drive-harddisk",
            "passwd" | "shadow" => return "password-manager",
            "bashrc" | "zshrc" | "profile" => return "utilities-terminal",
            _ => {}
        }

        // 2. Standard Mime-Type Style Icons
        match self.extension() {
            // Binaries & Execution
            "so" | "a" | "o" => "lib",
            "bin" | "elf" => "application-x-executable",
            "sh" | "bash" | "py" | "pl" => "application-x-executable-script",

            // Packages
            "deb" | "rpm" | "pkg" => "package-x-generic",
            "tar" | "gz" | "xz" | "zip" | "7z" => "package-x-generic",

            // Configuration & Text
            "conf" | "cfg" | "ini" | "yaml" | "toml" => "preferences-system",
            "json" | "xml" => "text-x-script",
            "log" => "text-x-generic",
            "txt" => "text-x-generic",
            "md" | "markdown" => "text-x-preview",

            // Security & Keys
            "pem" | "crt" | "key" | "gpg" | "pub" => "security-high",

            // Programming (Standard fallback names)
            "rs" | "c" | "cpp" | "java" | "go" => "text-x-source",

            // Media
            "png" | "jpg" | "jpeg" | "svg" => "image-x-generic",
            "mp4" | "mkv" | "avi" => "video-x-generic",
            "mp3" | "ogg" | "wav" => "audio-x-generic",
            "pdf" => "document-print",

            // Desktop Entries
            "desktop" => "application-x-desktop",

            _ => "text-x-generic",
        }
    }
}

/// A min-heap slot: we keep a fixed-size sorted array.
/// Scores are inverted so the *worst* result is at index 0 for fast eviction.
struct ResultHeap {
    buf: Vec<FileResult>,
    capacity: usize,
}

impl ResultHeap {
    fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Returns true if the result was inserted.
    #[inline]
    fn push(&mut self, result: FileResult) -> bool {
        if self.buf.len() < self.capacity {
            self.buf.push(result);
            // Keep sorted descending (best first = lowest score first in our scheme)
            self.buf
                .sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            return true;
        }
        // Only evict the worst (last, highest score) if new result is better
        if let Some(worst) = self.buf.last() {
            if result.score < worst.score {
                *self.buf.last_mut().unwrap() = result;
                self.buf
                    .sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
                return true;
            }
        }
        false
    }

    fn snapshot(&self) -> Vec<FileResult> {
        self.buf.clone()
    }
}

pub struct FileSearchModel {
    backend: FileSearchBackend,
    launcher: Arc<Launcher>,
    pub results: Vec<FileResult>,
    cancel_tx: Option<mpsc::Sender<()>>,
    _poll_task: Option<Task<()>>,
}

pub(super) const MAX_RESULTS: usize = 50;
pub(super) const POLL_INTERVAL_MS: u64 = 50;
pub(super) const MAX_SEARCH_DEPTH: usize = 6;

impl FileSearchModel {
    pub fn new(launcher: Arc<Launcher>) -> Self {
        Self {
            backend: FileSearchBackend::Fd(CommandBackend::new(backends::fd::FdFactory {})),
            launcher,
            results: Vec::with_capacity(MAX_RESULTS),
            cancel_tx: None,
            _poll_task: None,
        }
    }

    pub fn search(
        &mut self,
        query: String,
        search_paths: Vec<PathBuf>,
        result_entity: WeakEntity<Arc<Vec<RenderableChild>>>,
        launcher_weak: WeakEntity<LauncherView>,
        cx: &mut App,
    ) {
        // Dropping to cacel running tasks
        self.cancel_tx = None;
        self._poll_task = None;
        self.results.clear();

        if query.is_empty() {
            return;
        }

        let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
        self.cancel_tx = Some(cancel_tx);

        let (result_tx, mut result_rx) = mpsc::channel::<Vec<FileResult>>(32);

        let backend = self.backend.clone();
        std::thread::spawn(move || {
            let query_lower = query.to_lowercase();
            let mut heap = ResultHeap::new(MAX_RESULTS);
            let completed =
                backend.search(query_lower, search_paths, &mut heap, cancel_rx, &result_tx);
            if completed {
                let _ = result_tx.try_send(heap.snapshot());
            }
            // result_tx drops here → result_rx.recv() returns None → poll task exits
        });

        let launcher = Arc::clone(&self.launcher);
        let poll_task = cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(POLL_INTERVAL_MS))
                    .await;

                let mut latest: Option<Vec<FileResult>> = None;
                let mut channel_open = true;

                loop {
                    match result_rx.try_recv() {
                        Ok(snap) => {
                            latest = Some(snap);
                        }
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        // Sender dropped — thread has exited (completed or cancelled)
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            channel_open = false;
                            break;
                        }
                    }
                }

                if let Some(snapshot) = latest {
                    let count = snapshot.len();
                    let children = Arc::new(
                        snapshot
                            .into_iter()
                            .map(|r| RenderableChild::FileLike {
                                launcher: Arc::clone(&launcher),
                                inner: FileData::new(r.path.clone())
                                    .with_icon_name(r.get_icon_name()),
                            })
                            .collect::<Vec<_>>(),
                    );

                    let indices: Arc<[usize]> = (0..count).collect::<Vec<_>>().into();

                    if let Some(view) = launcher_weak.upgrade() {
                        let _ = cx.update(|cx| {
                            view.update(cx, |this, cx| {
                                // Swap the entity data directly
                                if let Some(entity) = result_entity.upgrade() {
                                    entity.update(cx, |e, _| *e = children);
                                }
                                // Reuse the exact same apply_results path as regular search
                                this.apply_results(indices, String::new(), cx);
                            });
                        });
                    }
                }

                if !channel_open {
                    break;
                }
            }
        });

        self._poll_task = Some(poll_task);
    }
}

pub(super) struct FileSearchUtility;
impl FileSearchUtility {
    /// Returns true if `entry` is a hidden file or inside a hidden directory.
    #[inline]
    fn is_hidden(entry: &walkdir::DirEntry) -> bool {
        entry.file_name().as_encoded_bytes().first().copied() == Some(b'.')
    }

    #[inline]
    fn bytes_eq_ci(a: &[u8], b: &[u8]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_ascii_lowercase() == *y)
    }

    /// Case-insensitive substring search over raw bytes — no allocation.
    /// Only handles ASCII correctly!!
    #[inline]
    fn bytes_contain_ci(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() {
            return true;
        }
        if needle.len() > haystack.len() {
            return false;
        }
        haystack.windows(needle.len()).any(|w| {
            w.iter()
                .zip(needle.iter())
                .all(|(h, n)| h.to_ascii_lowercase() == *n)
        })
    }

    #[inline]
    fn memrchr_slash(bytes: &[u8]) -> Option<usize> {
        bytes.iter().rposition(|&b| b == b'/')
    }

    // Comparing with already lower-cased query,
    // using a zero-alloc case-insensitive comparator
    #[inline]
    fn score_file_ci(name_bytes: &[u8], query: &str) -> f32 {
        let q = query.as_bytes();
        let len = name_bytes.len();
        let qlen = q.len();

        let eq = len == qlen && Self::bytes_eq_ci(name_bytes, q);
        let ends = len > qlen && Self::bytes_eq_ci(&name_bytes[len - qlen..], q);
        let starts = len > qlen && Self::bytes_eq_ci(&name_bytes[..qlen], q);

        if eq {
            return 0.0;
        }
        if ends {
            return 0.05;
        }
        if starts {
            return 0.1 + 0.1 * (1.0 - qlen as f32 / len as f32);
        }
        if Self::bytes_contain_ci(name_bytes, q) {
            return 0.4;
        }
        0.8
    }

    #[inline]
    fn score_path(path_bytes: &[u8], query: &[u8]) -> f32 {
        let plen = path_bytes.len();
        let qlen = query.len();

        if Self::bytes_eq_ci(path_bytes, query) {
            return 0.0;
        }

        if plen > qlen {
            let tail = &path_bytes[plen - qlen..];
            if Self::bytes_eq_ci(tail, query) {
                let prev = path_bytes[plen - qlen - 1];
                if prev == b'/' || prev == b'\\' {
                    return 0.05;
                }
            }
        }
        if Self::bytes_contain_ci(path_bytes, query) {
            return 0.3 + 0.1 * (1.0 - qlen as f32 / plen as f32);
        }
        0.8
    }
}
