use std::{path::Path, sync::Arc};

use gpui::{
    AnyElement, Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div, img,
    prelude::FluentBuilder, px,
};

use crate::{
    app::ActiveTheme,
    launcher::{
        ExecMode, Launcher,
        children::{RenderableChildImpl, Selection},
    },
    loader::resolve_icon_path,
};

#[derive(Clone, Default, Debug)]
pub struct FileData {
    loc: SharedString,
    name: SharedString,
    icon: Option<Arc<Path>>,
}

impl FileData {
    pub fn new(loc: Arc<str>) -> Self {
        let name: Arc<str> = loc
            .trim_end_matches('/')
            .rsplit_once('/')
            .map(|(_, name)| name)
            .unwrap_or(&loc)
            .into();

        Self {
            loc: loc.clone().into(),
            name: name.into(),
            icon: None,
        }
    }

    pub fn with_icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }
}

impl<'a> RenderableChildImpl<'a> for FileData {
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        selection: Selection,
        theme: &ActiveTheme,
    ) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(if let Some(icon) = self.icon.as_ref() {
                img(Arc::clone(&icon))
                    .size(px(24.))
                    .flex_shrink_0()
                    .into_any_element()
            } else {
                img(ImageSource::Image(Arc::new(Image::empty())))
                    .size(px(24.))
                    .flex_shrink_0()
                    .into_any_element()
            })
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .min_w_0()
                    .w_full()
                    .child(
                        div()
                            .text_sm()
                            .w_full()
                            .text_color(theme.secondary_text)
                            .when(selection.is_selected, |this| {
                                this.text_color(theme.primary_text)
                            })
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .w_full()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_color(theme.secondary_text)
                            .child(self.loc.clone()),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        None
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.loc
    }
}
