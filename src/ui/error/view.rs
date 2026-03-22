use crate::{
    ui::{error::error_box::ErrorBox, launcher::Quit},
    utils::errors::SherlockError,
};
use gpui::{
    App, Context, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, div, prelude::FluentBuilder,
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
            .p_6()
            .when(!self.errors.is_empty(), |this| {
                this.children(self.errors.iter().map(|e| ErrorBox::new(e.to_string())))
            })
            .when(!self.warnings.is_empty(), |this| {
                this.children(
                    self.warnings
                        .iter()
                        .map(|e| ErrorBox::warning(e.to_string())),
                )
            })
            .child(div().flex_1())
    }
}

impl Focusable for ErrorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
