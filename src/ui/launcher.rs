use crate::app::{RenderableChildEntity, RenderableChildWeak};
use crate::launcher::Launcher;
use crate::launcher::variant_type::LauncherType;
use crate::ui::launcher::context_menu::ContextMenuAction;
use crate::ui::launcher::views::{NavigationStack, NavigationViewType};
use crate::ui::model::Model;
use crate::ui::utils::scoring::make_prio;
use crate::ui::utils::search::SherlockSearch;
use crate::ui::widgets::{LauncherValues, RenderableChildDelegate};
use crate::utils::config::HomeType;
use gpui::WeakEntity;
use gpui::{App, Context, Entity, FocusHandle, Focusable, SharedString, Subscription};
use gpui::{AsyncApp, Task};
use std::sync::Arc;

use crate::ui::search_bar::TextInput;

pub mod actions;
pub mod context_menu;
pub mod render;
pub mod views;

pub use actions::{
    Execute, NextVar, OpenContext, PrevVar, Quit, SelectionDown, SelectionLeft, SelectionRight,
    SelectionUp,
};

pub struct LauncherView {
    pub text_input: Entity<TextInput>,
    pub focus_handle: FocusHandle,
    pub _subs: Vec<Subscription>,

    // mode
    pub mode: LauncherMode,
    pub modes: Arc<[LauncherMode]>,

    // context menu
    pub context_idx: Option<usize>,
    pub context_actions: Arc<[Arc<ContextMenuAction>]>,
    pub has_actions: bool,

    // variable input fields
    pub variable_input: Vec<Entity<TextInput>>,
    pub active_bar: usize,

    // Model
    pub navigation: NavigationStack,

    // State
    pub config_initialized: bool,

    pub active_update_task: Option<Task<()>>,
}

