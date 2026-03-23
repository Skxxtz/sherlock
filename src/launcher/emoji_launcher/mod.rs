use crate::launcher::emoji_launcher::data::EmojiEntry;

pub mod data;

#[derive(Clone, Debug, Default)]
pub struct EmojiPicker {}

#[derive(Clone, Debug)]
pub struct EmojiData {
    pub entry: &'static EmojiEntry,
}
