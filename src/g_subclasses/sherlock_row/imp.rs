use gtk4::prelude::{GestureSingleExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{glib, GestureClick};
use std::cell::Cell;
use std::collections::HashMap;

use crate::actions::execute_from_attrs;
use crate::ui::util::get_row_attrs;
// Object holding the state
#[derive(Default)]
pub struct SherlockRow {
    pub spawn_focus: Cell<bool>,
    pub shortcut: Cell<bool>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SherlockRow {
    const NAME: &'static str = "MyGtkAppCustomButton";
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
                    let attrs: HashMap<String, String> = get_row_attrs(&obj_clone);
                    execute_from_attrs(attrs);
                }
            }
        });

        obj.add_controller(gesture);
    }
}

// Make SherlockRow function with `IsA widget and ListBoxRow`
impl WidgetImpl for SherlockRow {}
impl ListBoxRowImpl for SherlockRow {}
