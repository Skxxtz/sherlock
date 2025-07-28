use gio::glib::object::ObjectExt;
use gio::glib::subclass::Signal;
use gio::glib::SignalHandlerId;
use gtk4::prelude::*;
use gtk4::prelude::{GestureSingleExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{glib, GestureClick};
use once_cell::unsync::OnceCell;
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::loader::util::ApplicationAction;

/// ## Fields:
/// * **active**: Whether the row should be shown as active in multi selection
/// * **gesture**: State to hold and replace double-click gestures.
/// * **actions**: Additional actions this tile has
/// * **num_actions**: Number of additional actions
/// * **terminal**: If the app should be executed using the terminal
#[derive(Default)]
pub struct SherlockRow {
    /// Whether the row should be shown as active in multi selection
    pub active: Cell<bool>,

    /// State to hold and replace double-click gestures
    pub gesture: OnceCell<GestureClick>,

    /// State to hold and replace activate signale
    pub signal_id: RefCell<Option<SignalHandlerId>>,

    /// * **actions**: Additional actions this tile has
    pub actions: RefCell<Vec<ApplicationAction>>,

    /// * **num_actions**: Number of additional actions
    pub num_actions: Cell<usize>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SherlockRow {
    const NAME: &'static str = "CustomSherlockRow";
    type Type = super::SherlockRow;
    type ParentType = gtk4::Box;
}

// Trait shared by all GObjects
impl ObjectImpl for SherlockRow {
    fn constructed(&self) {
        self.parent_constructed();

        // Only install gesture once
        self.gesture.get_or_init(|| {
            let gesture = GestureClick::new();
            gesture.set_button(0);

            let obj = self.obj().downgrade();
            gesture.connect_pressed(move |_, n_clicks, _, _| {
                if n_clicks >= 2 {
                    if let Some(obj) = obj.upgrade() {
                        let exit: u8 = 0;
                        obj.emit_by_name::<()>("row-should-activate", &[&exit, &""]);
                    }
                }
            });

            self.obj().add_controller(gesture.clone());
            gesture
        });
    }
    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
        // Signal used to activate actions connected to the SherlockRow
        // u8 can either be 0, 1, 2
        // 0 => gives default
        // 1 => forces NO
        // 2 => forces YES
        SIGNALS.get_or_init(|| {
            vec![Signal::builder("row-should-activate")
                .param_types([u8::static_type(), String::static_type()])
                .build()]
        })
    }
}

// Make SherlockRow function with `IsA widget and ListBoxRow`
impl WidgetImpl for SherlockRow {}
impl BoxImpl for SherlockRow {}
