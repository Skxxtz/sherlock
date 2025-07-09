mod imp;

use glib::Object;
use gtk4::{
    glib,
    prelude::WidgetExt,
};

glib::wrapper! {
    pub struct SherlockLazyBox(ObjectSubclass<imp::SherlockLazyBox>)
        @extends gtk4::Box, gtk4::Widget;
}

impl SherlockLazyBox {
    pub fn new() -> Self {
        let myself: Self = Object::builder().build();
        myself.add_css_class("tile");
        myself
    }
}

impl Default for SherlockLazyBox {
    fn default() -> Self {
        let row = Self::new();
        row
    }
}
