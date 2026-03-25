use std::sync::Arc;

use gpui::{
    AnyEntity, App, AppContext, Entity, ListState, ScrollStrategy, UniformListScrollHandle, px,
};
use simd_json::prelude::Indexed;

use crate::{
    launcher::{
        Launcher,
        children::{RenderableChild, RenderableChildDelegate},
    },
    loader::utils::ApplicationAction,
    ui::model::{Model, emoji::EmojiView, home::HomeView},
};

pub struct NavigationStack {
    stack: Vec<NavigationView>,
}

impl NavigationStack {
    pub fn new(initial: Entity<Arc<Vec<RenderableChild>>>, len: usize, cx: &mut App) -> Self {
        let home = NavigationView {
            view: cx.new(|cx| HomeView::new(initial, cx)).into(),
            style: EntityStyle::Row {
                state: ListState::new(len, gpui::ListAlignment::Top, px(48.)),
                selected_index: 0,
            },
            kind: NavigationViewType::Home,
        };
        Self { stack: vec![home] }
    }
}

impl NavigationStack {
    pub fn clear(&mut self) {
        self.stack.truncate(1);
    }
    pub fn len(&self) -> usize {
        self.stack.len()
    }
    pub fn push(&mut self, view: NavigationView) {
        self.stack.push(view);
    }
    pub fn pop(&mut self) -> Option<NavigationView> {
        // ensures home view will always stay in view
        if self.stack.len() == 1 {
            return None;
        }

        self.stack.pop()
    }
    pub fn current(&self) -> &NavigationView {
        // Since we ensure to always keep the stack populated with at least the home item, this is
        // safe
        self.stack
            .last()
            .expect("NavigationStack must always contain a root view.")
    }
    pub fn current_mut(&mut self) -> &mut NavigationView {
        // Since we ensure to always keep the stack populated with at least the home item, this is
        // safe
        self.stack
            .last_mut()
            .expect("NavigationStuck must always contain a root view.")
    }
    pub fn with_model<R>(&self, cx: &mut App, f: impl FnOnce(&Model) -> R) -> R {
        let current = self.current();
        match current.kind {
            NavigationViewType::Home => {
                let view = current.view.clone().downcast::<HomeView>().unwrap();
                f(&view.read(cx).model)
            }

            NavigationViewType::Emoji => {
                let view = current.view.clone().downcast::<EmojiView>().unwrap();
                f(&view.read(cx).model)
            }
        }
    }
    pub fn with_model_mut<R, C: AppContext>(
        &self,
        cx: &mut C,
        f: impl FnOnce(&mut Model, &mut App) -> R,
    ) -> R {
        let current = self.current();
        match current.kind {
            NavigationViewType::Home => {
                let view = current.view.clone().downcast::<HomeView>().unwrap();
                view.update(cx, |this, cx| f(&mut this.model, cx))
            }

            NavigationViewType::Emoji => {
                let view = current.view.clone().downcast::<EmojiView>().unwrap();
                view.update(cx, |this, cx| f(&mut this.model, cx))
            }
        }
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.current().style.selected_index()
    }
    pub fn selected_item_index(&self, cx: &mut App) -> Option<usize> {
        let current = self.current();
        let ui_idx = current.style.selected_index()?;

        self.with_model(cx, |mdl| {
            if mdl.filtered_indices.is_empty() {
                return None;
            }

            let safe_idx = ui_idx.min(mdl.filtered_indices.len() - 1);
            mdl.filtered_indices.get(safe_idx).copied()
        })
    }
    pub fn selected_item(&self, cx: &mut App) -> Option<RenderableChild> {
        let ui_idx = self.current().style.selected_index()?;
        let (data_idx, data_entity) = self.with_model(cx, |mdl| {
            if mdl.filtered_indices.is_empty() {
                return (None, mdl.data.clone());
            }

            let safe_ui_idx = ui_idx.min(mdl.filtered_indices.len() - 1);

            (
                mdl.filtered_indices.get(safe_ui_idx).copied(),
                mdl.data.clone(),
            )
        });

        let idx = data_idx?;
        data_entity.read(cx).get(idx).cloned()
    }
    pub fn current_actions(&self, cx: &mut App) -> Option<Arc<[Arc<ApplicationAction>]>> {
        let ui_idx = self.current().style.selected_index()?;
        let (data_idx, data_entity) = self.with_model(cx, |mdl| {
            if mdl.filtered_indices.is_empty() {
                return (None, mdl.data.clone());
            }

            let safe_ui_idx = ui_idx.min(mdl.filtered_indices.len() - 1);

            (
                mdl.filtered_indices.get(safe_ui_idx).copied(),
                mdl.data.clone(),
            )
        });

        let idx = data_idx?;

        data_entity.read(cx).get(idx).and_then(|i| i.actions())
    }
}

