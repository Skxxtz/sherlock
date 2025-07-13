mod imp;

use std::{rc::Rc, usize};

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, WeakRef};
use glib::Object;
use gtk4::glib;
use simd_json::prelude::Indexed;

use crate::{g_subclasses::sherlock_row::SherlockRow, launcher::Launcher, loader::util::AppData};

glib::wrapper! {
    pub struct TileItem(ObjectSubclass<imp::TileItem>);
}

impl TileItem {
    pub fn set_index<T: TryInto<u16>>(&self, index: T) {
        self.imp().index.replace(index.try_into().ok());
    }
    pub fn set_launcher(&self, launcher: Rc<Launcher>) {
        self.imp().launcher.replace(launcher);
    }
    pub fn set_parent(&self, parent: &SherlockRow) {
        let weak = parent.downgrade();
        self.imp().parent.replace(weak);
    }

    pub fn get_by_key<F, T>(&self, key: F) -> Option<T>
    where
        F: FnOnce(&AppData) -> T,
    {
        let imp = self.imp();
        let launcher = imp.launcher.borrow();
        let index = imp.index.get()?;
        let inner = launcher.inner()?;
        let data = inner.get(index as usize)?;
        Some(key(&data))
    }

    pub fn get_patch(&self) -> Option<SherlockRow> {
        let imp = self.imp();
        let launcher = imp.launcher.borrow();
        let index = imp.index.get();
        launcher.get_tile(index, launcher.clone())
    }
    pub fn parent(&self) -> WeakRef<SherlockRow> {
        self.imp().parent.borrow().clone()
    }
    pub fn search(&self) -> Option<String> {
        self.get_by_key(|data| data.search_string.clone())
    }
    pub fn priority(&self) -> f32 {
        self.get_by_key(|data| data.priority).unwrap_or(self.imp().launcher.borrow().priority as f32)
    }

    // Constructors
    pub fn from(launcher: Rc<Launcher>) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();

        imp.launcher.replace(launcher);

        obj
    }

    pub fn new() -> Self {
        Object::builder().build()
    }
}
