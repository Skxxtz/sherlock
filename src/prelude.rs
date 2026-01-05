use std::{borrow::Cow, cell::RefCell, collections::HashSet, fmt::Debug, rc::Rc, time::SystemTime};

use gio::{
    ListStore,
    glib::{
        self, Object, WeakRef,
        object::{Cast, CastNone, IsA, ObjectExt},
        variant::ToVariant,
    },
    prelude::ListModelExt,
};
use gtk4::{
    Box as GtkBox, GridView, Image, Label, ListScrollFlags, ListView, SingleSelection, Stack,
    StackPage, Widget, prelude::WidgetExt,
};

use crate::{
    g_subclasses::{emoji_item::EmojiObject, tile_item::TileItem},
    loader::{icon_loader::IconThemeGuard, pipe_loader::PipedElements},
    ui::search::UserBindHandler,
};

/// Custom string matching
pub trait SherlockSearch {
    fn fuzzy_match<'a, T: Into<Cow<'a, str>> + Debug>(&self, substring: T) -> bool;
}

impl SherlockSearch for String {
    fn fuzzy_match<'a, T>(&self, substring: T) -> bool
    where
        T: Into<Cow<'a, str>> + Debug,
    {
        let lowercase = substring.into().to_lowercase();
        let char_pattern: HashSet<char> = lowercase.chars().collect();
        let concat_str: String = self
            .to_lowercase()
            .chars()
            .filter(|s| char_pattern.contains(s) || *s == ';')
            .collect();
        concat_str.contains(&lowercase)
    }
}
impl SherlockSearch for PipedElements {
    fn fuzzy_match<'a, T>(&self, substring: T) -> bool
    where
        T: Into<Cow<'a, str>> + Debug,
    {
        // check which value to use
        let search_in = match self.title {
            Some(_) => &self.title,
            None => &self.description,
        };
        if let Some(search_in) = search_in {
            let lowercase = substring.into().to_lowercase();
            let char_pattern: HashSet<char> = lowercase.chars().collect();
            let concat_str: String = search_in
                .to_lowercase()
                .chars()
                .filter(|s| char_pattern.contains(s) || *s == ';')
                .collect();
            return concat_str.contains(&lowercase);
        }
        false
    }
}
/// Apply icon by name or by path if applicable
pub trait IconComp {
    fn set_icon(&self, icon_name: Option<&str>, icon_class: Option<&str>, fallback: Option<&str>);
}
impl IconComp for Image {
    fn set_icon(&self, icon_name: Option<&str>, icon_class: Option<&str>, fallback: Option<&str>) {
        if let Some(icon_name) = icon_name.or(fallback) {
            if let Ok(Some(icon)) = IconThemeGuard::lookup_icon(icon_name) {
                self.set_from_file(Some(icon));
                return;
            }

            if icon_name.starts_with("/") {
                self.set_from_file(Some(icon_name));
            } else {
                self.set_icon_name(Some(icon_name));
            }
        } else {
            self.set_visible(false);
        }
        if let Some(class) = icon_class {
            self.add_css_class(class);
        }
    }
}
pub trait ShortCut {
    fn apply_shortcut(&self, index: i32, mod_str: &str) -> i32;
    fn remove_shortcut(&self) -> i32;
}
impl ShortCut for GtkBox {
    fn apply_shortcut(&self, index: i32, mod_str: &str) -> i32 {
        let mut internal_index = index;
        if index == 10 {
            internal_index = 0;
        }
        if let Some(child) = self.first_child()
            && let Some(label) = child.downcast_ref::<Label>()
        {
            self.set_visible(true);
            label.set_text(mod_str);
        }
        if let Some(child) = self.last_child()
            && let Some(label) = child.downcast_ref::<Label>()
        {
            self.set_visible(true);
            label.set_text(&format!("{}", internal_index));
            return 1;
        }
        0
    }
    fn remove_shortcut(&self) -> i32 {
        let r = if self.is_visible() { 1 } else { 0 };
        self.set_visible(false);
        r
    }
}

