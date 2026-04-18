use std::sync::Arc;

use gpui::{
    AnyElement, App, AppContext, Image, ImageSource, IntoElement, ParentElement, Styled, div, img,
    prelude::FluentBuilder, px,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher, audio_launcher::MusicPlayerFunctions, utils::MprisState,
        variant_type::InnerFunction,
    },
    ui::widgets::{RenderableChildImpl, Selection},
};

impl<'a> RenderableChildImpl<'a> for MprisState {
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
            .border_1()
            .rounded_md()
            .when(!selection.is_selected, |this| {
                this.border_color(theme.border_idle)
            })
            .child(if let Some(icon) = &self.image {
                img(ImageSource::Image(Arc::clone(icon)))
                    .size(px(64.))
                    .rounded_md()
            } else {
                img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
            })
            .child(
                div()
                    .text_color(theme.secondary_text)
                    .when(selection.is_selected, |this| {
                        this.text_color(theme.primary_text)
                    })
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_family(theme.font_family.clone())
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .children(
                                self.raw
                                    .as_ref()
                                    .and_then(|s| s.metadata.title.as_ref())
                                    .map(|name| div().child(name.clone())),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(theme.font_family.clone())
                            .children(
                                self.raw
                                    .as_ref()
                                    .and_then(|s| s.metadata.artists.as_ref())
                                    .map(|arts| arts.join(", ").to_string()),
                            ),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<Launcher>) -> Option<ExecMode> {
        Some(ExecMode::Inner {
            func: InnerFunction::MusicPlayer(MusicPlayerFunctions::TogglePlayback),
            exit: launcher.exit,
        })
    }
    #[inline(always)]
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        ""
    }
    #[inline(always)]
    fn based_show<C: AppContext>(&self, _keyword: &str, _cx: &mut C) -> Option<bool> {
        if self.raw.is_some() {
            return None;
        } else {
            Some(false)
        }
    }
}
