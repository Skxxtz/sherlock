use std::{env::home_dir, process::Stdio, sync::Arc, time::Duration};

use gpui::{
    App, AsyncApp, Entity, IntoElement, ParentElement, SharedString, Styled, Task, WeakEntity, div,
    prelude::FluentBuilder,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, BufReader},
    process::Command,
    time::timeout,
};

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
                                        .line_height(gpui::relative(1.4))
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
    fn based_show(&self, _keyword: &str) -> Option<bool> {
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
    fn update_sync(&self, query: SharedString, cx: &mut App) {
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

            this.task = None;
            this.last_query = query.clone();

            let command = self.command.clone();
            let args = self.args.clone();

            this.task = Some(cx.spawn(
                |weak_self: WeakEntity<ScriptDataUpdateEntity>, cx: &mut AsyncApp| {
                    let mut cx_inner = cx.clone();
                    async move {
                        let script_fut = ScriptData::get_result(command.clone(), args.clone(), query.as_str());
                        let timer_fut = cx_inner.background_executor().timer(Duration::from_millis(75));
                        tokio::select! {
                            result = script_fut => {
                                // Script finished fast
                                let _ = weak_self.update(&mut cx_inner, |this, cx| {
                                    this.task = None;
                                    this.show_loading = false;
                                    this.result = result;
                                    cx.notify();
                                });
                            }
                            _ = timer_fut => {
                                // Script is taking a while
                                let _ = weak_self.update(&mut cx_inner, |this, cx| {
                                    this.show_loading = true;
                                    cx.notify();
                                });

                                // wait for the actual script result
                                let result = ScriptData::get_result(command, args, query.as_str()).await;
                                let _ = weak_self.update(&mut cx_inner, |this, cx| {
                                    this.task = None;
                                    this.show_loading = false;
                                    this.result = result;
                                    cx.notify();
                                });
                            }
                        }
                    }
                },
            ));
        })
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
        let args_iter = split_as_command(&processed_args);

        let mut cmd = Command::new(absolute_exec);
        cmd.args(args_iter)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let mut response = AsyncCommandResponse::new();
                response.title = Some("Failed to execute script.".into());
                response.content = Some(format!("Execution Error: {}", e).into());
                return Some(response);
            }
        };

        let result = timeout(Duration::from_secs(10), async {
            let mut stdout_content = String::new();
            let mut stderr_content = String::new();

            if let Some(stdout) = child.stdout.take() {
                let mut reader = BufReader::new(stdout);
                let _ = reader.read_to_string(&mut stdout_content).await;
            }

            if let Some(stderr) = child.stderr.take() {
                let mut reader = BufReader::new(stderr);
                let _ = reader.read_to_string(&mut stderr_content).await;
            }

            let status = child.wait().await;
            (status, stdout_content, stderr_content)
        })
        .await;

        match result {
            Ok((Ok(status), stdout, stderr)) => {
                if status.success() {
                    let mut input = stdout.into_bytes();
                    match simd_json::from_slice::<AsyncCommandResponse>(&mut input) {
                        Ok(res) => Some(res),
                        Err(e) => {
                            let mut response = AsyncCommandResponse::new();
                            response.title = Some("Invalid JSON Output".into());
                            response.content = Some(format!("Parse error: {}", e).into());
                            Some(response)
                        }
                    }
                } else {
                    let mut response = AsyncCommandResponse::new();
                    response.title = Some("Script Error".into());
                    response.content =
                        Some(format!("Exit Status: {}\nStderr: {}", status, stderr).into());
                    Some(response)
                }
            }
            Ok((Err(e), _, _)) => {
                let mut response = AsyncCommandResponse::new();
                response.title = Some("Runtime Error".into());
                response.content = Some(e.to_string().into());
                Some(response)
            }
            Err(_) => {
                let _ = child.kill().await;
                let mut response = AsyncCommandResponse::new();
                response.title = Some("Timeout".into());
                response.content = Some("The script took too long to respond (10s).".into());
                Some(response)
            }
        }
    }
}