impl Focusable for LauncherView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl LauncherView {
    pub fn apply_results(
        &mut self,
        results: Arc<[usize]>,
        query: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.navigation.current().style.list_state() else {
            return;
        };

        let mut changed = false;
        let query: SharedString = query.into();

        let old_count = state.item_count();
        let new_count = results.len();
        if old_count != new_count {
            state.splice(0..old_count, new_count);
        }

        self.active_bar = 0;
        self.navigation.with_model_mut(cx, |mdl, _| match mdl {
            Model::Standard {
                filtered_indices: idx,
                last_query: q,
                ..
            }
            | Model::FileSearch {
                filtered_indices: idx,
                last_query: q,
                ..
            } => {
                if idx != &results {
                    changed = true;
                    *idx = results;
                }
                *q = Some(query.clone());
            }
        });

        self.update_sync(query, cx);

        if changed {
            self.update_vars(cx);
            self.focus_first(cx);
        }

        cx.notify();
    }
    pub fn filter_and_sort(&mut self, cx: &mut Context<Self>) {
        let mut query: SharedString = self.text_input.read(cx).content.to_lowercase().into();

        // handle mode change
        match self.mode.transition_for_query(&query, &self.modes) {
            ModeTransition::None => {}
            ModeTransition::ClearInput => {
                self.text_input.update(cx, |this, _cx| {
                    this.reset();
                });
                query = "".into();
            }
            ModeTransition::PushStack(launcher) => {
                let view = match &launcher.launcher_type {
                    LauncherType::Emoji(_) => NavigationViewType::Emoji,
                    LauncherType::Files(files) => NavigationViewType::Files {
                        dir: Some(files.loc.clone()),
                    },
                    _ => return,
                };

                self.text_input.update(cx, |this, _| this.reset());
                self.navigation.push(view.create_view(launcher, cx));
                query = "".into();
            }
        }

        enum ModelKind {
            FileSearch {
                weak_data: RenderableChildWeak,
                last_query: Option<SharedString>,
            },
            Standard {
                data: RenderableChildEntity,
            },
        }

        let kind = self.navigation.with_model(cx, |mdl| match mdl {
            Model::FileSearch {
                data, last_query, ..
            } => ModelKind::FileSearch {
                weak_data: data.downgrade(),
                last_query: last_query.clone(),
            },
            Model::Standard { data, .. } => ModelKind::Standard { data: data.clone() },
        });

        match kind {
            ModelKind::FileSearch {
                weak_data,
                last_query,
            } => {
                if last_query.is_some_and(|s| s == query) {
                    return;
                }

                let weak_self = cx.entity().downgrade();
                self.navigation.with_model_mut(cx, |mdl, cx| {
                    if let Model::FileSearch { search, .. } = mdl {
                        search.search(query.into(), weak_data, weak_self, cx);
                    }
                });
            }
            ModelKind::Standard { data } => {
                // drop active tasks
                self.navigation.with_model_mut(cx, |mdl, _| {
                    if let Model::Standard {
                        deferred_render_task,
                        ..
                    } = mdl
                    {
                        *deferred_render_task = None;
                    }
                });

                let mode = self.mode.clone();
                let data_arc = data.read(cx).clone();
                let render_task = Some(cx.spawn(
                    |this: WeakEntity<LauncherView>, cx: &mut AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            let mode = mode.as_str();
                            let is_home = query.is_empty() && mode == "all";

                            // collects Vec<(index, priority)>
                            let mut results: Vec<(usize, f32)> = (0..data_arc.len())
                                .map(|i| (i, &data_arc[i]))
                                .filter(|(_, data)| {
                                    let home = data.home();
                                    // [Rule 1]
                                    // Case 1: Early return if mode applies but item is not assigned to that mode
                                    // Case 2: Early return if current mode is not required mode for item
                                    if Some(mode) != data.alias()
                                        && (mode != "all" || data.priority() < 1.0)
                                    {
                                        return false;
                                    }

                                    // [Rule 2]
                                    // Early return if item should always show (websearch for example)
                                    if home == HomeType::Persist {
                                        return true;
                                    }

                                    // [Rule 3]
                                    // Early return if not home but item is assigned to only show on home
                                    if !is_home && home == HomeType::OnlyHome {
                                        return false;
                                    }

                                    // [Rule 4]
                                    // Early return if based show (calc for example) applies
                                    if let Some(based) = data.based_show(&query, &mut cx) {
                                        return based;
                                    }

                                    // [Rule 5]
                                    // Early return if item should only show on search but mode is home
                                    if is_home && home == HomeType::Search {
                                        return false;
                                    }

                                    // [Rule 6]
                                    // Check if query matches
                                    data.search().fuzzy_match(&query)
                                })
                                .map(|(i, data)| {
                                    let prio = make_prio(data.priority(), &query, data.search());
                                    (i, prio)
                                })
                                .collect();

                            // drop here to release lock faster
                            drop(data_arc);

                            // sort based on priority
                            results.sort_unstable_by(|a, b| {
                                a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                            });

                            // strip the priority from results
                            let results_arc: Arc<[usize]> = results
                                .into_iter()
                                .map(|(i, _)| i)
                                .collect::<Vec<_>>()
                                .into();

                            this.update(&mut cx, |this, cx| {
                                this.apply_results(results_arc, query, cx);
                            })
                            .ok();

                            Some(())
                        }
                    },
                ));

                // set active render task
                self.navigation.with_model_mut(cx, |mdl, _| {
                    if let Model::Standard {
                        deferred_render_task,
                        ..
                    } = mdl
                    {
                        *deferred_render_task = render_task;
                    }
                })
            }
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum LauncherMode {
    Home,
    Search,
    Alias {
        short: SharedString,
        name: SharedString,
        launcher: Arc<Launcher>,
    },
}

pub enum ModeTransition {
    None,
    PushStack(Arc<Launcher>),
    ClearInput,
}

impl LauncherMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Home | Self::Search => "all",
            Self::Alias { short, .. } => short.as_ref(),
        }
    }
    pub fn display_str(&self) -> SharedString {
        match self {
            // "".into() uses static literals (no allocation) → efficient
            Self::Home => "All".into(),
            Self::Search => "Search".into(),
            Self::Alias { name, .. } => name.clone(),
        }
    }
    pub fn transition_for_query(&mut self, query: &str, modes: &[Self]) -> ModeTransition {
        match (self, query.is_empty()) {
            (m @ Self::Search, true) => *m = Self::Home,
            (m @ Self::Home, false) => *m = Self::Search,
            (m @ Self::Search, false) | (m @ Self::Alias { .. }, false) => {
                if let Some(alias_input) = query.strip_suffix(' ') {
                    let found_mode = modes.iter().find(|mode| {
                        if let Self::Alias { short, .. } = mode {
                            short.eq_ignore_ascii_case(alias_input)
                        } else {
                            false
                        }
                    });

                    if let Some(new_mode) = found_mode {
                        *m = new_mode.clone();
                        if let Self::Alias { launcher, .. } = new_mode
                            && matches!(
                                &launcher.launcher_type,
                                LauncherType::Files(_) | LauncherType::Emoji(_)
                            )
                        {
                            return ModeTransition::PushStack(launcher.clone());
                        }
                        // should clear search bar
                        return ModeTransition::ClearInput;
                    }
                } else {
                    *m = Self::Search;
                }
            }
            _ => {}
        }

        // only minor change
        ModeTransition::None
    }
}
