use std::sync::Arc;

use gpui::{
    AnyElement, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled,
    div, prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher,
        children::{RenderableChildImpl, Selection},
    },
    utils::errors::{SherlockMessage, SherlockMessageLevel},
};

#[derive(Clone)]
pub struct MessageChild {
    pub message: SherlockMessage,
    pub on_dismiss: Option<Arc<dyn Fn(&mut gpui::App, usize) + Send + Sync + 'static>>,
}

impl MessageChild {
    pub fn new(message: SherlockMessage) -> Self {
        Self {
            message,
            on_dismiss: None,
        }
    }
    pub fn on_dismiss(mut self, f: impl Fn(&mut gpui::App, usize) + Send + Sync + 'static) -> Self {
        self.on_dismiss = Some(std::sync::Arc::new(f));
        self
    }
}

impl<'a> RenderableChildImpl<'a> for MessageChild {
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
    ) -> AnyElement {
        let (bg, border, text) = match self.message.level {
            SherlockMessageLevel::Error => (
                theme
                    .color_err
                    .alpha(if selection.is_selected { 0.15 } else { 0.08 }),
                theme
                    .color_err
                    .alpha(if selection.is_selected { 0.8 } else { 0.4 }),
                theme.color_err,
            ),
            SherlockMessageLevel::Warning => (
                theme
                    .color_warn
                    .alpha(if selection.is_selected { 0.15 } else { 0.08 }),
                theme
                    .color_warn
                    .alpha(if selection.is_selected { 0.8 } else { 0.4 }),
                theme.color_warn,
            ),
            SherlockMessageLevel::Info => (
                theme
                    .color_succ
                    .alpha(if selection.is_selected { 0.15 } else { 0.08 }),
                theme
                    .color_succ
                    .alpha(if selection.is_selected { 0.8 } else { 0.4 }),
                theme.color_succ,
            ),
        };

        let dismiss_btn = self.on_dismiss.as_ref().map(|f| {
            let f = f.clone();
            div()
                .id("dismiss")
                .absolute()
                .top(px(6.))
                .right(px(8.))
                .px(px(4.))
                .py(px(1.))
                .rounded_sm()
                .text_size(px(10.))
                .font_family(theme.font_family.clone())
                .text_color(text)
                .cursor_pointer()
                .group_hover("error-box", |s| s.text_color(text))
                .hover(|s| s.bg(border))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| f(cx, selection.data_idx))
                .child("✕")
        });

        div()
            .id("error-box")
            .group("error-box")
            .w_full()
            .px_4()
            .py_3()
            .rounded_md()
            .bg(bg)
            .border_1()
            .border_color(border)
            .when(selection.is_selected, |this| this.shadow_md())
            .text_size(px(12.0))
            .text_color(text)
            .font_family("monospace")
            .relative()
            .child(
                div()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(if selection.is_selected {
                                        theme.primary_text
                                    } else {
                                        text
                                    })
                                    .child(self.message.error_type.to_string()),
                            )
                            .children(dismiss_btn),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .font_family("monospace")
                            .text_color(text)
                            .opacity(if selection.is_selected { 1.0 } else { 0.8 })
                            .child(self.message.traceback.clone()),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        None
    }
    #[inline(always)]
    fn priority(&self, _launcher: &Arc<Launcher>) -> f32 {
        1.0
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.message.traceback
    }
}
