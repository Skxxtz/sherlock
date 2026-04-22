use std::{
    env,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use crate::{sherlock_msg, utils::errors::types::SherlockErrorType};

use super::errors::SherlockMessage;

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
pub fn expand_path<T: AsRef<Path>>(path: T, home: &Path) -> PathBuf {
    let path = path.as_ref();
    let mut components = path.components();
    if let Some(std::path::Component::Normal(first)) = components.next()
        && first == "~"
    {
        return home.join(components.as_path());
    }
    path.to_path_buf()
}
pub fn home_dir() -> Result<PathBuf, SherlockMessage> {
    env::var("HOME")
        .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::EnvError("$HOME".into()), e))
        .map(PathBuf::from)
}
