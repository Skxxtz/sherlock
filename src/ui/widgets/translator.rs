use gpui::{
    Animation, AnimationExt, App, AppContext, AsyncApp, Entity, FontWeight, IntoElement,
    ParentElement, Styled, div, prelude::FluentBuilder, px,
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

mod mymemory;
mod utils;

use crate::{
    app::theme::ThemeData,
    launcher::{ExecMode, Launcher},
    sherlock_msg,
    ui::widgets::{
        RenderableChildImpl, Selection,
        translator::{
            mymemory::MyMemoryResponse,
            utils::{ApiStatus, TranslationResult},
        },
    },
    utils::{
        errors::{
            SherlockMessage,
            types::{NetworkAction, SherlockErrorType},
        },
        intent::{Intent, IntentResult, translation::Language},
    },
};

#[derive(Clone)]
pub struct TranslationData {
    update_entity: Entity<TranslationResult>,
}
impl TranslationData {
    pub fn new(cx: &mut App) -> Self {
        Self {
            update_entity: cx.new(|_| Default::default()),
        }
    }
}

impl<'a> RenderableChildImpl<'a> for TranslationData {
    #[inline(always)]
    fn search(&'a self, _launcher: &std::sync::Arc<crate::launcher::Launcher>) -> &'a str {
        ""
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<Launcher>) -> Option<ExecMode> {
        None
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
        cx: &mut App,
    ) -> gpui::AnyElement {
        let TranslationResult { intent, api, .. } = self.update_entity.read(cx);
        let Some(Intent::Translation { text, target_lang }) = intent else {
            return div()
                .size_full()
                .p_3()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .child("Translator")
                        .text_xs()
                        .text_color(theme.secondary_text),
                )
                .child(div().child("Invalid Intent").text_color(theme.primary_text))
                .into_any_element();
        };

        let display_text = match api {
            ApiStatus::Uninitialized => "Uninit...".into(),
            ApiStatus::Pending => "Loading...".into(),
            ApiStatus::Done { res } => match res {
                IntentResult::String(st) => st.clone(),
                _ => "Invalid Response".into(),
            },
            ApiStatus::Error { msg } => msg.traceback.clone(),
        };
        let is_loading = matches!(api, ApiStatus::Pending);

        div()
            .border_1()
            .rounded_md()
            .when(!selection.is_selected, |this| {
                this.border_color(theme.border_idle)
            })
            .size_full()
            .px(px(14.))
            .py(px(12.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .overflow_hidden()
                            .text_xs()
                            .text_color(theme.secondary_text)
                            .opacity(0.8)
                            .font_weight(FontWeight::BOLD)
                            .child(text.clone()),
                    )
                    .child(
                        div()
                            .px(px(5.))
                            .py(px(1.))
                            .rounded(px(4.))
                            .bg(theme.bg_code)
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(10.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.secondary_text)
                            .when(!selection.is_selected, |this| {
                                this.bg(theme.bg_idle)
                                    .border_color(theme.border_idle)
                                    .text_color(theme.secondary_text.opacity(0.6))
                            })
                            .child(target_lang.iso_code().to_uppercase()),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(1.))
                    .bg(theme.border)
                    .when(!selection.is_selected, |this| this.bg(theme.border_idle)),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_start()
                    .pt(px(2.))
                    .when(!is_loading, |this| {
                        this.child(
                            div()
                                .text_xl()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.primary_text)
                                .when(!selection.is_selected, |this| {
                                    this.text_color(theme.secondary_text)
                                })
                                .child(display_text.to_string())
                                .with_animation(
                                    "translation_appear",
                                    Animation::new(Duration::from_millis(250)),
                                    |this, frac| {
                                        let t =
                                            -(((std::f32::consts::PI * frac).cos() - 1.0) / 2.0);
                                        this.opacity(t).top(px((1.0 - t) * 4.0))
                                    },
                                ),
                        )
                    })
                    .when(is_loading, |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .children((0..3).map(|i| {
                                    let width = if i == 0 {
                                        32.
                                    } else if i == 1 {
                                        48.
                                    } else {
                                        20.
                                    };
                                    div()
                                        .h(px(5.))
                                        .w(px(width))
                                        .rounded(px(3.))
                                        .bg(theme.secondary_text)
                                        .with_animation(
                                            format!("shimmer_{}", i),
                                            Animation::new(Duration::from_millis(1400))
                                                .repeat()
                                                .with_easing(|t| {
                                                    -(((std::f32::consts::PI * t).cos() - 1.0)
                                                        / 2.0)
                                                }),
                                            move |this, frac| {
                                                let phase = i as f32 * 0.4;
                                                let t = ((frac + phase) % 1.0
                                                    * std::f32::consts::TAU)
                                                    .sin();
                                                let opacity = (t + 1.0) / 2.0 * 0.12 + 0.06;
                                                this.opacity(opacity)
                                            },
                                        )
                                }))
                                .with_animation(
                                    "shimmer_appear",
                                    Animation::new(Duration::from_millis(200)),
                                    |this, frac| {
                                        let t =
                                            -(((std::f32::consts::PI * frac).cos() - 1.0) / 2.0);
                                        this.opacity(t).top(px((1.0 - t) * 4.0))
                                    },
                                ),
                        )
                    }),
            )
            .into_any_element()
    }

    fn based_show<C: AppContext>(&self, keyword: &str, cx: &mut C) -> Option<bool> {
        if keyword.trim().is_empty() {
            return Some(false);
        }

        let intent = Intent::try_parse_translation(keyword)?;

        if let Intent::Translation { text, target_lang } = intent {
            self.update_entity.update(cx, |ent, cx| {
                ent.task = None;
                ent.intent = Some(Intent::Translation {
                    text: text.clone(),
                    target_lang: target_lang.clone(),
                });
                ent.api = ApiStatus::Pending;

                ent.task = Some(cx.spawn(
                    move |weak_self: gpui::WeakEntity<TranslationResult>, cx: &mut AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            let result = translate(&text, target_lang).await;
                            let _ = weak_self.update(&mut cx, |this, cx| {
                                this.api = result.into();
                                cx.notify();
                            });
                        }
                    },
                ));
            });

            return Some(true);
        } else {
            self.update_entity.update(cx, |ent, _| {
                ent.intent = None;
                ent.api = ApiStatus::Uninitialized;
            });
        }

        None
    }
}

pub async fn translate(
    text: &str,
    target_language: Language,
) -> Result<IntentResult, SherlockMessage> {
    // Debounce
    sleep(Duration::from_millis(400)).await;

    // URL construction
    let lang_pair = format!("Autodetect|{}", target_language.iso_code());
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair={}",
        urlencoding::encode(text),
        lang_pair
    );

    // GET request
    let response = reqwest::get(url).await.map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::NetworkError(
                NetworkAction::Get,
                "api.mymemory.translated.net".into()
            ),
            e
        )
    })?;

    // Response parsing
    let text = response
        .text()
        .await
        .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;
    let data: MyMemoryResponse = serde_json::from_str(&text)
        .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;

    Ok(IntentResult::String(data.response_data.translated_text))
}
