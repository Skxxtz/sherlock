use std::sync::{
    Arc, LazyLock, OnceLock, RwLock,
    atomic::{AtomicU8, AtomicUsize, Ordering},
};

use arrayvec::ArrayString;
use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, px};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher,
        emoji_launcher::{
            EmojiData, SkinTone,
            data::{EMOJIS, EmojiEntry},
        },
    },
    ui::{
        launcher::context_menu::ContextMenuAction,
        widgets::{RenderableChildImpl, Selection},
    },
};

static SELECTED_SKIN_TONE: OnceLock<RwLock<[SkinTone; 2]>> = OnceLock::new();
static EMOJI_CONTEXT_ACTIONS: LazyLock<Arc<[Arc<ContextMenuAction>]>> = LazyLock::new(|| {
    let default_tone = get_selected_skin_tones()[0];
    Arc::from([
        Arc::new(ContextMenuAction::Emoji(EmojiAction {
            selected_index: AtomicU8::new(default_tone as u8),
            for_tone: 0,
            ..Default::default()
        })),
        Arc::new(ContextMenuAction::Emoji(EmojiAction {
            selected_index: AtomicU8::new(default_tone as u8),
            for_tone: 1,
            ..Default::default()
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

pub fn get_emoji(entry: &EmojiEntry, tones: &[SkinTone]) -> ArrayString<64> {
    let all_same = tones.windows(2).all(|w| w[0] == w[1]);
    if all_same {
        if let Some(fallback) = entry.same_tone_emoji {
            apply_skin_tones(fallback, &tones[..1])
        } else {
            apply_skin_tones(entry.emoji, tones)
        }
    } else {
        apply_skin_tones(entry.emoji, tones)
    }
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
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        let emoji = get_emoji(&self.entry, &get_selected_skin_tones());
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
                    .text_color(theme.primary_text)
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
                    .font_family(theme.font_family.clone())
                    .text_center()
                    .text_color(if selection.is_selected {
                        theme.primary_text
                    } else {
                        theme.secondary_text
                    })
                    .child(self.entry.name),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        Some(ExecMode::Copy {
            content: get_emoji(&self.entry, &get_selected_skin_tones())
                .as_str()
                .to_string(),
        })
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.entry.keywords
    }
    #[inline(always)]
    fn actions(
        &self,
        _launcher: &Arc<Launcher>,
        _cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let num_tones = self.entry.skin_tones as usize;
        let template = &*EMOJI_CONTEXT_ACTIONS;

        for action_arc in template.iter().take(num_tones) {
            if let ContextMenuAction::Emoji(act) = action_arc.as_ref() {
                let idx = EMOJIS
                    .iter()
                    .position(|e| std::ptr::eq(e, self.entry))
                    .unwrap();
                act.emoji.store(idx, Ordering::Relaxed);
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
    #[inline(always)]
    fn has_actions(&self, _cx: &mut App) -> bool {
        self.entry.skin_tones > 0
    }
}

#[derive(Debug, Default)]
pub struct EmojiAction {
    pub for_tone: u8,
    emoji: AtomicUsize,
    selected_index: AtomicU8,
}
impl EmojiAction {
    pub fn entry(&self) -> Option<&EmojiEntry> {
        EMOJIS.get(self.emoji.load(Ordering::Relaxed))
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
        self.emoji.load(Ordering::Relaxed) == other.emoji.load(Ordering::Relaxed)
            && self.for_tone == other.for_tone
    }
}
