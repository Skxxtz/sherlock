use futures::future::join_all;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::u32;

use gio::glib::{self, WeakRef};
use gio::ListStore;
use gtk4::gdk::{Key, ModifierType};
use gtk4::{
    prelude::*, Box as GtkBox, CustomFilter, CustomSorter, Entry, Label, ListView, ScrolledWindow,
    Spinner,
};
use serde::Deserialize;

use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::loader::Loader;
use crate::utils::config::default_modkey_ascii;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::{sherlock_error, CONFIG};

#[derive(Debug, Clone, PartialEq)]
pub struct ConfKeys {
    // Next
    pub next: Option<Key>,
    pub next_mod: Option<ModifierType>,
    // Previous
    pub prev: Option<Key>,
    pub prev_mod: Option<ModifierType>,
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
        if let Some(c) = CONFIG.get() {
            let (prev_mod, prev) = match &c.binds.prev {
                Some(prev) => ConfKeys::eval_bind_combination(prev),
                _ => (None, (None, None)),
            };
            let (next_mod, next) = match &c.binds.next {
                Some(next) => ConfKeys::eval_bind_combination(next),
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
            context: None,
            context_mod: None,
            context_mod_str: String::new(),
            context_str: None,
            shortcut_modifier: None,
            shortcut_modifier_str: String::new(),
        }
    }
    fn eval_bind_combination<T: AsRef<str>>(
        key: T,
    ) -> (Option<ModifierType>, (Option<Key>, Option<String>)) {
        let key_str = key.as_ref();
        match key_str.split("-").collect::<Vec<&str>>().as_slice() {
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
        let strings = CONFIG
            .get()
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
}
pub struct SherlockCounter {
    path: PathBuf,
}
impl SherlockCounter {
    pub fn new() -> Result<Self, SherlockError> {
        let cache_home = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                // Fallback to ~/.cache if XDG_CACHE_HOME is not set
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"))
            })
            .ok_or_else(|| {
                sherlock_error!(
                    SherlockErrorType::EnvVarNotFoundError("HOME or XDG_CACHE_HOME".to_string()),
                    "Neither HOME nor XDG_CACHE_HOME environment variable found".to_string()
                )
            })?;

        let path = cache_home.join("sherlock/sherlock_count");

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::DirCreateError(parent.display().to_string()),
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
    pub modes: Rc<RefCell<HashMap<String, Option<String>>>>,
    pub task: Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    pub error_model: WeakRef<ListStore>,
}
impl SearchHandler {
    pub fn new(model: WeakRef<ListStore>, error_model: WeakRef<ListStore>) -> Self {
        Self {
            model: Some(model),
            modes: Rc::new(RefCell::new(HashMap::new())),
            task: Rc::new(RefCell::new(None)),
            error_model,
        }
    }
    pub fn empty(error_model: WeakRef<ListStore>) -> Self {
        Self {
            model: None,
            modes: Rc::new(RefCell::new(HashMap::new())),
            task: Rc::new(RefCell::new(None)),
            error_model,
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
            let rows: Vec<WeakRef<SherlockRow>> = launchers
                .into_iter()
                .map(|launcher| {
                    let patch = launcher.get_patch();
                    if let Some(alias) = &launcher.alias {
                        holder.insert(format!("{} ", alias), launcher.name);
                    }
                    patch
                })
                .flatten()
                .map(|row| {
                    model.append(&row);
                    row.downgrade()
                })
                .collect();
            update_async(rows, &self.task, String::new());
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
