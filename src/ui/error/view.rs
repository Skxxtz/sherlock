use crate::{
    ui::{error::error_box::ErrorBox, launcher::Quit},
    utils::errors::SherlockError,
};
use gpui::{
    App, Context, FocusHandle, Focusable, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, div, prelude::FluentBuilder, px, rgb,
};

pub struct DismissErrorEvent;
impl gpui::EventEmitter<DismissErrorEvent> for ErrorView {}

pub struct ErrorView {
    pub errors: Vec<SherlockError>,
    pub warnings: Vec<SherlockError>,
    pub focus_handle: FocusHandle,
}

impl ErrorView {
    pub fn push(&mut self, error: SherlockError, cx: &mut Context<Self>) {
        self.errors.push(error);
        cx.notify();
    }
}

impl Render for ErrorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|_this, _: &Quit, _, cx| {
                cx.emit(DismissErrorEvent);
            }))
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_8()
            .when(!self.errors.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xff6b6b))
                        .child("Sherlock encountered errors"),
                )
                .children(self.errors.iter().map(|e| ErrorBox::new(e.to_string())))
            })
            .when(
                !self.errors.is_empty() && !self.warnings.is_empty(),
                |this| {
                    this.child(
                        div()
                            .w_full()
                            .border_t_1()
                            .border_color(rgb(0x222222))
                            .my_2(),
                    )
                },
            )
            .when(!self.warnings.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xefb827))
                        .child("Warnings"),
                )
                .children(
                    self.warnings
                        .iter()
                        .map(|e| ErrorBox::warning(e.to_string())),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(0x555555))
                    .child("Press Escape to close"),
            )
    }
}

impl Focusable for ErrorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
