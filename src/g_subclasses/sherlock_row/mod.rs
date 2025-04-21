mod imp;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk4::glib;

use crate::ui::tiles::util::TileWidgets;

glib::wrapper! {
    pub struct SherlockRow(ObjectSubclass<imp::SherlockRow>)
        @extends gtk4::ListBoxRow, gtk4::Widget;
}

impl SherlockRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn widgets(&self) -> &TileWidgets {
        self.imp().widgets.get().unwrap()
    }

    pub fn set_widgets(&self, widgets: TileWidgets) {
        let _ = self.imp().widgets.set(widgets);
    }

    pub fn set_spawn_focus(&self, focus: bool) {
        self.imp().spawn_focus.set(focus);
    }
    pub fn set_shortcut(&self, shortcut: bool) {
        self.imp().shortcut.set(shortcut);
    }

    pub fn set_priority(&self, priority: f32) {
        self.imp().priority.set(priority);
    }
}

impl Default for SherlockRow {
    fn default() -> Self {
        Self::new()
    }
}
