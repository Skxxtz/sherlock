use gio::glib::{
    object::{CastNone, ObjectExt},
    WeakRef,
};
use gtk4::{prelude::EditableExt, Entry, ListView};
use std::{cell::RefCell, rc::Rc};

use crate::{
    g_subclasses::{action_entry::ContextAction, sherlock_row::SherlockRow, tile_item::TileItem},
    prelude::SherlockNav,
    ui::search::UserBindHandler,
};

use super::util::ContextUI;

pub struct KeyActions {
    pub results: WeakRef<ListView>,
    pub search_bar: WeakRef<Entry>,
    pub context: ContextUI,
    pub custom_handler: Rc<RefCell<UserBindHandler>>,
}
impl KeyActions {
    pub fn new(
        results: WeakRef<ListView>,
        search_bar: WeakRef<Entry>,
        context: ContextUI,
        custom_handler: Rc<RefCell<UserBindHandler>>,
    ) -> Self {
        Self {
            results,
            search_bar,
            context,
            custom_handler,
        }
    }
    pub fn on_multi_return(&self) {
        // no context menu yet
        if self.context.open.get() {
            return;
        }
        if let Some(actives) = self
            .results
            .upgrade()
            .and_then(|r| r.get_actives::<SherlockRow>())
        {
            let len = actives.len();
            actives.into_iter().enumerate().for_each(|(i, row)| {
                let exit: u8 = if i < len - 1 { 1 } else { 0 };
                row.emit_by_name::<()>("row-should-activate", &[&exit, &""]);
            });
        }
    }
    pub fn on_return(&self, context_open: bool, close: Option<bool>) {
        let exit: u8 = close.map_or(0, |v| if v { 2 } else { 1 });
        if context_open {
            // Activate action
            if let Some(upgr) = self.context.view.upgrade() {
                if let Some(row) = upgr.selected_item().and_downcast::<ContextAction>() {
                    row.emit_by_name::<()>("context-action-should-activate", &[&exit]);
                }
            }
        } else {
            // Activate apptile
            if let Some(item) = self
                .results
                .upgrade()
                .and_then(|r| r.selected_item())
                .and_downcast::<TileItem>()
            {
                if let Some(row) = item.parent().upgrade() {
                    row.emit_by_name::<()>("row-should-activate", &[&exit, &""]);
                }
            } else {
                if let Some(current_text) = self.search_bar.upgrade().map(|s| s.text()) {
                    println!("{}", current_text);
                }
            }
        }
    }
    pub fn mark_active(&self) {
        if let Some(results) = self.results.upgrade() {
            results.mark_active();
        }
    }
    pub fn on_prev(&self) {
        if self.context.open.get() {
            self.move_prev_context();
        } else {
            self.move_prev();
        }
    }
    pub fn on_next(&self) {
        if self.context.open.get() {
            self.move_next_context();
        } else {
            self.move_next();
        }
    }
    pub fn open_context(&self) -> Option<()> {
        // Early return if context is already opened
        if self.context.open.get() {
            self.close_context()?;
        }
        let results = self.results.upgrade()?;
        let row = results.selected_item().and_downcast::<TileItem>()?;
        let context = self.context.model.upgrade()?;

        context.remove_all();
        if row.num_actions() > 0 {
            for action in row.actions().iter() {
                context.append(&ContextAction::new(
                    "",
                    action,
                    row.terminal(),
                    row.downgrade(),
                ))
            }
            let context_selection = self.context.view.upgrade()?;
            context_selection.focus_first(None, None, None);
            self.context.open.set(true);
        }
        Some(())
    }
    pub fn close_context(&self) -> Option<()> {
        // Early return if context is closed
        if !self.context.open.get() {
            return None;
        }
        let context = self.context.model.upgrade()?;
        context.remove_all();
        self.context.open.set(false);
        Some(())
    }
    pub fn focus_first(&self, current_mode: Rc<RefCell<String>>) -> Option<()> {
        let results = self.results.upgrade()?;
        results.focus_first(
            Some(&self.context.model),
            Some(current_mode),
            Some(self.custom_handler.clone()),
        );
        Some(())
    }

    // ---- PRIVATES ----
    fn move_prev(&self) -> Option<()> {
        let results = self.results.upgrade()?;
        results.focus_prev(Some(&self.context.model), Some(self.custom_handler.clone()));
        None
    }
    fn move_next(&self) -> Option<()> {
        let results = self.results.upgrade()?;
        results.focus_next(Some(&self.context.model), Some(self.custom_handler.clone()));
        None
    }
    fn move_next_context(&self) -> Option<()> {
        let model = self.context.view.upgrade()?;
        let _ = model.focus_next(None, None);
        None
    }
    fn move_prev_context(&self) -> Option<()> {
        let model = self.context.view.upgrade()?;
        let _ = model.focus_prev(None, None);
        None
    }
}
