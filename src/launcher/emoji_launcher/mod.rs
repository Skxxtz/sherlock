use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::launcher::emoji_launcher::data::EmojiEntry;

pub mod data;

#[derive(Clone, Debug, Default)]
pub struct EmojiPicker {}

#[derive(Clone, Debug)]
pub struct EmojiData {
    pub entry: &'static EmojiEntry,
}

#[derive(Clone, Debug, Deserialize, Serialize, Copy, Default)]
pub enum SkinTone {
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
    #[default]
    Simpsons,
}
impl Display for SkinTone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Light => "\u{1F3FB}",
            Self::MediumLight => "\u{1F3FC}",
            Self::Medium => "\u{1F3FD}",
            Self::MediumDark => "\u{1F3FE}",
            Self::Dark => "\u{1F3FF}",
            Self::Simpsons => "",
        };
        f.write_str(s)
    }
}

impl SkinTone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "\u{1F3FB}",
            Self::MediumLight => "\u{1F3FC}",
            Self::Medium => "\u{1F3FD}",
            Self::MediumDark => "\u{1F3FE}",
            Self::Dark => "\u{1F3FF}",
            Self::Simpsons => "",
        }
    }
}
