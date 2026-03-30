use std::sync::{Arc, RwLock};

use gpui::{
    IntoElement, ParentElement, SharedString, Styled, div, prelude::FluentBuilder, px, rgb,
};

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher,
        children::{RenderableChildImpl, Selection},
    },
    utils::intent::{Capabilities, Intent, IntentResult},
};

#[derive(Clone)]
pub struct CalcData {
    capabilities: Capabilities,
    result: Arc<RwLock<Option<(SharedString, IntentResult)>>>,
}

impl CalcData {
    pub fn new(capabilities: Capabilities) -> Self {
        Self {
            capabilities,
            result: Arc::new(RwLock::new(None)),
        }
    }
}

impl<'a> RenderableChildImpl<'a> for CalcData {
    #[inline(always)]
    fn search(&'a self, _launcher: &std::sync::Arc<crate::launcher::Launcher>) -> &'a str {
        ""
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
    fn render(
        &self,
        _launcher: &std::sync::Arc<crate::launcher::Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
    ) -> gpui::AnyElement {
        let result = {
            let guard = self.result.read().unwrap();
            let Some((_, res)) = guard.as_ref() else {
                return div().into_any_element();
            };
            res.clone()
        };

        match result {
            IntentResult::String(s) => calc_tile(s, selection, theme),
            IntentResult::Color(c) => color_show(c, selection, theme),
        }
    }
    fn based_show(&self, keyword: &str) -> Option<bool> {
        if keyword.trim().is_empty() {
            return Some(false);
        }

        let mut result = None;

        if self.capabilities.allows(Capabilities::MATH) {
            let trimmed_keyword = keyword.trim();
            if let Ok(r) = meval::eval_str(trimmed_keyword) {
                let r = r.to_string();
                if &r != trimmed_keyword {
                    result = Some((r.clone(), IntentResult::String(format!("= {}", r).into())));
                }
            }
        }

        {
            let intent = Intent::parse(keyword, &self.capabilities);
            let r = match intent {
                Intent::ColorConvert { .. } => intent.execute(),
                Intent::Conversion { .. } => intent.execute(),
                Intent::ColorDisplay { .. } => intent.execute(),
                _ => None,
            };

            if let Some(r) = r {
                result = Some((keyword.to_string(), r));
            }
        }

        let show = result.is_some();
        if let Ok(mut writer) = self.result.write() {
            *writer = result.map(|(o, r)| (SharedString::from(o), r));
        }
        Some(show)
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
                .font_family(theme.font_family.clone())
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
        .child(
            div()
                .size(px(24.))
                .rounded_full()
                .bg(rgb(result))
                .flex_shrink_0(),
        )
        .child(
            div().flex_col().justify_between().items_center().child(
                div()
                    .font_family(theme.font_family.clone())
                    .text_sm()
                    .text_color(theme.secondary_text)
                    .when(selection.is_selected, |this| {
                        this.text_color(theme.primary_text)
                    })
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(format!("#{:06x}", result)),
            ),
        )
        .into_any_element()
}
