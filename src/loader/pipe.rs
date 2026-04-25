use std::{
    fs::File,
    io::{self, Read},
    os::linux::fs::MetadataExt,
};

pub fn read_stdin_piped() -> Vec<u8> {
    if let Ok(metadata) = File::open("/dev/stdin").and_then(|f| f.metadata()) {
        // 0o020000 - Character device (e.g. TTY)
        // 0o170000 - octal mask to extract all file types
        if metadata.st_mode() & 0o170000 == 0o020000 {
            return vec![];
        }
    }
    let stdin = io::stdin();
    let mut buf = Vec::new();
    let _ = stdin.lock().read_to_end(&mut buf);
    buf
}
