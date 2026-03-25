use std::sync::{
    Arc, LazyLock, OnceLock, RwLock,
    atomic::{AtomicU8, Ordering},
};

use arrayvec::ArrayString;
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px, rgb};

use crate::{
    launcher::{
        ExecMode, Launcher,
        children::RenderableChildImpl,
        emoji_launcher::{EmojiData, SkinTone},
    },
    ui::launcher::context_menu::ContextMenuAction,
};

static SELECTED_SKIN_TONE: OnceLock<RwLock<[SkinTone; 2]>> = OnceLock::new();
static EMOJI_CONTEXT_ACTIONS: LazyLock<Arc<[Arc<ContextMenuAction>]>> = LazyLock::new(|| {
    Arc::from([
        Arc::new(ContextMenuAction::Emoji(EmojiAction {
            emoji: RwLock::new("😀"),
            for_tone: 0,
            selected_index: AtomicU8::new(0),
        })),
        Arc::new(ContextMenuAction::Emoji(EmojiAction {
            emoji: RwLock::new("👍"),
            for_tone: 1,
            selected_index: AtomicU8::new(0),
        })),
    ])
});

pub fn set_selected_skin_tone(tone: SkinTone, place: usize) {
    let lock = SELECTED_SKIN_TONE.get_or_init(|| RwLock::new([tone, tone]));

    if let Ok(mut w) = lock.write() {
        if place < w.len() {
            w[place] = tone;
        }
    }
}
pub fn get_selected_skin_tones() -> [SkinTone; 2] {
    let lock =
        SELECTED_SKIN_TONE.get_or_init(|| RwLock::new([SkinTone::Simpsons, SkinTone::Simpsons]));

    *lock.read().unwrap_or_else(|e| e.into_inner())
}

pub fn apply_skin_tones(template: &str, tones: &[SkinTone]) -> ArrayString<64> {
    let mut result = ArrayString::<64>::new();
    let mut tone_idx = 0;
    let mut remaining = template;
    while let Some(pos) = remaining.find("{skin_tone}") {
        let _ = result.try_push_str(&remaining[..pos]);
        if tone_idx < tones.len() {
            let _ = result.try_push_str(tones[tone_idx].as_str());
            tone_idx += 1;
        }
        remaining = &remaining[pos + "{skin_tone}".len()..];
    }
    let _ = result.try_push_str(remaining);
    result
}

impl<'a> RenderableChildImpl<'a> for EmojiData {
    fn render(&self, _launcher: &Arc<Launcher>, is_selected: bool) -> AnyElement {
        let emoji = apply_skin_tones(self.entry.emoji, &get_selected_skin_tones());
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(5.))
            .py(px(25.))
            .items_center()
            .justify_center()
            .rounded_md()
            .child(
                div()
                    .text_size(px(24.))
                    .line_height(px(24.))
                    .text_color(rgb(0xffffff)) // fallback for bad fonts
                    .child(emoji.as_str().to_string()),
            )
            .child(
                div()
                    .w_full()
                    .px(px(10.))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(px(10.))
                    .text_center()
                    .text_color(if is_selected {
                        rgb(0xffffff)
                    } else {
                        rgb(0xcccccc)
                    })
                    .child(self.entry.name),
            )
            .into_any_element()
    }
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        Some(ExecMode::Copy {
            content: self.entry.emoji.to_string(),
        })
    }
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.entry.keywords
    }
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let num_tones = self.entry.skin_tones as usize;
        let template = &*EMOJI_CONTEXT_ACTIONS;

        for action_arc in template.iter().take(num_tones) {
            if let ContextMenuAction::Emoji(act) = action_arc.as_ref() {
                let mut writer = act.emoji.write().unwrap();
                *writer = self.entry.emoji;
            }
        }

        let subset: Vec<Arc<ContextMenuAction>> =
            template.iter().take(num_tones).cloned().collect();

        if subset.is_empty() {
            None
        } else {
            Some(Arc::from(subset))
        }
    }
}

#[derive(Debug)]
pub struct EmojiAction {
    pub for_tone: u8,
    emoji: RwLock<&'static str>,
    selected_index: AtomicU8,
}
impl EmojiAction {
    pub fn emoji(&self) -> &'static str {
        let emj = self.emoji.read().unwrap();
        *emj
    }
    pub fn update_index<F>(&self, f: F)
    where
        F: FnOnce(u8) -> u8,
    {
        let current = self.selected_index.load(Ordering::SeqCst);
        let new_value = f(current);
        self.selected_index.store(new_value, Ordering::SeqCst);
    }
    pub fn get_index(&self) -> u8 {
        self.selected_index.load(Ordering::SeqCst)
    }
}

impl PartialEq for EmojiAction {
    fn eq(&self, other: &Self) -> bool {
        let self_emoji = self.emoji.read().unwrap();
        let other_emoji = other.emoji.read().unwrap();

        *self_emoji == *other_emoji
            && self.for_tone == other.for_tone
            && self.selected_index.load(Ordering::SeqCst)
                == other.selected_index.load(Ordering::SeqCst)
    }
}
