use futures::future::join_all;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::u32;

use gio::glib::{self, WeakRef};
use gio::ListStore;
use gtk4::gdk::{Key, ModifierType};
use gtk4::{
    prelude::*, Box as GtkBox, CustomFilter, CustomSorter, Entry, Justification, Label, ListView,
    ScrolledWindow, Spinner,
};
use serde::Deserialize;

use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::loader::Loader;
use crate::sherlock_error;
use crate::utils::config::{default_modkey_ascii, ConfigGuard};
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::utils::paths;
use crate::{sherlock_error, CONFIG};


use super::tiles::util::TextViewTileBuilder;

#[derive(Debug, Clone, PartialEq)]
pub struct ConfKeys {
    // Next
    pub next: Option<Key>,
    pub next_mod: Option<ModifierType>,
    // Previous
    pub prev: Option<Key>,
    pub prev_mod: Option<ModifierType>,
    // Inplace execution
    pub exec_inplace: Option<Key>,
    pub exec_inplace_mod: Option<ModifierType>,
    // ContextMenu
    pub context: Option<Key>,
    pub context_mod: Option<ModifierType>,
    pub context_str: Option<String>,
    pub context_mod_str: String,
    // Shortcuts
    pub shortcut_modifier: Option<ModifierType>,
    pub shortcut_modifier_str: String,
}
impl ConfKeys {
    pub fn new() -> Self {
        if let Ok(c) = ConfigGuard::read() {
            let (prev_mod, prev) = match &c.binds.prev {
                Some(prev) => ConfKeys::eval_bind_combination(prev),
                _ => (None, (None, None)),
            };
            let (next_mod, next) = match &c.binds.next {
                Some(next) => ConfKeys::eval_bind_combination(next),
                _ => (None, (None, None)),
            };
            let (exec_inplace_mod, inplace) = match &c.binds.exec_inplace {
                Some(inplace) => ConfKeys::eval_bind_combination(inplace),
                _ => (None, (None, None)),
            };
            let (context_mod, context) = match &c.binds.context {
                Some(context) => ConfKeys::eval_bind_combination(context),
                _ => (None, (None, None)),
            };
            let shortcut_modifier = match &c.binds.modifier {
                Some(shortcut) => ConfKeys::eval_mod(shortcut),
                _ => Some(ModifierType::CONTROL_MASK),
            };
            let shortcut_modifier_str = ConfKeys::get_mod_str(&shortcut_modifier);
            let context_mod_str = ConfKeys::get_mod_str(&context_mod);
            return ConfKeys {
                next: next.0,
                next_mod,
                prev: prev.0,
                prev_mod,
                exec_inplace: inplace.0,
                exec_inplace_mod,
                context: context.0,
                context_mod,
                context_str: context.1,
                context_mod_str,
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
            exec_inplace: None,
            exec_inplace_mod: None,
            context: None,
            context_mod: None,
            context_mod_str: String::new(),
            context_str: None,
            shortcut_modifier: None,
            shortcut_modifier_str: String::new(),
        }
    }
    fn eval_bind_combination(key: &str) -> (Option<ModifierType>, (Option<Key>, Option<String>)) {
        match key.split("-").collect::<Vec<&str>>().as_slice() {
            [modifier, key, ..] => (ConfKeys::eval_mod(modifier), ConfKeys::eval_key(key)),
            [key, ..] => (None, ConfKeys::eval_key(key)),
            _ => (None, (None, None)),
        }
    }
    fn eval_key<T: AsRef<str>>(key: T) -> (Option<Key>, Option<String>) {
        match key.as_ref().to_ascii_lowercase().as_ref() {
            "tab" => (Some(Key::Tab), Some(String::from("⇥"))),
            "up" => (Some(Key::Up), Some(String::from("↑"))),
            "down" => (Some(Key::Down), Some(String::from("↓"))),
            "left" => (Some(Key::Left), Some(String::from("←"))),
            "right" => (Some(Key::Right), Some(String::from("→"))),
            "pgup" => (Some(Key::Page_Up), Some(String::from("⇞"))),
            "pgdown" => (Some(Key::Page_Down), Some(String::from("⇟"))),
            "end" => (Some(Key::End), Some(String::from("End"))),
            "home" => (Some(Key::Home), Some(String::from("Home"))),
            "return" => (Some(Key::Return), Some(String::from("↩"))),
            // Alphabet
            k if k.len() == 1 && k.chars().all(|c| c.is_ascii_alphabetic()) => {
                (Key::from_name(k), Some(k.to_uppercase()))
            }
            _ => (None, None),
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
        let strings = ConfigGuard::read()
            .ok()
            .and_then(|c| {
                let s = &c.appearance.mod_key_ascii;
                if s.len() == 8 {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(default_modkey_ascii);

        let k = match mod_key {
            Some(ModifierType::SHIFT_MASK) => 0,
            Some(ModifierType::LOCK_MASK) => 1,
            Some(ModifierType::CONTROL_MASK) => 2,
            Some(ModifierType::META_MASK) => 3,
            Some(ModifierType::ALT_MASK) => 4,
            Some(ModifierType::SUPER_MASK) => 5,
            Some(ModifierType::HYPER_MASK) => 6,
            _ => 7,
        };
        strings.get(k).cloned().unwrap_or(String::from("modkey"))
    }
}

#[derive(Debug, Deserialize)]
pub struct SherlockAction {
    pub on: u32,
    pub action: String,
    pub exec: Option<String>,
}
impl Display for SherlockAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{{"on": {}, "action": "{}", "exec": {} }}"#,
            self.on,
            self.action,
            match &self.exec {
                Some(s) => format!(r#""{}""#, s),
                None => "null".to_string(),
            }
        )
    }
}

pub struct SherlockCounter {
    path: PathBuf,
}
impl SherlockCounter {
    pub fn new() -> Result<Self, SherlockError> {
        let cache_dir = paths::get_cache_dir()?;
        let path = cache_dir.join("sherlock_count");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::DirCreateError(parent.to_string_lossy().to_string()),
                    e.to_string()
                )
            })?;
        }
        Ok(Self { path })
    }
    pub fn increment(&self) -> Result<u32, SherlockError> {
        let content = self.read()?.saturating_add(1);
        self.write(content)?;
        Ok(content)
    }
    pub fn read(&self) -> Result<u32, SherlockError> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(0);
            }
            Err(e) => {
                return Err(sherlock_error!(
                    SherlockErrorType::FileReadError(self.path.clone()),
                    e.to_string()
                ));
            }
        };
        let mut buf = [0u8; 4];

        file.read_exact(&mut buf).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileReadError(self.path.clone()),
                e.to_string()
            )
        })?;
        Ok(u32::from_le_bytes(buf))
    }
    pub fn write(&self, count: u32) -> Result<(), SherlockError> {
        let file = File::create(self.path.clone()).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileWriteError(self.path.clone()),
                e.to_string()
            )
        })?;

        let mut writer = BufWriter::new(file);
        writer.write_all(&count.to_le_bytes()).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileWriteError(self.path.clone()),
                e.to_string()
            )
        })?;

        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct SearchHandler {
    pub model: Option<WeakRef<ListStore>>,
    pub mode: Rc<RefCell<String>>,
    pub modes: Rc<RefCell<HashMap<String, Option<String>>>>,
    pub task: Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    pub error_model: WeakRef<ListStore>,
    pub filter: WeakRef<CustomFilter>,
    pub sorter: WeakRef<CustomSorter>,
    pub binds: ConfKeys,
    pub first_iter: Cell<bool>,
}
impl SearchHandler {
    pub fn new(
        model: WeakRef<ListStore>,
        mode: Rc<RefCell<String>>,
        error_model: WeakRef<ListStore>,
        filter: WeakRef<CustomFilter>,
        sorter: WeakRef<CustomSorter>,
        binds: ConfKeys,
        first_iter: Cell<bool>,
    ) -> Self {
        Self {
            model: Some(model),
            mode,
            modes: Rc::new(RefCell::new(HashMap::new())),
            task: Rc::new(RefCell::new(None)),
            error_model,
            filter,
            sorter,
            binds,
            first_iter,
        }
    }
    pub fn clear(&self) {
        if let Some(model) = self.model.as_ref().and_then(|m| m.upgrade()) {
            model.remove_all();
        }
    }

