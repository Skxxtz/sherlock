use std::sync::Arc;

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
pub struct ResultHeap {
    buf: Vec<FileResult>,
    capacity: usize,
}

impl ResultHeap {
    #[inline]
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Returns true if the result was inserted.
    #[inline]
    pub(super) fn push(&mut self, result: FileResult) -> bool {
        if self.buf.len() < self.capacity {
            self.buf.push(result);
            // Keep sorted descending (best first = lowest score first in our scheme)
            self.buf
                .sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            return true;
        }
        // Only evict the worst (last, highest score) if new result is better
        if let Some(worst) = self.buf.last()
            && result.score < worst.score
        {
            *self.buf.last_mut().unwrap() = result;
            self.buf
                .sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            return true;
        }

        false
    }

    pub(super) fn snapshot(&self) -> Vec<FileResult> {
        self.buf.clone()
    }
}
