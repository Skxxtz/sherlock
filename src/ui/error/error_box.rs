use gpui::{
    FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled, div, hsla, px,
    rgb,
};

use crate::utils::errors::{SherlockMessage, SherlockMessageLevel};

#[allow(dead_code)]
pub enum MessageType {
    Info,
    Warning,
    Error,
}

pub struct ErrorBox {
    message: SherlockMessage,
    on_dismiss: Option<Box<dyn Fn(&mut gpui::App) + 'static>>,
}

#[allow(dead_code)]
impl ErrorBox {
    pub fn new(message: SherlockMessage) -> Self {
        Self {
            message,
            on_dismiss: None,
        }
    }
    pub fn on_dismiss(mut self, f: impl Fn(&mut gpui::App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(f));
        self
    }
}

impl IntoElement for ErrorBox {
    type Element = gpui::AnyElement;
    fn into_element(self) -> Self::Element {
        let (bg, border, text) = match self.message.level {
            SherlockMessageLevel::Error => (
                hsla(0.0, 0.7, 0.08, 1.0),
                hsla(0.0, 0.7, 0.35, 0.4),
                rgb(0xcc8888),
            ),
            SherlockMessageLevel::Warning => (
                hsla(0.11, 0.8, 0.08, 1.0),
                hsla(0.11, 0.8, 0.4, 0.4),
                rgb(0xc9943a),
            ),
            SherlockMessageLevel::Info => (
                hsla(0.6, 0.5, 0.08, 1.0),
                hsla(0.6, 0.5, 0.4, 0.4),
                rgb(0x7a9ec4),
            ),
        };

        let dismiss_btn = self.on_dismiss.map(|f| {
            div()
                .id("dismiss")
                .absolute()
                .top(px(6.))
                .right(px(8.))
                .px(px(4.))
                .py(px(1.))
                .rounded_sm()
                .text_size(px(10.))
                .text_color(text)
                .cursor_pointer()
                .group_hover("error-box", |s| s.text_color(text))
                .hover(|s| s.bg(border))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| f(cx))
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
                                // Main Error Title
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(text)
                                    .child(self.message.error_type.to_string()),
                            )
                            .children(dismiss_btn),
                    )
                    .child(
                        // Traceback / Content
                        div()
                            .mt_1()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .font_family("monospace")
                            .text_color(text)
                            .opacity(0.8) // Makes it look less "busy"
                            .child(self.message.traceback.clone()),
                    ),
            )
            .into_any_element()
    }
}
