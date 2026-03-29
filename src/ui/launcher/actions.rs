use std::{path::PathBuf, sync::Arc};

use futures::{StreamExt, stream::FuturesUnordered};
use gpui::{
    AppContext, AsyncApp, ClipboardItem, Context, Focusable, SharedString, Window, actions,
};
use simd_json::prelude::Indexed;
use smallvec::SmallVec;

use crate::{
    launcher::{
        ExecMode,
        children::{LauncherValues, RenderableChildDelegate, emoji_data::set_selected_skin_tone},
    },
    loader::utils::{CounterReader, ExecVariable},
    sherlock_msg,
    ui::{
        launcher::{LauncherView, context_menu::ContextMenuAction, views::MoveDirection},
        search_bar::{EmptyBackspace, TextInput},
    },
    utils::{
        command_launch::spawn_detached,
        errors::{SherlockMessage, types::SherlockErrorType},
        websearch::websearch,
    },
};

actions!(
    example_input,
    [
        Quit,
        SelectionDown,
        SelectionUp,
        SelectionLeft,
        SelectionRight,
        NextVar,
        PrevVar,
        Execute,
        OpenContext,
        Backspace,
    ]
);

impl LauncherView {
    pub fn focus_first(&mut self, cx: &mut Context<Self>) {
        let snapshot = self.navigation.with_model(cx, |mdl| {
            let filtered_indices = mdl.filtered_indices();
            if filtered_indices.is_empty() {
                None
            } else {
                Some((filtered_indices, mdl.data()))
            }
        });

        let Some((indices, data_entity)) = snapshot else {
            return;
        };

        // Find the first focusable item
        let first_valid_index = {
            let data_guard = data_entity.read(cx);
            indices.iter().position(|&idx| {
                data_guard
                    .get(idx)
                    .map_or(false, |child| child.spawn_focus())
            })
        };

        if let Some(n) = first_valid_index {
            self.focus_nth(n, cx);
        }
    }
    #[inline(always)]
    fn valid_selection_idx(&self, n: usize, cx: &mut Context<Self>) -> bool {
        let list_len = self.navigation.with_model(cx, |mdl| mdl.len());
        n < list_len && list_len != 0
    }
    pub fn focus_nth(&mut self, n: usize, cx: &mut Context<Self>) {
        // early return on invalid index
        if !self.valid_selection_idx(n, cx) {
            return;
        }

        // early return if not a list view
        if self.navigation.current_mut().style.focus_nth(n).is_none() {
            return;
        };

        // Handle variable inputs
        self.update_vars(cx);
        self.active_bar = 0;

        // Handle context menu entries
        self.has_actions = self
            .navigation
            .selected_item(cx)
            .map_or(false, |i| i.has_actions());

        cx.notify()
    }
    fn move_selection(&mut self, direction: MoveDirection, cx: &mut Context<Self>) {
        if let Some(idx) = self.context_idx {
            match direction {
                MoveDirection::Down => {
                    if idx < self.context_actions.len().saturating_sub(1) {
                        self.context_idx = Some(idx + 1);
                    }
                }
                MoveDirection::Up => {
                    if idx > 0 {
                        self.context_idx = Some(idx - 1);
                    }
                }
                MoveDirection::Left => {
                    if let Some(action_arc) = self.context_actions.get(idx) {
                        if let ContextMenuAction::Emoji(act) = action_arc.as_ref() {
                            act.update_index(|i| {
                                set_selected_skin_tone((i - 1).into(), act.for_tone as usize);
                                i.saturating_sub(1)
                            });
                        } else if idx > 0 {
                            self.context_idx = Some(idx - 1);
                        }
                    }
                }
                MoveDirection::Right => {
                    if let Some(action_arc) = self.context_actions.get(idx) {
                        if let ContextMenuAction::Emoji(act) = action_arc.as_ref() {
                            act.update_index(|i| {
                                let new = (i + 1).clamp(0, 5);
                                set_selected_skin_tone(new.into(), act.for_tone as usize);
                                new
                            });
                        } else if idx < self.context_actions.len().saturating_sub(1) {
                            self.context_idx = Some(idx + 1);
                        }
                    }
                }
            }
            cx.notify();
            return;
        }

        let current_style = &mut self.navigation.current_mut().style;

        if let Some(target_idx) = current_style.next_index(direction) {
            if self.valid_selection_idx(target_idx, cx) {
                self.focus_nth(target_idx, cx);
            }
        }
    }
    pub(super) fn selection_down(
        &mut self,
        _: &SelectionDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_selection(MoveDirection::Down, cx);
    }

    pub(super) fn selection_up(&mut self, _: &SelectionUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(MoveDirection::Up, cx);
    }

    pub(super) fn selection_left(
        &mut self,
        _: &SelectionLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_selection(MoveDirection::Left, cx);
    }

    pub(super) fn selection_right(
        &mut self,
        _: &SelectionRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_selection(MoveDirection::Right, cx);
    }

