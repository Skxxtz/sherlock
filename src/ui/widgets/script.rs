use std::{env::home_dir, sync::Arc, time::Duration};

use gpui::{
    App, AppContext, AsyncApp, Entity, IntoElement, ParentElement, SharedString, Styled, Task,
    WeakEntity, div, prelude::FluentBuilder,
};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

use crate::{
    launcher::Launcher,
    ui::{
        launcher::context_menu::ContextMenuAction, utils::pango::render_pango,
        widgets::RenderableChildImpl,
    },
    utils::command_launch::split_as_command,
};

#[derive(Clone)]
pub struct ScriptData {
    pub update_entity: Entity<ScriptDataUpdateEntity>,
    pub command: SharedString,
    pub args: SharedString,
}

#[derive(Default)]
pub struct ScriptDataUpdateEntity {
    pub result: Option<AsyncCommandResponse>,
    pub show_loading: bool,
    pub last_query: SharedString,
    pub task: Option<Task<()>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AsyncCommandResponse {
    pub title: Option<SharedString>,
    pub content: Option<SharedString>,
    pub next_content: Option<SharedString>,
    pub result: Option<SharedString>,
    pub actions: Option<Arc<[Arc<ContextMenuAction>]>>,
}
impl AsyncCommandResponse {
    fn new() -> Self {
        AsyncCommandResponse {
            title: None,
            content: None,
            next_content: None,
            actions: None,
            result: None,
        }
    }
}

impl<'a> RenderableChildImpl<'a> for ScriptData {
    fn render(
        &self,
        _launcher: &std::sync::Arc<crate::launcher::Launcher>,
        _selection: super::Selection,
        theme: std::sync::Arc<crate::app::theme::ThemeData>,
        cx: &mut App,
    ) -> gpui::AnyElement {
        let state = self.update_entity.read(cx);
        let is_loading = state.show_loading;
        let (display_text, is_placeholder) = state
            .result
            .as_ref()
            .and_then(|r| r.title.clone())
            .map(|title| (title, false))
            .unwrap_or_else(|| {
                if !state.last_query.is_empty() {
                    (state.last_query.clone(), false)
                } else {
                    (SharedString::from("Start typing to search..."), true)
                }
            });

        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .px_5()
            .py_4()
            .child(
                div().flex().items_center().gap_4().w_full().child(
                    div()
                        .flex_col()
                        .flex_1()
                        .overflow_hidden()
                        .child(
                            div()
                                .font_family(theme.font_family.clone())
                                .text_sm()
                                .text_color(if is_loading {
                                    theme.secondary_text
                                } else {
                                    theme.primary_text
                                })
                                .when(is_placeholder, |this| this.italic().opacity(0.6))
                                .child(display_text),
                        )
                        .when_some(
                            state.result.as_ref().and_then(|r| r.content.as_ref()),
                            |this, content| {
                                this.child(
                                    div()
                                        .w_full()
                                        .mt_0p5()
                                        .text_xs()
                                        .text_color(theme.secondary_text)
                                        .line_height(gpui::relative(1.15))
                                        .child(render_pango(content.as_str(), &theme)),
                                )
                            },
                        )
                        // Loading Indicator
                        .when(is_loading, |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .text_xs()
                                    .text_color(theme.secondary_text)
                                    .italic()
                                    .child("Fetching results..."),
                            )
                        }),
                ),
            )
            .into_any_element()
    }
    fn priority(&self, launcher: &std::sync::Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn search(&'a self, _launcher: &std::sync::Arc<Launcher>) -> &'a str {
        ""
    }
    fn build_exec(
        &self,
        _launcher: &std::sync::Arc<Launcher>,
    ) -> Option<crate::launcher::ExecMode> {
        None
    }
    fn based_show<C: AppContext>(&self, _keyword: &str, _cx: &mut C) -> Option<bool> {
        Some(true)
    }
    fn has_actions(&self, cx: &mut App) -> bool {
        self.update_entity
            .read(cx)
            .result
            .as_ref()
            .and_then(|res| res.actions.as_ref())
            .map_or(false, |actions| !actions.is_empty())
    }
    fn actions(
        &self,
        launcher: &Arc<Launcher>,
        cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        let actions = self
            .update_entity
            .read(cx)
            .result
            .as_ref()
            .and_then(|r| r.actions.as_ref());
        let extra = launcher.add_actions.as_ref();

        let mut cap = actions.map_or(0, |a| a.len());
        if let Some(adds) = extra {
            cap += adds.len();
        }

        if cap == 0 {
            return None;
        }
        let mut combined = Vec::with_capacity(cap);

        if let Some(actions) = actions {
            combined.extend(actions.iter().cloned());
        }

        if let Some(extra) = extra {
            combined.extend(extra.iter().cloned());
        }

        Some(combined.into())
    }
    fn update_sync(&self, query: SharedString, launcher: &Arc<Launcher>, cx: &mut App) {
        self.update_entity.update(cx, |this, cx| {
            if query.is_empty() {
                this.task = None;
                this.result = None;
                this.last_query = SharedString::default();
                this.show_loading = false;
                cx.notify();
                return;
            }

            if this.last_query == query {
                return;
            }

            this.last_query = query.clone();
        });

        if launcher.r#async {
            self.update_async(cx);
        }
    }
}

