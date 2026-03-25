use std::fmt::Display;

use serde::{Deserialize, Serialize};
use strum::FromRepr;

use crate::launcher::emoji_launcher::data::EmojiEntry;

pub mod data;

pub static ALL_SKIN_TONES: [SkinTone; 6] = [
    SkinTone::Simpsons,
    SkinTone::Light,
    SkinTone::MediumLight,
    SkinTone::Medium,
    SkinTone::MediumDark,
    SkinTone::Dark,
];

#[derive(Clone, Debug, Default)]
pub struct EmojiPicker {}

#[derive(Clone, Debug)]
pub struct EmojiData {
    pub entry: &'static EmojiEntry,
}

#[derive(Copy, Clone, Debug, FromRepr, Default, Deserialize, Serialize, PartialEq)]
#[repr(u8)] // This tells Rust to treat the enum like a u8 in memory
pub enum SkinTone {
    #[default]
    Simpsons = 0,
    Light = 1,
    MediumLight = 2,
    Medium = 3,
    MediumDark = 4,
    Dark = 5,
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
impl From<u8> for SkinTone {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Light,
            2 => Self::MediumLight,
            3 => Self::Medium,
            4 => Self::MediumDark,
            5 => Self::Dark,
            _ => Self::Simpsons,
        }
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
