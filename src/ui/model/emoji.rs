use std::sync::Arc;

use gpui::App;

use crate::{
    launcher::{
        Launcher,
        emoji_launcher::{EmojiData, data::EMOJIS},
    },
    ui::{model::Model, widgets::RenderableChild},
};

pub struct EmojiView {
    pub model: Model,
}

impl EmojiView {
    pub fn new(launcher: Arc<Launcher>, cx: &mut App) -> Self {
        let data: Vec<RenderableChild> = EMOJIS
            .iter()
            .map(|entry| RenderableChild::Emoji {
                launcher: launcher.clone(),
                inner: EmojiData { entry },
            })
            .collect();

        Self {
            model: Model::standard(data, cx),
        }
    }
}
