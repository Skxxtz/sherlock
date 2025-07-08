use std::fmt::Display;

use crate::utils::errors::SherlockError;
use serde::{Deserialize, Serialize};

use super::api::SherlockModes;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ApiCall {
    // Settings
    InputOnly,
    Obfuscate(bool),
    // Actions
    Socket(Option<String>),
    Show(String),
    Clear,
    SherlockError(SherlockError),
    SherlockWarning(SherlockError),
    ClearAwaiting,
    Pipe(String),
    DisplayRaw(String),
    SwitchMode(SherlockModes),
}
impl Display for ApiCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Settings
            Self::InputOnly => write!(f, "setting.InputOnly"),
            Self::Obfuscate(val) => write!(f, "setting.Obfuscate:{}", val),
            // Actions
            Self::Show(submenu) => write!(f, "action.Show:{}", submenu),
            Self::Socket(socket) => write!(f, "action.Socket:{:?}", socket),
            Self::Clear => write!(f, "action.Clear"),
            Self::SherlockError(err) => write!(f, "action.InsertError:{}", err),
            Self::SherlockWarning(err) => write!(f, "action.InsertWarning:{}", err),
            Self::ClearAwaiting => write!(f, "action.ClearAwaiting"),
            Self::Pipe(pipe) => write!(f, "action.ProcessPipe:{}", pipe),
            Self::DisplayRaw(pipe) => write!(f, "action.DisplayRaw:{}", pipe),
            Self::SwitchMode(mode) => write!(f, "action.SwitchMode:{}", mode),
        }
    }
}
