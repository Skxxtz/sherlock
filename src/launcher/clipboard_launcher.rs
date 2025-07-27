use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct ClipboardLauncher {
    pub clipboard_content: String,
    pub capabilities: HashSet<String>,
}
