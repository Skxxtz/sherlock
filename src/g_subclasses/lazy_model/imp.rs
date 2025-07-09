use gio::glib::subclass::Signal;
use gio::glib::WeakRef;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::glib;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

#[derive(Default)]
pub struct SherlockLazyBox {
    /// Holds all child elements
    pub children: RefCell<Vec<WeakRef<gtk4::Widget>>>,
    /// Holds the capacity this object can hold
    pub max_items: OnceCell<usize>,
    /// Holds the number of visible children
    pub visible_children: Cell<usize>
}

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

