use std::u32;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gtk4::gdk::{Key, ModifierType};
use gtk4::{prelude::*, Box as HVBox, Label, SingleSelection};

use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::CONFIG;

pub trait ShortCut {
    fn apply_shortcut(&self, index: i32, mod_str: &str) -> i32;
    fn remove_shortcut(&self) -> i32;
}
impl ShortCut for HVBox {
    fn apply_shortcut(&self, index: i32, mod_str: &str) -> i32 {
        if index < 6 {
            if let Some(child) = self.first_child() {
                if let Some(label) = child.downcast_ref::<Label>() {
                    self.set_visible(true);
                    label.set_text(&format!("{}", mod_str));
                }
            }
            if let Some(child) = self.last_child() {
                if let Some(label) = child.downcast_ref::<Label>() {
                    self.set_visible(true);
                    label.set_text(&format!("{}", index));
                    return 1;
                }
            }
        }
        return 0;
    }
    fn remove_shortcut(&self) -> i32 {
        let r = if self.is_visible() { 1 } else { 0 };
        self.set_visible(false);
        r
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfKeys {
    pub next: Option<Key>,
    pub next_mod: Option<ModifierType>,
    pub prev: Option<Key>,
    pub prev_mod: Option<ModifierType>,
    pub shortcut_modifier: Option<ModifierType>,
    pub shortcut_modifier_str: String,
}
impl ConfKeys {
    pub fn new() -> Self {
        if let Some(c) = CONFIG.get() {
            let (prev_mod, prev) = match &c.binds.prev {
                Some(prev) => ConfKeys::eval_bind_combination(prev),
                _ => (None, None),
            };
            let (next_mod, next) = match &c.binds.next {
                Some(next) => ConfKeys::eval_bind_combination(next),
                _ => (None, None),
            };
            let shortcut_modifier = match &c.binds.modifier {
                Some(shortcut) => ConfKeys::eval_mod(shortcut),
                _ => Some(ModifierType::CONTROL_MASK),
            };
            let shortcut_modifier_str = ConfKeys::get_mod_str(&shortcut_modifier);
            return ConfKeys {
                next,
                next_mod,
                prev,
                prev_mod,
                shortcut_modifier,
                shortcut_modifier_str,
            };
        }
        ConfKeys::empty()
    }
    pub fn empty() -> Self {
        ConfKeys {
            next: None,
            next_mod: None,
            prev: None,
            prev_mod: None,
            shortcut_modifier: None,
            shortcut_modifier_str: String::new(),
        }
    }
    fn eval_bind_combination<T: AsRef<str>>(key: T) -> (Option<ModifierType>, Option<Key>) {
        let key_str = key.as_ref();
        match key_str.split("-").collect::<Vec<&str>>().as_slice() {
            [modifier, key, ..] => (ConfKeys::eval_mod(modifier), ConfKeys::eval_key(key)),
            [key, ..] => (None, ConfKeys::eval_key(key)),
            _ => (None, None),
        }
    }
    fn eval_key<T: AsRef<str>>(key: T) -> Option<Key> {
        match key.as_ref().to_ascii_lowercase().as_ref() {
            "tab" => Some(Key::Tab),
            "up" => Some(Key::Up),
            "down" => Some(Key::Down),
            "left" => Some(Key::Left),
            "right" => Some(Key::Right),
            "pgup" => Some(Key::Page_Up),
            "pgdown" => Some(Key::Page_Down),
            "end" => Some(Key::End),
            "home" => Some(Key::Home),
            // Alphabet
            k if k.len() == 1 && k.chars().all(|c| c.is_ascii_alphabetic()) => Key::from_name(k),
            _ => None,
        }
    }
    fn eval_mod(key: &str) -> Option<ModifierType> {
        match key {
            "shift" => Some(ModifierType::SHIFT_MASK),
            "control" => Some(ModifierType::CONTROL_MASK),
            "alt" => Some(ModifierType::ALT_MASK),
            "super" => Some(ModifierType::SUPER_MASK),
            "lock" => Some(ModifierType::LOCK_MASK),
            "hypr" => Some(ModifierType::HYPER_MASK),
            "meta" => Some(ModifierType::META_MASK),
            _ => None,
        }
    }
    fn get_mod_str(mod_key: &Option<ModifierType>) -> String {
        let k = match mod_key {
            Some(ModifierType::SHIFT_MASK) | Some(ModifierType::LOCK_MASK) => "⇧",
            Some(ModifierType::CONTROL_MASK) | Some(ModifierType::META_MASK) => "⌘",
            Some(ModifierType::ALT_MASK) => "⎇",
            Some(ModifierType::SUPER_MASK) | Some(ModifierType::HYPER_MASK) => "✦",
            _ => "⌘",
        };
        k.to_string()
    }
}

pub trait SherlockNav {
    fn focus_next(&self) -> (u32, u32);
    fn focus_prev(&self) -> (u32, u32);
    fn focus_first(&self) -> (u32, u32);
    fn execute_by_index(&self, index: u32);
}
impl SherlockNav for SingleSelection {
    fn focus_next(&self) -> (u32, u32) {
        let index = self.selected();
        if index == u32::MAX {
            return (index, 0);
        }
        let new_index = index + 1;
        let n_items = self.n_items();
        if new_index < n_items {
            self.set_selected(new_index);
            return (new_index, n_items);
        }
        (index, n_items)
    }
    fn focus_prev(&self) -> (u32, u32) {
        let index = self.selected();
        let n_items = self.n_items();
        if index > 0 {
            self.set_selected(index - 1);
            return (index - 1, n_items);
        }
        (index, n_items)
    }
    fn focus_first(&self) -> (u32, u32) {
        let mut i = 0;
        let current_index = self.selected();
        let n_items = self.n_items();
        while i < n_items {
            if let Some(item) = self.item(i).and_downcast::<SherlockRow>() {
                if item.imp().spawn_focus.get() {
                    self.set_selected(i);
                    return (i, n_items);
                } else {
                    i += 1;
                }
            }
        }
        self.set_selected(current_index);
        (current_index, n_items)
    }
    fn execute_by_index(&self, index: u32) {
        for item in index..self.n_items() {
            if let Some(row) = self.item(item).and_downcast::<SherlockRow>() {
                if row.imp().shortcut.get() {
                    row.emit_by_name::<()>("row-should-activate", &[]);
                    break;
                }
            }
        }
    }
}

pub fn truncate_if_needed(s: String, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len]) // IF TOO LONG
    } else {
        s.clone()
    }
}
