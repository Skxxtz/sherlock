use gpui::{
    AnyEntity, App, AppContext, ListState, ScrollStrategy, SharedString, UniformListScrollHandle,
    px,
};
use simd_json::prelude::{ArrayTrait, Indexed};
use std::sync::Arc;

use crate::{
    app::RenderableChildEntity,
    launcher::Launcher,
    ui::{
        launcher::context_menu::ContextMenuAction,
        model::{
            Model, emoji::EmojiView, file::view::FileView, home::HomeView, message::MessageView,
        },
        widgets::{LauncherValues, RenderableChild, RenderableChildDelegate},
    },
    utils::errors::SherlockMessage,
};

/// The number of views that have to remain in the NavigationStack.
///
/// **Example:**
/// `[Errors, Home, .. ]` => **2** Persistent views
static REMAINING_VIEWS: usize = 2;

pub struct NavigationStack {
    stack: Vec<NavigationView>,
    active_idx: Option<usize>,
}

impl NavigationStack {
    pub fn new(
        initial: RenderableChildEntity,
        messages: Vec<SherlockMessage>,
        len: usize,
        cx: &mut App,
    ) -> Self {
        let home = NavigationView {
            view: cx.new(|cx| HomeView::new(initial, cx)).into(),
            style: EntityStyle::Row {
                state: ListState::new(len, gpui::ListAlignment::Top, px(48.)),
                selected_index: 0,
            },
            kind: NavigationViewType::Home,
        };
        let message_len = messages.len();
        let errors = NavigationView {
            view: cx.new(|cx| MessageView::new(messages, cx)).into(),
            style: EntityStyle::Row {
                state: ListState::new(message_len, gpui::ListAlignment::Top, px(100.)),
                selected_index: 0,
            },
            kind: NavigationViewType::Message,
        };

        Self {
            stack: vec![errors, home],
            active_idx: None,
        }
    }
}

impl NavigationStack {
    #[inline(always)]
    fn get_message_view(&self) -> Option<&NavigationView> {
        self.stack
            .iter()
            .find(|s| s.kind == NavigationViewType::Message)
    }
    pub fn push_message(&self, message: SherlockMessage, cx: &mut App) {
        if let Some(message_view) = self.get_message_view() {
            let view = message_view.view.clone().downcast::<MessageView>().unwrap();
            let weak_entity = view.downgrade();

            view.update(cx, |this, cx| {
                this.push_message(message, weak_entity, cx);
            });
        }
    }
    pub fn set_messages_active(&mut self) {
        self.active_idx = self
            .stack
            .iter()
            .position(|view| view.kind == NavigationViewType::Message);
    }
    pub fn message_count(&self, cx: &mut App) -> usize {
        if let Some(view) = self.get_message_view() {
            view.view
                .clone()
                .downcast::<MessageView>()
                .unwrap()
                .read(cx)
                .count
                .get()
        } else {
            0
        }
    }

    pub fn set_prev_and_cleanup(&mut self) {
        if self.active_idx.take().is_none() {
            self.pop();
        }
        self.current_mut().reset_selected_index();
    }
}

