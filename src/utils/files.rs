use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

use super::errors::{SherlockError, SherlockErrorType};

pub fn read_file(file_path: &str) -> std::io::Result<String> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    Ok(content)
}

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
        .map_err(|e| SherlockError {
            error: SherlockErrorType::EnvVarNotFoundError(String::from("HOME")),
            traceback: e.to_string(),
        })
        .map(PathBuf::from)
}
