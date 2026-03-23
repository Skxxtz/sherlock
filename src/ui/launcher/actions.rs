use std::{path::PathBuf, sync::Arc};

use gpui::{AppContext, ClipboardItem, Context, SharedString, Window, actions};
use smallvec::SmallVec;

use crate::{
    launcher::{
        ExecMode,
        children::{LauncherValues, RenderableChild, RenderableChildDelegate},
    },
    loader::utils::{CounterReader, ExecVariable},
    ui::{
        launcher::LauncherView,
        search_bar::{EmptyBackspace, TextInput},
        workspace::LauncherErrorEvent,
    },
    utils::{command_launch::spawn_detached, errors::SherlockError, websearch::websearch},
};

actions!(
    example_input,
    [
        Quit,
        FocusNext,
        FocusPrev,
        NextVar,
        PrevVar,
        Execute,
        OpenContext,
        Backspace,
    ]
);

impl LauncherView {
    pub fn focus_first(&mut self, cx: &mut Context<Self>) {
        // early return if no indices
        if self.filtered_indices.is_empty() {
            return;
        }

        let first_valid_index = {
            let data_guard = self.data.read(cx);
            self.filtered_indices
                .iter()
                .position(|idx| data_guard[*idx].spawn_focus())
        };

        if let Some(n) = first_valid_index {
            self.focus_nth(n, cx);
        }
    }
    pub fn focus_nth(&mut self, n: usize, cx: &mut Context<Self>) {
        // early return on invalid index
        if self.filtered_indices.len() <= n {
            return;
        }

        self.selected_index = n;
        self.list_state.scroll_to_reveal_item(n);

        // Handle variable inputs
        self.update_vars(cx);
        self.active_bar = 0;

        // Handle context menu entries
        self.context_actions = self
            .filtered_indices
            .get(n)
            .and_then(|i| self.data.read(cx).get(*i))
            .and_then(RenderableChild::actions)
            .unwrap_or_default();

        cx.notify()
    }
    pub(super) fn focus_next(&mut self, _: &FocusNext, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.filtered_indices.len();
        if count == 0 {
            return;
        }

        if let Some(idx) = self.context_idx {
            // handle context
            if idx < self.context_actions.len() - 1 {
                self.context_idx = Some(idx + 1);
                cx.notify();
            }
        } else {
            // handle normal view
            if self.selected_index < count - 1 {
                self.focus_nth(self.selected_index + 1, cx);
            }
        }
    }
    pub(super) fn focus_prev(&mut self, _: &FocusPrev, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.data.read(cx).len();
        if count == 0 {
            return;
        }

        if let Some(idx) = self.context_idx {
            // handle context
            if idx > 0 {
                self.context_idx = Some(idx - 1);
                cx.notify();
            }
        } else {
            // handle normal view
            if self.selected_index > 0 {
                self.focus_nth(self.selected_index - 1, cx);
            }
        }
    }
    pub(super) fn next_var(&mut self, _: &NextVar, win: &mut Window, cx: &mut Context<Self>) {
        let total_inputs = 1 + self.variable_input.len();

        if self.active_bar < total_inputs - 1 {
            self.active_bar += 1;

            if self.active_bar == 0 {
                self.text_input.read(cx).focus_handle.focus(win);
            } else {
                // handle switching forward
                let var_idx = self.active_bar - 1;
                let Some(active_bar) = self.variable_input.get(var_idx) else {
                    return;
                };
                let handle = active_bar.read(cx).focus_handle.clone();
                handle.focus(win);

                // handle switching back if variable input is empty
                let sub = Some(cx.subscribe(
                    &active_bar.clone(),
                    |_this, _entity, _ev: &EmptyBackspace, cx| {
                        cx.dispatch_action(&PrevVar);
                    },
                ));
                active_bar.update(cx, |var_input, _| {
                    var_input._sub = sub;
                });
            }

            cx.notify();
        }
    }

