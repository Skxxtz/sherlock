use std::sync::Arc;

use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px, rgb};

use crate::launcher::{ExecMode, Launcher, children::RenderableChildImpl, emoji_launcher::EmojiData};

impl<'a> RenderableChildImpl<'a> for EmojiData {
    fn render(&self, _launcher: &Arc<Launcher>, is_selected: bool) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(div().text_size(px(24.)).child(self.entry.emoji))
            .child(
                div().flex_col().child(
                    div()
                        .text_sm()
                        .text_color(if is_selected {
                            rgb(0xffffff)
                        } else {
                            rgb(0xcccccc)
                        })
                        .child(self.entry.name),
                ),
            )
            .into_any_element()
    }
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        Some(ExecMode::Copy { content: self.entry.emoji.to_string() })
    }
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.entry.keywords
    }
}

