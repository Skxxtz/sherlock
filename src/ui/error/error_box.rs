use gpui::{
    InteractiveElement, IntoElement, MouseButton, ParentElement, Styled, div, hsla, px, rgb,
};

#[allow(dead_code)]
pub enum MessageType {
    Info,
    Warning,
    Error,
}

pub struct ErrorBox {
    message: String,
    message_type: MessageType,
    on_dismiss: Option<Box<dyn Fn(&mut gpui::App) + 'static>>,
}

impl ErrorBox {
    pub fn new(message: String) -> Self {
        Self {
            message,
            message_type: MessageType::Error,
            on_dismiss: None,
        }
    }
    pub fn warning(message: String) -> Self {
        Self {
            message,
            message_type: MessageType::Warning,
            on_dismiss: None,
        }
    }
    pub fn info(message: String) -> Self {
        Self {
            message,
            message_type: MessageType::Info,
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
        let (bg, border, text) = match self.message_type {
            MessageType::Error => (
                hsla(0.0, 0.7, 0.08, 1.0),
                hsla(0.0, 0.7, 0.35, 0.4),
                rgb(0xcc8888),
            ),
            MessageType::Warning => (
                hsla(0.11, 0.8, 0.08, 1.0),
                hsla(0.11, 0.8, 0.4, 0.4),
                rgb(0xc9943a),
            ),
            MessageType::Info => (
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
            .child(self.message)
            .children(dismiss_btn)
            .into_any_element()
    }
}
