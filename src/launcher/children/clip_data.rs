use std::sync::{Arc, RwLock};

use gpui::{
    Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div, img, px, rgb,
};

use crate::{
    launcher::{ExecMode, Launcher, children::RenderableChildImpl},
    loader::{
        resolve_icon_path,
        utils::{ApplicationAction, ContextMenuAction},
    },
    utils::{
        clipboard::get_clipboard,
        intent::{Capabilities, Intent, IntentResult},
    },
};

#[derive(Clone)]
pub struct ClipData {
    pub content: SharedString,
    pub capabilities: Capabilities,
    result: Arc<RwLock<Option<(Intent, IntentResult)>>>,
    pub actions: Arc<[Arc<ContextMenuAction>]>,
}

impl ClipData {
    pub fn new(capabilities: Capabilities, content: SharedString) -> Self {
        let mut this = Self {
            content,
            capabilities,
            result: Arc::new(RwLock::new(None)),
            actions: Arc::from([]),
        };
        this.update_async();

        this
    }
    pub fn update_async(&mut self) -> Option<()> {
        let content = get_clipboard()?;
        let intent = Intent::parse(&content, &self.capabilities);

        // early return if intents are the same
        if let Ok(guard) = self.result.read() {
            if let Some((res_intent, _)) = guard.as_ref() {
                if res_intent == &intent {
                    return None;
                }
            }
        }

        let r = match &intent {
            Intent::ColorConvert { .. } => intent.execute(),
            Intent::Conversion { .. } => intent.execute(),
            Intent::ColorDisplay { .. } => intent.execute(),
            Intent::Url { url } => {
                self.actions = Arc::new([Arc::from(
                    ApplicationAction::new("create_bookmark")
                        .name("Create Bookmark".into())
                        .icon_name("sherlock-bookmark"),
                )]);
                Some(IntentResult::String(url.into()))
            }
            _ => None,
        }?;

        if let Ok(mut writer) = self.result.write() {
            *writer = Some((intent, r));
        }
        Some(())
    }

    #[inline]
    pub fn based_show(&self) -> bool {
        self.result
            .read()
            .ok()
            .and_then(|r| r.as_ref().map(|r| r.0.is_some()))
            .unwrap_or(false)
    }
}

impl<'a> RenderableChildImpl<'a> for ClipData {
    fn search(&'a self, _launcher: &std::sync::Arc<crate::launcher::Launcher>) -> &'a str {
        self.content.as_str()
    }
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        let lock = self.result.read().ok()?;
        let (_, res) = lock.as_ref()?;
        Some(ExecMode::Copy {
            content: res.to_string().into(),
        })
    }
    fn priority(&self, launcher: &std::sync::Arc<crate::launcher::Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn render(
        &self,
        _launcher: &std::sync::Arc<crate::launcher::Launcher>,
        is_selected: bool,
    ) -> gpui::AnyElement {
        let guard = self.result.read().ok();
        let Some((intent, result)) = guard
            .as_ref()
            .and_then(|r| r.as_ref())
            .map(|(i, r)| (i.clone(), r.clone()))
        else {
            return div().into_any_element();
        };

        match (&intent, &result) {
            (Intent::Url { url }, _) => url_show(url.clone(), is_selected),
            (Intent::Conversion { .. }, IntentResult::String(s)) => {
                calc_tile(s.clone(), is_selected)
            }
            (Intent::ColorConvert { .. }, IntentResult::String(s)) => {
                calc_tile(s.clone(), is_selected)
            }
            (Intent::ColorDisplay { .. }, IntentResult::Color(c)) => color_show(*c, is_selected),
            _ => div().into_any_element(),
        }
    }
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        Some(self.actions.clone())
    }
}

fn calc_tile(result: SharedString, is_selected: bool) -> gpui::AnyElement {
    div()
        .px_4()
        .py_7()
        .size_full()
        .flex()
        .gap_5()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(24.0))
                .text_color(if is_selected {
                    rgb(0xDDD5D0)
                } else {
                    rgb(0x6E6E6E)
                })
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(result),
        )
        .into_any_element()
}

fn color_show(result: u32, is_selected: bool) -> gpui::AnyElement {
    div()
        .px_4()
        .py_2()
        .w_full()
        .flex()
        .gap_5()
        .items_center()
        .child(div().size(px(24.)).rounded_full().bg(rgb(result)))
        .child(
            div()
                .flex_col()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(if is_selected {
                            rgb(0xffffff)
                        } else {
                            rgb(0xcccccc)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(format!("#{:06X}", result)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if is_selected {
                            rgb(0x999999)
                        } else {
                            rgb(0x666666)
                        })
                        .child("From Clipboard"),
                ),
        )
        .into_any_element()
}

fn url_show(url: SharedString, is_selected: bool) -> gpui::AnyElement {
    div()
        .px_4()
        .py_2()
        .w_full()
        .flex()
        .gap_5()
        .items_center()
        .child(if let Some(icon) = resolve_icon_path("sherlock-link") {
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
                        .text_color(if is_selected {
                            rgb(0xffffff)
                        } else {
                            rgb(0xcccccc)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(url),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if is_selected {
                            rgb(0x999999)
                        } else {
                            rgb(0x666666)
                        })
                        .child("From Clipboard"),
                ),
        )
        .into_any_element()
}
