use std::{
    env,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use crate::sherlock_error;

use super::errors::{SherlockError, SherlockErrorType};

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
pub fn expand_path(path: &Path, home: &Path) -> PathBuf {
    let mut components = path.components();
    if let Some(std::path::Component::Normal(first)) = components.next() {
        if first == "~" {
            return home.join(components.as_path());
        }
    }
    path.to_path_buf()
}
pub fn home_dir() -> Result<PathBuf, SherlockError> {
    env::var("HOME")
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::EnvVarNotFoundError(String::from("HOME")),
                e.to_string()
            )
        })
        .map(PathBuf::from)
}
