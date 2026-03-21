use crate::{get_window_options, ui::error::error_box::ErrorBox};
use gpui::{
    App, AppContext, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, Styled, Window, div, hsla, px, rgb,
};

pub fn spawn_error_window(cx: &mut App, error: String) {
    cx.open_window(get_window_options(), |_, cx| {
        cx.new(move |cx| ErrorWindow {
            error: error.clone(),
            focus_handle: cx.focus_handle(),
        })
    })
    .unwrap()
    .update(cx, |view, window, cx| {
        window.focus(&view.focus_handle);
        cx.activate(true);
    })
    .unwrap();
}

struct ErrorWindow {
    error: String,
    focus_handle: FocusHandle,
}

impl Render for ErrorWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|_, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" {
                    cx.quit();
                }
            }))
            .bg(rgb(0x0F0F0F))
            .border_2()
            .border_color(hsla(0., 0., 0.1882, 1.0))
            .rounded_md()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .child(
                div()
                    .text_size(px(18.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xff6b6b))
                    .child("Sherlock failed to start"),
            )
            .child(ErrorBox::new(self.error.clone()))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(0x555555))
                    .child("Press Escape to close"),
            )
    }
}
