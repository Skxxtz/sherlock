use gio::glib::object::ObjectExt;
use gio::glib::subclass::Signal;
use gtk4::prelude::{GestureSingleExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{glib, GestureClick};
use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::sync::OnceLock;

use crate::ui::tiles::util::TileWidgets;

// SHERLOCK ROW
// Object holding the state
#[derive(Default)]
pub struct SherlockRow {
    pub spawn_focus: Cell<bool>,
    pub shortcut: Cell<bool>,
    pub priority: Cell<f32>,
    pub launcher_name: OnceCell<String>,
    pub widgets: OnceCell<TileWidgets>,
}

impl Drop for SherlockRow {
    fn drop(&mut self) {
        println!("ROW DROPPPED");
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SherlockRow {
    const NAME: &'static str = "CustomSherlockRow";
    type Type = super::SherlockRow;
    type ParentType = gtk4::ListBoxRow;
}

// Trait shared by all GObjects
impl ObjectImpl for SherlockRow {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();
        // Make Sherlock execute current row on multi click
        let gesture = GestureClick::new();
        gesture.set_button(0);
        gesture.connect_pressed({
            let obj_clone = obj.clone();
            move |_, n_clicks, _, _| {
                if n_clicks >= 2 {
                    obj_clone.emit_by_name::<()>("row-should-activate", &[]);
                }
            }
        });
        obj.add_controller(gesture);
    }
    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
        SIGNALS.get_or_init(|| vec![Signal::builder("row-should-activate").build()])
    }
}

// Make SherlockRow function with `IsA widget and ListBoxRow`
impl WidgetImpl for SherlockRow {}
impl ListBoxRowImpl for SherlockRow {}
