mod imp;

use std::{future::Future, pin::Pin};

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, SignalHandlerId, WeakRef};
use glib::Object;
use gtk4::{glib, prelude::WidgetExt};

use crate::launcher::Launcher;

glib::wrapper! {
    pub struct SherlockRow(ObjectSubclass<imp::SherlockRow>)
        @extends gtk4::Box, gtk4::Widget;
}

impl SherlockRow {
    pub fn new() -> Self {
        Object::builder().build()
    }
    // setters
    pub fn set_spawn_focus(&self, focus: bool) {
        self.imp().spawn_focus.set(focus);
    }
    pub fn set_shortcut(&self, shortcut: bool) {
        self.imp().shortcut.set(shortcut);
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
    pub fn set_home(&self, home: bool) {
        self.imp().home.set(home);
    }
    pub fn set_only_home(&self, home: bool) {
        self.imp().only_home.set(home);
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
    pub fn set_keyword_aware(&self, state: bool) {
        self.imp().keyword_aware.set(state);
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
    pub fn home(&self) -> (bool, bool) {
        let only_home = self.imp().only_home.get();
        let home = self.imp().home.get();
        (home, only_home)
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
    /// Sets shared values from a launcher to the SherlockRow
    /// * only_home
    /// * home
    /// * spawn_focus
    /// * priority
    /// * alias
    pub fn with_launcher(&self, launcher: &Launcher) {
        self.set_only_home(launcher.only_home);
        self.set_home(launcher.home);
        self.set_shortcut(launcher.shortcut);
        self.set_spawn_focus(launcher.spawn_focus);
        self.set_priority(launcher.priority as f32);
        if let Some(alias) = &launcher.alias {
            self.set_alias(alias);
        }
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
