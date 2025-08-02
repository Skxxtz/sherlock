use std::collections::HashSet;
use std::path::PathBuf;

use crate::loader::util::AppData;

#[derive(Clone, Debug)]
pub struct FileLauncher {
    pub dirs: HashSet<PathBuf>,
    pub data: Vec<AppData>,
    pub files: Option<Vec<FileData>>,
}

#[derive(Clone, Debug)]
pub struct FileData {
    pub name: String,
    pub loc: PathBuf,
}
