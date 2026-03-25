use std::sync::{Arc, OnceLock, RwLock};

use arrayvec::ArrayString;
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px, rgb};

use crate::launcher::{
    ExecMode, Launcher,
    children::RenderableChildImpl,
    emoji_launcher::{EmojiData, SkinTone},
};

static SELECTED_SKIN_TONE: OnceLock<RwLock<[SkinTone; 2]>> = OnceLock::new();

pub fn set_selected_skin_tone(tone: SkinTone, place: usize) {
    let lock = SELECTED_SKIN_TONE.get_or_init(|| RwLock::new([tone, tone]));

    if let Ok(mut w) = lock.write() {
        if place < w.len() {
            w[place] = tone;
        }
    }
}
fn get_selected_skin_tones() -> [SkinTone; 2] {
    let lock =
        SELECTED_SKIN_TONE.get_or_init(|| RwLock::new([SkinTone::Simpsons, SkinTone::Simpsons]));

    *lock.read().unwrap_or_else(|e| e.into_inner())
}

fn apply_skin_tones(template: &str, tones: &[SkinTone]) -> ArrayString<64> {
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
    fn actions(&self) -> Option<Arc<[Arc<crate::loader::utils::ApplicationAction>]>> {
        None
    }
}
