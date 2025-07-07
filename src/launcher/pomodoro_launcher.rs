use std::{path::PathBuf, str::FromStr};

#[derive(Clone, Debug)]
pub struct Pomodoro {
    pub program: PathBuf,
    pub socket: PathBuf,
    pub style: PomodoroStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PomodoroStyle {
    Minimal,
    Normal,
}
impl FromStr for PomodoroStyle {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "minimal" => Ok(Self::Minimal),
            _ => Ok(Self::Normal),
        }
    }
}