    pub(super) fn next_var(&mut self, _: &NextVar, win: &mut Window, cx: &mut Context<Self>) {
        let total_inputs = 1 + self.variable_input.len();

        if self.active_bar < total_inputs - 1 {
            self.active_bar += 1;

            if self.active_bar == 0 {
                self.text_input.focus_handle(cx).focus(win, cx);
            } else {
                // handle switching forward
                let var_idx = self.active_bar - 1;
                let Some(active_bar) = self.variable_input.get(var_idx) else {
                    return;
                };
                active_bar.focus_handle(cx).focus(win, cx);

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
                self.text_input.focus_handle(cx).focus(win, cx);
            } else {
                let var_idx = self.active_bar - 1;
                self.variable_input[var_idx].focus_handle(cx).focus(win, cx);
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
    ) -> Result<bool, SherlockMessage> {
        match what {
            ExecMode::Inner { func, exit } => {
                if let Some(item) = self.navigation.selected_item(cx) {
                    let _was_executed = item.launcher_type().execute_function(func, &item)?;
                    self.update_async(cx);
                    return Ok(exit);
                }
            }
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
            ExecMode::CreateBookmark { url, name } => {}
            ExecMode::Copy { content } => {
                cx.write_to_clipboard(ClipboardItem::new_string(content.to_string()));
            }
            ExecMode::CreateView { mode, launcher } => {
                self.text_input.update(cx, |this, _| this.reset());
                self.navigation.push(mode.create_view(launcher, cx));
                self.filter_and_sort(cx);
                return Ok(false);
            }
            ExecMode::DynamicContextMenuFunc { action } => {
                let ContextMenuAction::Fn(opts) = action.as_ref() else {
                    return Ok(false);
                };

                if let Some(func) = opts.func.as_ref() {
                    func(cx)
                }
            }
            ExecMode::SwitchView { idx } => {
                if self.navigation.set_active_idx(idx) {
                    self.text_input.update(cx, |this, _| this.reset());
                    self.filter_and_sort(cx);
                    return Ok(false);
                }
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
    pub(super) fn execute_listener(
        &mut self,
        _: &Execute,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(idx) = self.context_idx {
            if let Some(action) = self.context_actions.get(idx) {
                if let Some(selected) = self.navigation.selected_item(cx) {
                    let what = selected.build_action_exec(Arc::clone(action));

                    match self.execute_helper(what, "", &[], cx) {
                        Ok(exit) if exit => self.close_window(win, cx),
                        Err(e) => self.navigation.push_message(e, cx),
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

            if let Some(selected) = self.navigation.selected_item(cx) {
                if let Some(what) = selected.build_exec() {
                    match self.execute_helper(what, keyword.as_ref(), &variables, cx) {
                        Ok(exit) if exit => {
                            self.close_window(win, cx);
                            return;
                        }
                        Err(e) => {
                            self.navigation.push_message(e, cx);
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    pub(super) fn execute_inner_function(
        &mut self,
        what: ExecMode,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.execute_helper(what, "", &[], cx) {
            Ok(exit) if exit => {
                self.close_window(win, cx);
                return;
            }
            Err(e) => {
                self.navigation.push_message(e, cx);
                return;
            }
            _ => {}
        }
        cx.notify();
    }
    pub(super) fn open_context(
        &mut self,
        _: &OpenContext,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_actions {
            return;
        }

        self.context_actions = self.navigation.current_actions(cx).unwrap_or_default();

        // should never be called unless
        if self.context_actions.is_empty() {
            let launcher_type = self
                .navigation
                .selected_item(cx)
                .map(|i| i.launcher_type().to_owned());

            self.navigation.push_message(
                sherlock_msg!(
                    Error,
                    SherlockErrorType::Unreachable,
                    format!(
                        "Launcher {:?} configured incorrectly: context actions are empty but `has_actions` flag is set to true.",
                        launcher_type
                    )
                ),
                cx,
            );
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
        self.navigation.clear();

        // Close window
        win.remove_window();

        // Propagate state change
        cx.notify();
    }
    pub(super) fn update_vars(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.navigation.selected_item_index(cx) else {
            return;
        };

        let needed_vars: Option<Vec<ExecVariable>> = {
            self.navigation.with_model_mut(cx, |mdl, cx| {
                let data_guard = mdl.data().read(cx);
                data_guard
                    .get(idx)
                    .and_then(|data| data.vars().map(|slice| slice.to_vec()))
            })
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
    pub(crate) fn update_async(&mut self, cx: &mut Context<Self>) {
        let data = self.navigation.with_model(cx, |mdl| mdl.data());
        self.active_update_task = Some(cx.spawn(async move |this, cx: &mut AsyncApp| {
            let items = data.read_with(cx, |this, _| this.clone());

            let mut futures: FuturesUnordered<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.is_async())
                .map(|(idx, item)| async move { (idx, item.clone().update_async().await) })
                .collect();

            while let Some((idx, result)) = futures.next().await {
                let Some(update) = result else { continue };
                let _ = cx.update(|cx| {
                    data.update(cx, |items_arc, _| {
                        Arc::make_mut(items_arc)[idx] = update;
                    });
                    cx.notify(this.entity_id());
                });
            }

            this.upgrade().map(|t| {
                t.update(cx, |this, cx| {
                    this.filter_and_sort(cx);
                })
            });
        }));
    }
}

#[inline(always)]
fn increment(key: &str) {
    if let Ok(count_reader) = CounterReader::new() {
        let _ = count_reader.increment(key);
    };
}
