mod imp;

use std::rc::Rc;

use gtk4::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk4::{
    glib,
};

use crate::launcher::Launcher;

glib::wrapper! {
    pub struct TileItem(ObjectSubclass<imp::TileItem>);
}

impl TileItem {
    pub fn from(launcher: Rc<Launcher>) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();

        obj
    }
    pub fn new() -> Self {
        Object::builder().build()
    }
}