    pub fn populate(&self) {
        // clear potentially stuck rows
        self.clear();
        self.first_iter.set(true);

        // load launchers
        let (launchers, n) = match Loader::load_launchers().map_err(|e| e.tile("ERROR")) {
            Ok(r) => r,
            Err(e) => {
                if let Some(model) = self.error_model.upgrade() {
                    model.append(&e);
                }
                return;
            }
        };
        if let Some(model) = self.error_model.upgrade() {
            n.into_iter()
                .map(|n| n.tile("WARNING"))
                .for_each(|row| model.append(&row));
        }

        if let Some(model) = self.model.as_ref().and_then(|m| m.upgrade()) {
            let mut holder: HashMap<String, Option<String>> = HashMap::new();
            let rows: Vec<SherlockRow> = launchers
                .into_iter()
                .map(|mut launcher| {
                    let patch = launcher.get_patch();
                    if let Some(alias) = &launcher.alias {
                        holder.insert(format!("{} ", alias), launcher.name);
                    }
                    patch
                })
                .flatten()
                .collect();
            model.splice(0, model.n_items(), &rows);
            let weaks: Vec<WeakRef<SherlockRow>> =
                rows.into_iter().map(|row| row.downgrade()).collect();
            update_async(weaks, &self.task, String::new());
            *self.modes.borrow_mut() = holder;
        }
    }
}