impl NavigationStack {
    pub fn set_active_idx(&mut self, idx: usize) -> bool {
        if self.stack.len() > idx {
            self.active_idx = Some(idx);
            true
        } else {
            false
        }
    }
    pub fn current_kind(&self) -> NavigationViewType {
        self.current().kind.clone()
    }
    pub fn clear(&mut self) {
        self.stack.truncate(REMAINING_VIEWS);
    }
    pub fn push(&mut self, view: NavigationView) {
        self.stack.push(view);
    }
    pub fn pop(&mut self) -> Option<NavigationView> {
        // ensures home view will always stay in view
        if self.stack.len() == REMAINING_VIEWS {
            return None;
        }

        self.stack.pop()
    }
    pub fn current(&self) -> &NavigationView {
        // Since we ensure to always keep the stack populated with at least the home item, this is
        // safe
        self.active_idx
            .and_then(|idx| self.stack.get(idx))
            .or(self.stack.last())
            .expect("NavigationStack must always contain a root view.")
    }
    pub fn current_mut(&mut self) -> &mut NavigationView {
        if let Some(idx) = self.active_idx
            && idx < self.stack.len()
        {
            return &mut self.stack[idx];
        }

        // Since we ensure to always keep the stack populated with at least the home item, this is
        // safe
        self.stack
            .last_mut()
            .expect("NavigationStack must always contain a root view.")
    }
    pub fn with_model<R>(&self, cx: &mut App, f: impl FnOnce(&Model) -> R) -> R {
        let current = self.current();
        match current.kind {
            NavigationViewType::Files { .. } => {
                let view = current.view.clone().downcast::<FileView>().unwrap();
                f(&view.read(cx).model)
            }

            NavigationViewType::Home => {
                let view = current.view.clone().downcast::<HomeView>().unwrap();
                f(&view.read(cx).model)
            }

            NavigationViewType::Message => {
                let view = current.view.clone().downcast::<MessageView>().unwrap();
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
            NavigationViewType::Files { .. } => {
                let view = current.view.clone().downcast::<FileView>().unwrap();
                view.update(cx, |this, cx| f(&mut this.model, cx))
            }

            NavigationViewType::Home => {
                let view = current.view.clone().downcast::<HomeView>().unwrap();
                view.update(cx, |this, cx| f(&mut this.model, cx))
            }

            NavigationViewType::Message => {
                let view = current.view.clone().downcast::<MessageView>().unwrap();
                view.update(cx, |this, cx| f(&mut this.model, cx))
            }

            NavigationViewType::Emoji => {
                let view = current.view.clone().downcast::<EmojiView>().unwrap();
                view.update(cx, |this, cx| f(&mut this.model, cx))
            }
        }
    }
    pub fn selected_item_index(&self, cx: &mut App) -> Option<usize> {
        let current = self.current();
        let ui_idx = current.style.selected_index()?;

        self.with_model_mut(cx, |mdl, cx| {
            if mdl.data().read(cx).is_empty() {
                return None;
            }

            let filtered_indices = mdl.filtered_indices();
            let safe_idx = ui_idx.min(filtered_indices.len() - 1);
            filtered_indices.get(safe_idx).copied()
        })
    }
    pub fn selected_item(&self, cx: &mut App) -> Option<RenderableChild> {
        self.with_selected_item(cx, |selected, _| selected.cloned())
    }
    pub fn with_selected_item<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(Option<&RenderableChild>, &mut App) -> Option<R>,
    ) -> Option<R> {
        let ui_idx = self.current().style.selected_index()?;
        let (data_idx, data_entity) = self.with_model_mut(cx, |mdl, cx| {
            let data = mdl.data();
            if data.read(cx).is_empty() {
                return (None, data);
            }

            let filtered_indices = mdl.filtered_indices();
            if filtered_indices.is_empty() {
                return (None, mdl.data());
            }

            let safe_ui_idx = ui_idx.min(filtered_indices.len() - 1);

            (filtered_indices.get(safe_ui_idx).copied(), mdl.data())
        });
        let idx = data_idx?;
        data_entity.update(cx, |data, cx| f(data.get(idx), cx))
    }
    pub fn with_nth_shortcut_item<R>(
        &self,
        idx: usize,
        cx: &mut App,
        f: impl FnOnce(Option<&RenderableChild>, &mut App) -> Option<R>,
    ) -> Option<R> {
        let (data_idx, data_entity) = self.with_model_mut(cx, |mdl, cx| {
            let data_entity = mdl.data();
            let data = data_entity.read(cx);
            if data.is_empty() || data.len() < idx {
                return (None, data_entity);
            }

            let filtered_indices = mdl.filtered_indices();
            if filtered_indices.is_empty() || filtered_indices.len() < idx {
                return (None, data_entity);
            }

            let item_idx = filtered_indices
                .iter()
                .copied()
                .filter(|i| data.get(*i).is_some_and(|item| item.shortcut()))
                .nth(idx - 1);

            (item_idx, data_entity)
        });

        let idx = data_idx?;
        data_entity.update(cx, |data, cx| f(data.get(idx), cx))
    }
    pub fn current_actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        self.with_selected_item(cx, |item, cx| item.and_then(|i| i.actions(cx)))
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
                ..
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
                if let Some(row_index) = n.checked_div(cols) {
                    scroll_handle.scroll_to_item(row_index, ScrollStrategy::Nearest);
                }
                Some(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationViewType {
    Emoji,
    Message,
    Home,
    Files { dir: Option<SharedString> },
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
                    kind: self.clone(),
                }
            }
            Self::Files { dir } => {
                let view = cx.new(|cx| FileView::new(launcher, dir.clone(), cx));
                NavigationView {
                    view: view.into(),
                    style: EntityStyle::Row {
                        state: ListState::new(0, gpui::ListAlignment::Top, px(50.)),
                        selected_index: 0,
                    },
                    kind: self.clone(),
                }
            }
            Self::Message | Self::Home => {
                // This is not implemented because the initial views should be implemented
                // manually because its not dependent on a launcher
                unimplemented!()
            }
        }
    }
}
