use gio::glib::subclass::Signal;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::glib;
use std::sync::OnceLock;


#[derive(Default)]
pub struct SherlockLazyBox { }

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SherlockLazyBox {
    const NAME: &'static str = "CustomSherlockLazyBox";
    type Type = super::SherlockLazyBox;
    type ParentType = gtk4::Box;
}

// Trait shared by all GObjects
impl ObjectImpl for SherlockLazyBox {
    fn constructed(&self) {
        self.parent_constructed();
    }
    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
        SIGNALS.get_or_init(|| {
            vec![Signal::builder("row-should-activate")
                .param_types([u8::static_type(), String::static_type()])
                .build()]
        })
    }
}

// Make SherlockRow function with `IsA widget and ListBoxRow`
impl WidgetImpl for SherlockLazyBox {}
impl BoxImpl for SherlockLazyBox {}
