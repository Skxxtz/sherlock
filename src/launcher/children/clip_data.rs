use std::sync::{Arc, RwLock};

use gpui::{
    Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div, img,
    prelude::FluentBuilder, px, rgb,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher,
        children::{RenderableChildImpl, Selection},
    },
    loader::{resolve_icon_path, utils::ApplicationAction},
    ui::launcher::context_menu::ContextMenuAction,
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
                        .name("Create Bookmark")
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
}

impl<'a> RenderableChildImpl<'a> for ClipData {
    fn render(
        &self,
        _launcher: &std::sync::Arc<crate::launcher::Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
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
            (Intent::Url { url }, _) => url_show(url.clone(), selection, theme),
            (Intent::Conversion { .. }, IntentResult::String(s)) => {
                calc_tile(s.clone(), selection, theme)
            }
            (Intent::ColorConvert { .. }, IntentResult::String(s)) => {
                calc_tile(s.clone(), selection, theme)
            }
            (Intent::ColorDisplay { .. }, IntentResult::Color(c)) => {
                color_show(*c, selection, theme)
            }
            _ => div().into_any_element(),
        }
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &std::sync::Arc<crate::launcher::Launcher>) -> &'a str {
        self.content.as_str()
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        let lock = self.result.read().ok()?;
        let (_, res) = lock.as_ref()?;
        Some(ExecMode::Copy {
            content: res.to_string().into(),
        })
    }
    #[inline(always)]
    fn priority(&self, launcher: &std::sync::Arc<crate::launcher::Launcher>) -> f32 {
        launcher.priority as f32
    }
    #[inline(always)]
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        Some(self.actions.clone())
    }
    #[inline(always)]
    fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }
    fn based_show(&self, _keyword: &str) -> Option<bool> {
        Some(
            self.result
                .read()
                .ok()
                .and_then(|r| r.as_ref().map(|r| r.0.is_some()))
                .unwrap_or(false),
        )
    }
}

fn calc_tile(
    result: SharedString,
    selection: Selection,
    theme: Arc<ThemeData>,
) -> gpui::AnyElement {
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
                .text_color(theme.secondary_text)
                .when(selection.is_selected, |this| {
                    this.text_color(theme.primary_text)
                })
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(result),
        )
        .into_any_element()
}

fn color_show(result: u32, selection: Selection, theme: Arc<ThemeData>) -> gpui::AnyElement {
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
        .child(div().size(px(24.)).rounded_full().bg(rgb(result)))
        .child(
            div()
                .flex_col()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.secondary_text)
                        .when(selection.is_selected, |this| {
                            this.text_color(theme.primary_text)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(format!("#{:06X}", result)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.secondary_text)
                        .child("From Clipboard"),
                ),
        )
        .into_any_element()
}

fn url_show(url: SharedString, selection: Selection, theme: Arc<ThemeData>) -> gpui::AnyElement {
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
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .when(selection.is_selected, |this| {
                            this.text_color(theme.primary_text)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(url),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .child("From Clipboard"),
                ),
        )
        .into_any_element()
}