    pub(super) fn prev_var(&mut self, _: &PrevVar, win: &mut Window, cx: &mut Context<Self>) {
        if self.active_bar > 0 {
            self.active_bar -= 1;

            if self.active_bar == 0 {
                self.text_input.read(cx).focus_handle.focus(win);
            } else {
                let var_idx = self.active_bar - 1;
                let handle = self.variable_input[var_idx].read(cx).focus_handle.clone();
                handle.focus(win);
            }

            cx.notify();
        }
    }
    pub(self) fn execute_helper(
        &mut self,
        what: ExecMode,
        keyword: &str,
        variables: &[(SharedString, SharedString)],
        cx: &mut Context<Self>,
    ) -> Result<bool, SherlockError> {
        match what {
            ExecMode::App { exec, terminal } => {
                let cmd = if terminal {
                    format!(r#"{{terminal}} {exec}"#)
                } else {
                    exec.to_string()
                };

                spawn_detached(&cmd, keyword, variables)?;
                increment(&exec);
            }
            ExecMode::Category { category } => {
                self.mode = category;
                self.text_input.update(cx, |this, _cx| {
                    this.reset();
                });
                self.filter_and_sort(cx);
                cx.notify();
                return Ok(false);
            }
            ExecMode::Commmand { exec } => {
                spawn_detached(&exec, keyword, variables)?;
                increment(&exec);
            }
            ExecMode::CreateBookmark { url, name } => {

            }
            ExecMode::Copy { content } => {
                cx.write_to_clipboard(ClipboardItem::new_string(content.to_string()));
            }
            ExecMode::Web {
                engine,
                browser,
                exec,
            } => {
                let engine = engine.as_deref().unwrap_or("plain");
                let query = if let Some(query) = exec.as_deref() {
                    query
                } else {
                    keyword
                };
                websearch(engine, query, browser.as_deref(), variables)?;
            }
            _ => {}
        };

        Ok(true)
    }
    pub(super) fn execute(&mut self, _: &Execute, win: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.context_idx {
            if let Some(action) = self.context_actions.get(idx) {
                if let Some(selected) = self
                    .data
                    .read(cx)
                    .get(self.filtered_indices[self.selected_index])
                {
                    let what = selected.build_action_exec(action);

                    match self.execute_helper(what, "", &[], cx) {
                        Ok(exit) if exit => self.close_window(win, cx),
                        Err(e) => cx.emit(LauncherErrorEvent::Push(e)),
                        _ => {}
                    }
                }
            }
        } else {
            let keyword = self.text_input.read(cx).content.clone();
            // collect variables
            let mut variables: SmallVec<[(SharedString, SharedString); 4]> = SmallVec::new();
            for s in &self.variable_input {
                let guard = s.read(cx);
                let mut content = guard.content.to_string();

                // Only transform if it's a PathInput
                if let Some(ExecVariable::PathInput(_)) = &guard.variable {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                    if content.starts_with('~') {
                        content = content.replacen('~', &home, 1);
                    } else if !content.starts_with('/') {
                        let mut p = PathBuf::from(home);
                        p.push(&content);
                        content = p.to_string_lossy().to_string();
                    }
                }

                variables.push((guard.placeholder.clone(), SharedString::from(content)));
            }

            let data = self.data.read(cx).clone();
            if let Some(selected) = data.get(self.filtered_indices[self.selected_index]) {
                if let Some(what) = selected.build_exec() {
                    match self.execute_helper(what, keyword.as_ref(), &variables, cx) {
                        Ok(exit) if exit => {
                            self.close_window(win, cx);
                            return;
                        }
                        Err(e) => {
                            cx.emit(LauncherErrorEvent::Push(e));
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    pub(super) fn open_context(
        &mut self,
        _: &OpenContext,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_actions.is_empty() {
            return;
        }

        // toggle logic
        if self.context_idx.take().is_none() {
            self.context_idx = Some(0);
        }

        cx.notify();
    }
    pub(super) fn close_context(&mut self, cx: &mut Context<Self>) {
        if let Some(_) = self.context_idx.take() {
            cx.notify();
        }
    }
    pub(super) fn quit(&mut self, _: &Quit, win: &mut Window, cx: &mut Context<Self>) {
        if self.context_idx.is_some() {
            self.close_context(cx);
        } else {
            self.close_window(win, cx);
        }
    }
    pub(super) fn close_window(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        // Cleanup
        self.variable_input.clear();
        self.filtered_indices = Arc::new([]);
        if let Some(task) = self.deferred_render_task.take() {
            drop(task)
        }

        // Close window
        win.remove_window();

        // Propagate state change
        cx.notify();
    }
    pub(super) fn update_vars(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.filtered_indices.get(self.selected_index).copied() else {
            return;
        };

        let needed_vars: Option<Vec<ExecVariable>> = {
            let data_guard = self.data.read(cx);
            data_guard
                .get(idx)
                .and_then(|data| data.vars().map(|slice| slice.to_vec()))
        };

        if let Some(vars_to_create) = needed_vars {
            self.variable_input = vars_to_create
                .into_iter()
                .map(|var| {
                    cx.new(|cx| {
                        TextInput::builder()
                            .scope("variable")
                            .placeholder(var.placeholder())
                            .variable(var)
                            .build(cx)
                    })
                })
                .collect();
        } else {
            self.variable_input.clear();
        }
    }
}

#[inline(always)]
fn increment(key: &str) {
    if let Ok(count_reader) = CounterReader::new() {
        let _ = count_reader.increment(key);
    };
}