#[derive(Clone)]
pub struct ContextUI {
    pub model: WeakRef<ListStore>,
    pub view: WeakRef<ListView>,
    pub open: Rc<Cell<bool>>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct SearchUI {
    pub all: WeakRef<GtkBox>,
    pub result_viewport: WeakRef<ScrolledWindow>,
    pub results: WeakRef<ListView>,
    // will be later used for split view to display information about apps/commands
    pub preview_box: WeakRef<GtkBox>,
    pub status_bar: WeakRef<GtkBox>,
    pub search_bar: WeakRef<Entry>,
    pub search_icon_holder: WeakRef<GtkBox>,
    pub mode_title_holder: WeakRef<GtkBox>,
    pub mode_title: WeakRef<Label>,
    pub spinner: WeakRef<Spinner>,
    pub filter: WeakRef<CustomFilter>,
    pub sorter: WeakRef<CustomSorter>,
    pub binds: ConfKeys,
    pub context_menu_desc: WeakRef<Label>,
    pub context_menu_first: WeakRef<Label>,
    pub context_menu_second: WeakRef<Label>,
}
pub fn update_async(
    update_tiles: Vec<WeakRef<SherlockRow>>,
    current_task: &Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    keyword: String,
) {
    let current_task_clone = Rc::clone(current_task);
    if let Some(t) = current_task.borrow_mut().take() {
        t.abort();
    };
    let task = glib::MainContext::default().spawn_local({
        async move {
            // Set spinner active
            let spinner_row = update_tiles.get(0).cloned();
            if let Some(row) = spinner_row.as_ref().and_then(|row| row.upgrade()) {
                let _ = row.activate_action("win.spinner-mode", Some(&true.to_variant()));
            }
            // Make async tiles update concurrently
            let futures: Vec<_> = update_tiles
                .into_iter()
                .map(|row| {
                    let current_text = keyword.clone();
                    async move {
                        // Process text tile
                        if let Some(row) = row.upgrade() {
                            row.async_update(&current_text).await
                        }
                    }
                })
                .collect();

            let _ = join_all(futures).await;
            // Set spinner inactive
            if let Some(row) = spinner_row.as_ref().and_then(|row| row.upgrade()) {
                let _ = row.activate_action("win.spinner-mode", Some(&false.to_variant()));
            }
            *current_task_clone.borrow_mut() = None;
        }
    });
    *current_task.borrow_mut() = Some(task);
}

pub fn display_raw<T: AsRef<str>>(content: T, center: bool) -> GtkBox {
    let builder = TextViewTileBuilder::new("/dev/skxxtz/sherlock/ui/text_view_tile.ui");
    builder
        .content
        .as_ref()
        .and_then(|tmp| tmp.upgrade())
        .map(|ctx| {
            let buffer = ctx.buffer();
            ctx.add_css_class("raw_text");
            ctx.set_monospace(true);
            let sanitized: String = content.as_ref().chars().filter(|&c| c != '\0').collect();
            buffer.set_text(&sanitized);
            if center {
                ctx.set_justification(Justification::Center);
            }
        });
    let row = builder.object.unwrap_or_default();
    row
}
