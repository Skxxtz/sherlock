use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Pomodoro {
    pub program: PathBuf,
    pub socket: PathBuf,
}

