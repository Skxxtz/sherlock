use gpui::SharedString;
use serde::{Deserialize, Serialize};
use std::mem::discriminant;

use crate::utils::config::SherlockFlags;

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientMessage {
    ConfigUpdate(Box<SherlockFlags>),
    Dmenu(Vec<SharedString>),
    Open,
}

impl PartialEq for ClientMessage {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}
