use std::sync::Arc;

use gpui::{
    AnyElement, App, IntoElement, ParentElement, SharedString, Styled, div, prelude::FluentBuilder,
};

use crate::{
    app::theme::ThemeData,
    launcher::Launcher,
    ui::widgets::{RenderableChildImpl, Selection},
};

#[derive(Clone, Default)]
pub struct DmenuData {
    name: SharedString,
}

impl<'a> RenderableChildImpl<'a> for DmenuData {
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(
                div().flex_col().justify_between().items_center().child(
                    div()
                        .text_sm()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .when(selection.is_selected, |this| {
                            this.text_color(theme.primary_text)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(self.name.clone()),
                ),
            )
            .into_any_element()
    }
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<crate::launcher::ExecMode> {
        None
    }
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.name
    }
}

impl<T> From<T> for DmenuData
where
    T: Into<SharedString>,
{
    fn from(value: T) -> Self {
        Self { name: value.into() }
    }
}