/// Navigation for elements within a ListView
pub trait SherlockNav {
    // fn assign_binds(&self, context_model: Option<&WeakRef<ListStore>>, binds: &SherlockRowBinds, listener: Rc<EventControllerKey> ) -> Option<()>;
    fn context_action(&self, context_model: Option<&WeakRef<ListStore>>) -> Option<()>;
    fn focus_next(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()>;
    fn focus_prev(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()>;
    fn focus_first(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        current_mode: Option<Rc<RefCell<String>>>,
        custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()>;
    fn focus_offset(&self, context_model: Option<&WeakRef<ListStore>>, offset: i32) -> Option<()>;
    fn execute_by_index(&self, index: u32);
    fn selected_item(&self) -> Option<glib::Object>;
    fn get_weaks(&self) -> Option<Vec<WeakRef<TileItem>>>;
    fn mark_active(&self) -> Option<()>;
    fn get_actives<T: IsA<Object>>(&self) -> Option<Vec<T>>;
}
impl SherlockNav for ListView {
    fn context_action(&self, context_model: Option<&WeakRef<ListStore>>) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let selected = selection.selected_item().and_downcast::<TileItem>()?;
        if selected.num_actions() > 0 {
            let _ = self.activate_action(
                "win.context-mode",
                Some(&"Additional Actions".to_string().to_variant()),
            );
        } else {
            let _ = self.activate_action("win.context-mode", Some(&"".to_string().to_variant()));
        }
        if let Some(ctx) = context_model.and_then(|tmp| tmp.upgrade()) {
            ctx.remove_all()
        }
        Some(())
    }
    fn focus_offset(
        &self,
        _context_model: Option<&WeakRef<ListStore>>,
        _offset: i32,
    ) -> Option<()> {
        None
    }
    fn focus_next(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let index = selection.selected();
        if index == u32::MAX {
            return None;
        }
        let n_items = selection.n_items();
        let new_index = index + 1;
        if new_index < n_items {
            selection.set_selected(new_index);
            self.scroll_to(new_index, ListScrollFlags::NONE, None);
            let selected = selection.selected_item().and_downcast::<TileItem>()?;

            // Logic to handle custom user binds
            if let Some(handler) = custom_handler {
                let mut handler = handler.borrow_mut();
                let custom_binds = selected.binds();
                if let Some(id) = handler.set_binds(custom_binds, selected.downgrade()) {
                    handler.set_handler(id);
                }
            }
            self.context_action(context_model);
        }
        None
    }
    fn focus_prev(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let index = selection.selected();
        let n_items = selection.n_items();
        let new_index = if index > 0 {
            selection.set_selected(index - 1);
            index - 1
        } else {
            index
        };
        if new_index != index && new_index < n_items {
            self.scroll_to(new_index, ListScrollFlags::NONE, None);
            let selected = selection.selected_item().and_downcast::<TileItem>()?;

            // Logic to handle custom user binds
            if let Some(handler) = custom_handler {
                let mut handler = handler.borrow_mut();
                let custom_binds = selected.binds();
                if let Some(id) = handler.set_binds(custom_binds, selected.downgrade()) {
                    handler.set_handler(id);
                }
            }
            self.context_action(context_model);
        }
        None
    }
    fn focus_first(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        current_mode: Option<Rc<RefCell<String>>>,
        custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let current_mode = current_mode.unwrap_or(Rc::new(RefCell::new(String::from("all"))));
        let mut new_index = 0;
        let current_index = selection.selected();
        let n_items = selection.n_items();
        if n_items == 0 {
            return None;
        }
        while new_index < n_items {
            if let Some(item) = selection.item(new_index).and_downcast::<TileItem>() {
                if item.spawn_focus() {
                    break;
                } else {
                    new_index += 1;
                }
            } else {
                break;
            }
        }
        let changed = new_index != current_index;
        if changed {
            selection.set_selected(new_index);
        }
        if new_index < n_items {
            self.scroll_to(0, ListScrollFlags::NONE, None);
        }
        // Update context mode shortcuts
        let selected = selection.selected_item().and_downcast::<TileItem>()?;

        // Logic to handle custom user binds
        if let Some(handler) = custom_handler {
            let mut handler = handler.borrow_mut();
            let custom_binds = selected.binds();
            if let Some(id) = handler.set_binds(custom_binds, selected.downgrade()) {
                handler.set_handler(id);
            }
        }
        self.context_action(context_model);

        (changed || selected.alias() == *current_mode.borrow().trim()).then_some(())
    }
    fn execute_by_index(&self, index: u32) {
        if let Some(selection) = self.model().and_downcast::<SingleSelection>()
            && let Some(item_at_index) = (0..selection.n_items())
                .filter_map(|i| selection.item(i).and_downcast::<TileItem>())
                .filter(|item| item.shortcut().is_some())
                .nth(index as usize)
        {
            let exit: u8 = 0;
            if let Some(row) = item_at_index.parent().upgrade() {
                row.emit_by_name::<()>("row-should-activate", &[&exit, &""]);
            }
        }
    }
    fn selected_item(&self) -> Option<glib::Object> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        if selection.n_items() == 0 {
            return None;
        }
        selection.selected_item()
    }
    fn get_weaks(&self) -> Option<Vec<WeakRef<TileItem>>> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let n_items = selection.n_items();
        let weaks = (0..n_items)
            .filter_map(|i| selection.item(i).and_downcast::<TileItem>())
            .filter(|r| r.is_async() && r.parent().upgrade().is_some())
            .map(|row| row.downgrade())
            .collect();
        Some(weaks)
    }
    fn mark_active(&self) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let current = selection.selected_item().and_downcast::<TileItem>()?;
        current.toggle_active();
        Some(())
    }
    fn get_actives<T: IsA<Object>>(&self) -> Option<Vec<T>> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let actives: Vec<T> = (0..selection.n_items())
            .filter_map(|i| selection.item(i).and_downcast::<TileItem>())
            .filter(|r| r.active())
            .filter_map(|tile| tile.parent().upgrade())
            .map(|r| r.upcast::<Object>())
            .filter_map(|r| r.downcast::<T>().ok())
            .collect();
        Some(actives)
    }
}
impl SherlockNav for GridView {
    fn context_action(&self, context_model: Option<&WeakRef<ListStore>>) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let selected = selection.selected_item().and_downcast::<EmojiObject>()?;
        if selected.num_actions() > 0 {
            let _ = self.activate_action(
                "win.context-mode",
                Some(&"Additional Skin Tones".to_string().to_variant()),
            );
        } else {
            let _ = self.activate_action("win.context-mode", Some(&"".to_string().to_variant()));
        }
        if let Some(ctx) = context_model.and_then(|tmp| tmp.upgrade()) {
            ctx.remove_all()
        }
        Some(())
    }
    fn focus_next(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        _custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let index = selection.selected();
        if index == u32::MAX {
            return None;
        }
        let n_items = selection.n_items();
        let new_index = index + 1;

        if new_index != index && new_index < n_items {
            selection.set_selected(new_index);
            self.scroll_to(new_index, ListScrollFlags::NONE, None);
        }
        self.context_action(context_model);

        None
    }
    fn focus_prev(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        _custom_handler: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let index = selection.selected();
        let n_items = selection.n_items();
        let new_index = if index > 0 {
            selection.set_selected(index - 1);
            index - 1
        } else {
            index
        };
        if new_index != index && new_index < n_items {
            self.scroll_to(new_index, ListScrollFlags::NONE, None);
            self.context_action(context_model);
        }
        None
    }
    fn focus_first(
        &self,
        context_model: Option<&WeakRef<ListStore>>,
        _current_mode: Option<Rc<RefCell<String>>>,
        _: Option<Rc<RefCell<UserBindHandler>>>,
    ) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let current_index = selection.selected();
        let n_items = selection.n_items();
        if n_items == 0 || current_index == 0 {
            self.context_action(context_model);
            return None;
        }
        selection.set_selected(0);
        self.scroll_to(0, ListScrollFlags::NONE, None);
        self.context_action(context_model);
        Some(())
    }
    fn focus_offset(&self, context_model: Option<&WeakRef<ListStore>>, offset: i32) -> Option<()> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        let current_index = selection.selected() as i32;
        let n_items = selection.n_items() as i32;
        let new_index = offset.checked_add(current_index)?.clamp(0, n_items - 1);
        selection.set_selected(new_index as u32);
        self.scroll_to(new_index as u32, ListScrollFlags::NONE, None);
        self.context_action(context_model);
        Some(())
    }
    fn execute_by_index(&self, _index: u32) {}
    fn selected_item(&self) -> Option<glib::Object> {
        let selection = self.model().and_downcast::<SingleSelection>()?;
        selection.selected_item()
    }
    fn get_weaks(&self) -> Option<Vec<WeakRef<TileItem>>> {
        None
    }
    fn mark_active(&self) -> Option<()> {
        None
    }
    fn get_actives<T: IsA<Object>>(&self) -> Option<Vec<T>> {
        None
    }
}

pub trait PathHelpers {
    fn modtime(&self) -> Option<SystemTime>;
}

pub trait StackHelpers {
    fn get_page_names(&self) -> Vec<String>;
}
impl StackHelpers for Stack {
    fn get_page_names(&self) -> Vec<String> {
        let selection = self.pages();
        let pages: Vec<String> = (0..selection.n_items())
            .filter_map(|i| selection.item(i).and_downcast::<StackPage>())
            .filter_map(|item| item.name())
            .map(|name| name.to_string())
            .collect();
        pages
    }
}

pub trait TileHandler {
    fn replace_tile(&mut self, tile: &Widget);
}
