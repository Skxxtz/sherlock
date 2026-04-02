use std::{fmt::Display, sync::Arc};

use gpui::SharedString;
use serde::{Deserialize, Serialize};
use strum::FromRepr;

use crate::{
    launcher::{Launcher, LauncherProvider, LauncherType, emoji_launcher::data::EmojiEntry},
    loader::{resolve_icon_path, utils::AppData},
    ui::widgets::{RenderableChild, emoji::set_selected_skin_tone},
    utils::errors::SherlockMessage,
};

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

impl LauncherProvider for EmojiPicker {
    fn parse(_raw: &crate::loader::utils::RawLauncher) -> super::LauncherType {
        LauncherType::Emoji(Self {})
    }

    fn objects(
        &self,
        launcher: Arc<Launcher>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let mut inner = AppData::new();
        inner.name = launcher.name.as_ref().map(SharedString::from);
        inner.search_string = "emoji".into();
        inner.icon = resolve_icon_path("sherlock-emoji");

        let default_skin_tone: SkinTone = opts
            .get("default_skin_tone")
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or(SkinTone::Simpsons);
        set_selected_skin_tone(default_skin_tone, 0);

        let child = RenderableChild::AppLike { launcher, inner };

        Ok(vec![child])
    }
}

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
