use crate::{
    ui::{error::error_box::ErrorBox, launcher::Quit},
    utils::errors::SherlockMessage,
};
use gpui::{
    App, Context, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};

pub struct DismissErrorEvent;
impl gpui::EventEmitter<DismissErrorEvent> for ErrorView {}

pub struct ErrorView {
    pub messages: Vec<SherlockMessage>,
    pub focus_handle: FocusHandle,
    pub scroll_handle: ScrollHandle,
}

impl ErrorView {
    pub fn counts(&self) -> usize {
        self.messages.len()
    }

    pub fn push_error(&mut self, error: SherlockMessage, cx: &mut Context<Self>) {
        self.messages.push(error);
        cx.notify();
    }

    pub fn remove_error(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.messages.len() {
            self.messages.remove(idx);
        }
        if self.messages.is_empty() && self.messages.is_empty() {
            cx.emit(DismissErrorEvent);
        }
        cx.notify();
    }

    pub fn remove_warning(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.messages.len() {
            self.messages.remove(idx);
        }
        if self.messages.is_empty() && self.messages.is_empty() {
            cx.emit(DismissErrorEvent);
        }
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
            .size_full()
            .child(
                div()
                    .id("scroll-container")
                    .flex_1()
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .when(!self.messages.is_empty(), |this| {
                        this.children(self.messages.iter().cloned().enumerate().map(|(idx, e)| {
                            let weak_self = cx.entity().downgrade();
                            ErrorBox::new(e).on_dismiss(move |cx: &mut App| {
                                if let Some(view) = weak_self.upgrade() {
                                    view.update(cx, |this, cx| this.remove_error(idx, cx));
                                }
                            })
                        }))
                    }),
            )
    }
}

impl Focusable for ErrorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
