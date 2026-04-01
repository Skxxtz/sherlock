use std::sync::Arc;

use gpui::{
    AnyElement, App, Image, ImageSource, IntoElement, ParentElement, Styled, div, img,
    prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher,
        children::{RenderableChildImpl, Selection},
    },
    loader::utils::AppData,
    ui::launcher::context_menu::ContextMenuAction,
};

impl<'a> RenderableChildImpl<'a> for AppData {
    fn render(
        &self,
        launcher: &Arc<Launcher>,
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
            .child(if let Some(icon) = self.icon.as_ref() {
                img(Arc::clone(&icon)).size(px(24.)).into_any_element()
            } else {
                img(ImageSource::Image(Arc::new(Image::empty())))
                    .size(px(24.))
                    .into_any_element()
            })
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .child(
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
                            .children(
                                self.name
                                    .as_ref()
                                    .or(launcher.display_name.as_ref())
                                    .map(|name| div().child(name.clone())),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(theme.font_family.clone())
                            .text_color(theme.secondary_text)
                            .children(launcher.name.as_ref().map(|name| div().child(name.clone()))),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<Launcher>) -> Option<ExecMode> {
        Some(ExecMode::from_appdata(self, launcher))
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        self.priority.unwrap_or(launcher.priority as f32)
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.search_string
    }
    #[inline(always)]
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        Some(self.actions.clone())
    }
    #[inline(always)]
    fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }
}
