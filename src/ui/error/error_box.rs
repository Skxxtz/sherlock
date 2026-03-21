use gpui::{IntoElement, ParentElement, Styled, div, hsla, px, rgb};

pub enum MessageType {
    Info,
    Warning,
    Error,
}

pub struct ErrorBox {
    message: String,
    message_type: MessageType,
}

impl ErrorBox {
    pub fn new(message: String) -> Self {
        Self {
            message,
            message_type: MessageType::Error,
        }
    }

    pub fn warning(message: String) -> Self {
        Self {
            message,
            message_type: MessageType::Warning,
        }
    }

    pub fn info(message: String) -> Self {
        Self {
            message,
            message_type: MessageType::Info,
        }
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

        div()
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
            .child(self.message)
            .into_any_element()
    }
}
