use std::sync::Arc;

use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px, rgb};

use crate::launcher::{
    ExecMode, Launcher, children::RenderableChildImpl, emoji_launcher::EmojiData,
};

impl<'a> RenderableChildImpl<'a> for EmojiData {
    fn render(&self, _launcher: &Arc<Launcher>, is_selected: bool) -> AnyElement {
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
                    .child(self.entry.emoji.replace("{skin_tone}", "\u{1F3FE}")),
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
}
