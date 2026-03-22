use std::sync::{Arc, RwLock};

use gpui::{IntoElement, ParentElement, SharedString, Styled, div, px, rgb};

use crate::{
    launcher::{ExecMode, Launcher, children::RenderableChildImpl},
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
    pub fn based_show(&self, keyword: &str) -> bool {
        if keyword.trim().is_empty() {
            return false;
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
        show
    }
}

impl<'a> RenderableChildImpl<'a> for CalcData {
    fn search(&'a self, _launcher: &std::sync::Arc<crate::launcher::Launcher>) -> &'a str {
        ""
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
        let result = {
            let guard = self.result.read().unwrap();
            let Some((_, res)) = guard.as_ref() else {
                return div().into_any_element();
            };
            res.clone()
        };

        match result {
            IntentResult::String(s) => calc_tile(s, is_selected),
            IntentResult::Color(c) => color_show(c, is_selected),
        }
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
                    .text_sm()
                    .text_color(if is_selected {
                        rgb(0xffffff)
                    } else {
                        rgb(0xcccccc)
                    })
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(format!("#{:06x}", result)),
            ),
        )
        .into_any_element()
}
