use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gio::glib::WeakRef;
use gtk4::glib;
use gtk4::subclass::prelude::*;

use crate::g_subclasses::sherlock_row::{SherlockRow, SherlockRowBind};
use crate::g_subclasses::tile_item::UpdateHandler;
use crate::launcher::Launcher;
use crate::loader::util::ApplicationAction;

/// ## Fields:
#[derive(Default)]
pub struct TileItem {
    pub launcher: RefCell<Rc<Launcher>>,

    pub index: Cell<Option<u16>>,
    pub parent: RefCell<WeakRef<SherlockRow>>,

    pub update_handler: Rc<RefCell<UpdateHandler>>,

    // Customs
    pub actions: RefCell<Vec<ApplicationAction>>,
    pub binds: Rc<RefCell<Vec<SherlockRowBind>>>,
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