impl ScriptData {
    pub async fn get_result(
        command: SharedString,
        args_template: SharedString,
        keyword: &str,
    ) -> Option<AsyncCommandResponse> {
        if args_template.contains("{keyword}") && keyword.trim().is_empty() {
            return None;
        }

        let absolute_exec = if command.starts_with("~/") {
            home_dir()?.join(command.strip_prefix("~/").unwrap())
        } else {
            std::path::PathBuf::from(command.to_string())
        };

        let processed_args = args_template.replace("{keyword}", keyword);
        let args_iter: Vec<String> = split_as_command(&processed_args);

        // Run the blocking process on a thread
        let result = smol::unblock(move || {
            let child = match Command::new(&absolute_exec)
                .args(&args_iter)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    let mut response = AsyncCommandResponse::new();
                    response.title = Some("Failed to execute script.".into());
                    response.content = Some(format!("Execution Error: {}", e).into());
                    return Some(response);
                }
            };

            let timeout_result = {
                use std::sync::mpsc;
                let (tx, rx) = mpsc::channel();
                let _ = std::thread::spawn(move || {
                    let out = child.wait_with_output();
                    let _ = tx.send(out);
                });
                rx.recv_timeout(std::time::Duration::from_secs(10))
            };

            match timeout_result {
                Ok(Ok(output)) => {
                    if output.status.success() {
                        let mut input = output.stdout;
                        match simd_json::from_slice::<AsyncCommandResponse>(&mut input) {
                            Ok(res) => Some(res),
                            Err(_) => {
                                let mut response = AsyncCommandResponse::new();
                                response.title = None;
                                response.content = match String::from_utf8(input) {
                                    Ok(s) => Some(s.into()),
                                    Err(_) => Some("Invalid stdout.".into()),
                                };
                                Some(response)
                            }
                        }
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let mut response = AsyncCommandResponse::new();
                        response.title = Some("Script Error".into());
                        response.content = Some(
                            format!("Exit Status: {}\nStderr: {}", output.status, stderr).into(),
                        );
                        Some(response)
                    }
                }
                Ok(Err(e)) => {
                    let mut response = AsyncCommandResponse::new();
                    response.title = Some("Runtime Error".into());
                    response.content = Some(e.to_string().into());
                    Some(response)
                }
                Err(_) => {
                    let mut response = AsyncCommandResponse::new();
                    response.title = Some("Timeout".into());
                    response.content = Some("The script took too long to respond (10s).".into());
                    Some(response)
                }
            }
        })
        .await;

        result
    }
}

impl ScriptData {
    pub fn update_async<C: AppContext>(&self, cx: &mut C) {
        self.update_entity.update(cx, |this, cx| {
            this.task = None;

            let command = self.command.clone();
            let args = self.args.clone();
            let query = this.last_query.clone();

            this.task = Some(cx.spawn(
                |weak_self: WeakEntity<ScriptDataUpdateEntity>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        let result =
                            ScriptData::get_result(command.clone(), args.clone(), query.as_str());
                        let timer = cx.background_executor().timer(Duration::from_millis(75));

                        futures::pin_mut!(result);
                        futures::pin_mut!(timer);

                        match futures::future::select(result, timer).await {
                            futures::future::Either::Left((res, _)) => {
                                // Done before timer
                                let _ = weak_self.update(&mut cx, |this, cx| {
                                    this.task = None;
                                    this.show_loading = false;
                                    this.result = res;
                                    cx.notify();
                                });
                            }
                            futures::future::Either::Right((_, result_fut)) => {
                                // wait for result
                                let _ = weak_self.update(&mut cx, |this, cx| {
                                    this.show_loading = true;
                                    cx.notify();
                                });
                                let res = result_fut.await;
                                let _ = weak_self.update(&mut cx, |this, cx| {
                                    this.task = None;
                                    this.show_loading = false;
                                    this.result = res;
                                    cx.notify();
                                });
                            }
                        }
                    }
                },
            ));
        });
    }
}
