use gpui::{IntoElement, ParentElement, Styled, div, px, rgb};

pub struct ErrorBox {
    message: String,
}

impl ErrorBox {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl IntoElement for ErrorBox {
    type Element = gpui::AnyElement;
    fn into_element(self) -> Self::Element {
        div()
            .w_full()
            .max_w(px(500.0))
            .px_4()
            .py_3()
            .rounded_md()
            .bg(rgb(0x1a1a1a))
            .border_1()
            .border_color(rgb(0xff6b6b))
            .text_size(px(12.0))
            .text_color(rgb(0xcc8888))
            .font_family("monospace")
            .child(self.message)
            .into_any_element()
    }
}
