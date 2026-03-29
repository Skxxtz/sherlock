use std::sync::Arc;

use gpui::App;

use crate::{
    launcher::{
        Launcher,
        children::RenderableChild,
        emoji_launcher::{EmojiData, data::EMOJIS},
    },
    ui::model::Model,
};

pub struct EmojiView {
    pub model: Model,
}

impl EmojiView {
    pub fn new(launcher: Arc<Launcher>, cx: &mut App) -> Self {
        let data: Vec<RenderableChild> = EMOJIS
            .into_iter()
            .map(|entry| RenderableChild::EmojiLike {
                launcher: launcher.clone(),
                inner: EmojiData { entry },
            })
            .collect();

        Self {
            model: Model::standard(data, cx),
        }
    }
}
