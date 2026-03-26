use std::sync::Arc;

use gpui::{AnyElement, Image, ImageSource, IntoElement, ParentElement, Styled, div, img, px, rgb};

use crate::{
    launcher::{
        ExecMode, Launcher, audio_launcher::MusicPlayerFunctions, children::RenderableChildImpl,
        utils::MprisState, variant_type::InnerFunction,
    },
    ui::launcher::context_menu::ContextMenuAction,
};

impl<'a> RenderableChildImpl<'a> for MprisState {
    fn render(&self, _launcher: &Arc<Launcher>, is_selected: bool) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(if let Some(icon) = &self.image {
                img(ImageSource::Image(Arc::clone(icon)))
                    .size(px(64.))
                    .rounded_md()
            } else {
                img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
            })
            .child(
                div()
                    .text_color(if is_selected {
                        rgb(0xffffff)
                    } else {
                        rgb(0xcccccc)
                    })
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
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
                        div().text_xs().children(
                            self.raw
                                .as_ref()
                                .and_then(|s| s.metadata.artists.as_ref())
                                .map(|arts| arts.join(", ").to_string()),
                        ),
                    ),
            )
            .into_any_element()
    }
    fn build_exec(&self, launcher: &Arc<Launcher>) -> Option<ExecMode> {
        Some(ExecMode::Inner {
            func: InnerFunction::MusicPlayer(MusicPlayerFunctions::TogglePlayback),
            exit: launcher.exit,
        })
    }
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        ""
    }
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        None
    }
}