#[derive(Clone)]
pub struct NavigationView {
    pub view: AnyEntity,
    pub style: EntityStyle,
    pub kind: NavigationViewType,
}

impl NavigationView {
    pub fn reset_selected_index(&mut self) {
        match &mut self.style {
            EntityStyle::Row { selected_index, .. } => {
                *selected_index = 0;
            }
            EntityStyle::Grid { selected_index, .. } => {
                *selected_index = 0;
            }
        }
    }
}

#[derive(Clone)]
pub enum EntityStyle {
    Grid {
        state: ListState,
        selected_index: usize,
        columns: usize,
        scroll_handle: UniformListScrollHandle,
    },
    Row {
        state: ListState,
        selected_index: usize,
    },
}

pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}
impl EntityStyle {
    pub fn selected_index(&self) -> Option<usize> {
        match self {
            Self::Grid { selected_index, .. } => Some(*selected_index),
            Self::Row { selected_index, .. } => Some(*selected_index),
        }
    }

    pub fn list_state(&self) -> Option<&ListState> {
        match self {
            Self::Grid { state, .. } => Some(state),
            Self::Row { state, .. } => Some(state),
        }
    }

    pub fn next_index(&self, direction: MoveDirection) -> Option<usize> {
        let current = self.selected_index()?;

        match self {
            EntityStyle::Row { .. } => match direction {
                MoveDirection::Down => Some(current + 1),
                MoveDirection::Up => current.checked_sub(1),
                _ => None,
            },
            EntityStyle::Grid { columns, .. } => {
                let cols = *columns;
                match direction {
                    MoveDirection::Down => Some(current + cols),
                    MoveDirection::Up => current.checked_sub(cols),
                    MoveDirection::Right => Some(current + 1),
                    MoveDirection::Left => current.checked_sub(1),
                }
            }
        }
    }

    /// Sets internal focus to the first item of the view.
    ///
    /// # Returns:
    ///
    /// None: if the selected view is not a list type
    /// Some(()): if the item was selected
    pub fn focus_nth(&mut self, n: usize) -> Option<()> {
        match self {
            Self::Row {
                state,
                selected_index,
            } => {
                *selected_index = n;
                state.scroll_to_reveal_item(n);
                Some(())
            }
            Self::Grid {
                columns,
                scroll_handle,
                selected_index,
                ..
            } => {
                *selected_index = n;
                let cols = *columns;
                if cols > 0 {
                    let row_index = n / cols;
                    scroll_handle.scroll_to_item(row_index, ScrollStrategy::Nearest);
                }
                Some(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationViewType {
    Emoji,
    Home,
}

impl NavigationViewType {
    pub fn create_view(&self, launcher: Arc<Launcher>, cx: &mut App) -> NavigationView {
        match self {
            Self::Emoji => {
                let view = cx.new(|cx| EmojiView::new(launcher, cx));
                let count = view.read(cx).model.len();
                NavigationView {
                    view: view.into(),
                    style: EntityStyle::Grid {
                        columns: 6,
                        state: ListState::new(count, gpui::ListAlignment::Top, px(100.)),
                        scroll_handle: UniformListScrollHandle::new(),
                        selected_index: 0,
                    },
                    kind: *self,
                }
            }

            Self::Home => {
                // This is not implemented because the initial plugin should be implemented
                // manually because its not dependent on a launcher
                unimplemented!()
            }
        }
    }
}
