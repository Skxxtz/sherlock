mod imp;

use std::{
    cell::{Ref, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, GString, SignalHandlerId, WeakRef};
use glib::Object;
use gtk4::{
    gdk::{Key, ModifierType},
    glib,
    prelude::WidgetExt,
};
use serde::{de, Deserialize, Deserializer};

use crate::{
    launcher::{utils::HomeType, Launcher},
    loader::util::{AppData, ApplicationAction},
};

glib::wrapper! {
    pub struct SherlockRow(ObjectSubclass<imp::SherlockRow>)
        @extends gtk4::Box, gtk4::Widget;
}

impl SherlockRow {
    pub fn new() -> Self {
        let myself: Self = Object::builder().build();
        myself.add_css_class("tile");
        myself
    }
    pub fn show(&self) {
        let imp = self.imp();
        let search = imp.search.borrow().to_string();
        let prio = imp.priority.get();
        let home = imp.home.get();
        let spawn = imp.spawn_focus.get();
        let alias = imp.alias.borrow().to_string();

        println!("Search: {:?}", search);
        println!("Prio: {:?}", prio);
        println!("Home: {:?}", home);
        println!("Spawn: {:?}", spawn);
        println!("Alias: {:?}", alias);
    }
    // setters
    pub fn set_spawn_focus(&self, focus: bool) {
        self.imp().spawn_focus.set(focus);
    }
    pub fn set_active(&self, active: bool) {
        self.imp().active.set(active);
        let class_name = GString::from("multi-active");
        let class_exists = self.css_classes().contains(&class_name);
        if class_exists && !active {
            self.remove_css_class("multi-active");
        } else if !class_exists && active {
            self.add_css_class("multi-active");
        }
    }
    pub fn set_search(&self, search: &str) {
        *self.imp().search.borrow_mut() = search.to_lowercase();
    }
    pub fn set_priority(&self, prio: f32) {
        self.imp().priority.set(prio);
    }
    pub fn set_alias(&self, mode: &str) {
        *self.imp().alias.borrow_mut() = mode.to_string();
    }
    pub fn set_home(&self, home: HomeType) {
        self.imp().home.set(home);
    }
    pub fn set_shortcut_holder(&self, holder: Option<WeakRef<gtk4::Box>>) {
        let _ = self.imp().shortcut_holder.set(holder);
    }
    pub fn set_update<F>(&self, state: F)
    where
        F: Fn(&str) -> bool + 'static,
    {
        *self.imp().update.borrow_mut() = Some(Box::new(state));
    }
    pub fn set_async_update<F, Fut>(&self, f: F)
    where
        F: Fn(&str) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let boxed_fn: Box<dyn Fn(&str) -> Pin<Box<dyn Future<Output = ()>>>> =
            Box::new(move |s| Box::pin(f(s)));
        self.imp().async_content_update.replace(Some(boxed_fn));
    }
    pub fn set_signal_id(&self, signal: SignalHandlerId) {
        // Take the previous signal if it exists and disconnect it
        if let Some(old_id) = self.imp().signal_id.borrow_mut().take() {
            self.disconnect(old_id);
            // Store the new signal
        }
        *self.imp().signal_id.borrow_mut() = Some(signal);
    }
    pub fn clear_signal_id(&self) {
        if let Some(old) = self.imp().signal_id.borrow_mut().take() {
            self.disconnect(old);
        }
    }
    pub fn set_keyword_aware(&self, state: bool) {
        self.imp().keyword_aware.set(state);
    }
    pub fn set_actions(&self, actions: Vec<ApplicationAction>) {
        self.imp().num_actions.set(actions.len());
        *self.imp().actions.borrow_mut() = actions;
    }
    pub fn add_actions(&self, actions: &Option<Vec<ApplicationAction>>) {
        if let Some(actions) = actions {
            self.imp().actions.borrow_mut().extend(actions.clone());
        }
        self.imp()
            .num_actions
            .set(self.imp().actions.borrow().len());
    }
    pub fn set_num_actions(&self, num: usize) {
        self.imp().num_actions.set(num);
    }
    pub fn set_terminal(&self, term: bool) {
        self.imp().terminal.set(term);
    }
    pub fn set_binds(&self, binds: Vec<SherlockRowBind>) {
        self.imp().binds.replace(binds);
    }

    // getters
    pub fn shortcut_holder(&self) -> Option<gtk4::Box> {
        self.imp()
            .shortcut_holder
            .get()
            .and_then(|inner| inner.as_ref().and_then(|inner2| inner2.upgrade()))
    }
    pub fn alias(&self) -> String {
        self.imp().alias.borrow().clone()
    }
    pub fn search(&self) -> String {
        self.imp().search.borrow().clone()
    }
    pub fn priority(&self) -> f32 {
        self.imp().priority.get()
    }
    pub fn home(&self) -> HomeType {
        self.imp().home.get()
    }
    pub fn update(&self, keyword: &str) -> bool {
        if let Some(callback) = &*self.imp().update.borrow() {
            callback(keyword)
        } else {
            false
        }
    }
    pub async fn async_update(&self, keyword: &str) {
        if let Some(callback) = &*self.imp().async_content_update.borrow() {
            callback(keyword).await;
        }
    }
    pub fn is_keyword_aware(&self) -> bool {
        self.imp().keyword_aware.get()
    }
    pub fn actions(&self) -> Ref<Vec<ApplicationAction>> {
        self.imp().actions.borrow()
    }
    pub fn active(&self) -> bool {
        self.imp().active.get()
    }
    pub fn num_actions(&self) -> usize {
        self.imp().num_actions.get()
    }
    pub fn terminal(&self) -> bool {
        self.imp().terminal.get()
    }
    pub fn binds(&self) -> Rc<RefCell<Vec<SherlockRowBind>>> {
        self.imp().binds.clone()
    }
    /// Sets shared values from a launcher to the SherlockRow
    /// * only_home
    /// * home
    /// * spawn_focus
    /// * priority
    /// * alias
    pub fn with_launcher(&self, launcher: &Launcher) {
        self.set_home(launcher.home);
        self.set_spawn_focus(launcher.spawn_focus);
        self.set_priority((launcher.priority + 1) as f32);
        if let Some(alias) = &launcher.alias {
            self.set_alias(alias);
        }
        if let Some(binds) = &launcher.binds {
            self.set_binds(binds.clone())
        }
        if let Some(actions) = &launcher.actions {
            self.set_actions(actions.clone());
        }
        if !launcher.exit {
            self.add_css_class("exec-inplace");
        }
    }
    pub fn with_appdata(&self, data: &AppData) {
        self.set_search(&data.search_string);
        self.set_priority(data.priority);
        if !data.actions.is_empty() {
            self.set_actions(data.actions.clone());
        }
        self.set_terminal(data.terminal);
    }
}

impl Default for SherlockRow {
    fn default() -> Self {
        let row = Self::new();
        row.set_spawn_focus(true);
        row.set_css_classes(&["tile"]);
        row
    }
}

#[derive(Debug, Clone)]
pub struct SherlockRowBind {
    pub key: Option<Key>,
    pub modifier: ModifierType,
    pub callback: String,
    pub exit: Option<bool>,
}
impl<'de> Deserialize<'de> for SherlockRowBind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Temp {
            bind: String,
            callback: String,
            exit: Option<bool>,
        }

        let temp = Temp::deserialize(deserializer)?;

        // Parse bind string like "Ctrl+Shift+S"
        let mut key: Option<Key> = None;
        let mut modifier = ModifierType::empty();

        for token in temp.bind.split('+') {
            if let Some(m) = ModifierType::from_name(token) {
                modifier |= m;
            } else if key.is_none() {
                key = Key::from_name(token);
            } else {
                return Err(de::Error::custom(format!("Unknown bind token: {}", token)));
            }
        }

        Ok(SherlockRowBind {
            key,
            modifier,
            callback: temp.callback,
            exit: temp.exit,
        })
    }
}
