use std::cell::RefCell;
use std::rc::Rc;

use gtk4::subclass::prelude::*;
use gtk4::glib;

use crate::launcher::Launcher;

/// ## Fields:
#[derive(Default, Debug)]
pub struct TileItem {
    launcher: Rc<RefCell<Launcher>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TileItem {
    const NAME: &'static str = "TileObject";
    type Type = super::TileItem;
    type ParentType = glib::Object;
}

// Trait shared by all GObjects
impl ObjectImpl for TileItem {
    fn constructed(&self) {
        self.parent_constructed();
    }
}
