use gio::glib::object::ObjectExt;
use gio::glib::subclass::Signal;
use gio::glib::{SignalHandlerId, WeakRef};
use gtk4::prelude::*;
use gtk4::prelude::{GestureSingleExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{glib, GestureClick};
use once_cell::unsync::OnceCell;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::OnceLock;

use crate::g_subclasses::sherlock_row::SherlockRowBind;
use crate::launcher::utils::HomeType;
use crate::loader::util::ApplicationAction;

/// ## Fields:
/// * **spawn_focus**: Whether the tile should receive focus when Sherlock starts.
/// * **active**: Whether the row should be shown as active in multi selection
/// * **gesture**: State to hold and replace double-click gestures.
/// * **shortcut_holder**: A `GtkBox` widget that holds the `modkey + number` shortcut indicators.
/// * **priority**: Determines the tile's ordering within the `GtkListView`.
/// * **search**: The string used to compute Levenshtein distance for this tile.
/// * **alias**: The display mode in which this tile should appear.
/// * **home**: Whether the tile should appear on the home screen (i.e., when the search entry is empty and mode is `all`).
/// * **disable**: Whether the tile be forced to not show.
/// * **update**: The function used to update ui elements (i.e. calculator results or bulk text results)
/// * **keyword_aware**: Whether the tile shuold take the keyword as context
/// * **actions**: Additional actions this tile has
/// * **num_actions**: Number of additional actions
/// * **terminal**: If the app should be executed using the terminal
#[derive(Default)]
pub struct SherlockRow {
    /// Whether the tile should receive focus when Sherlock starts  
    pub spawn_focus: Cell<bool>,

    /// Whether the row should be shown as active in multi selection
    pub active: Cell<bool>,

    /// State to hold and replace double-click gestures
    pub gesture: OnceCell<GestureClick>,

    /// State to hold and replace activate signale
    pub signal_id: RefCell<Option<SignalHandlerId>>,

    /// A `GtkBox` widget that holds the `modkey + number` shortcut indicators  
    pub shortcut_holder: OnceCell<Option<WeakRef<gtk4::Box>>>,

    /// Determines the tile's ordering within the `GtkListView`  
    pub priority: Cell<f32>,

    /// The string used to compute Levenshtein distance for this tile  
    pub search: RefCell<String>,

    /// The display mode in which this tile should appear  
    pub alias: RefCell<String>,

    /// Whether the tile should appear on the home screen  
    ///             (i.e. when the search entry is empty and mode is `all`)  
    pub home: Cell<HomeType>,

    // The function used to update ui elements
    //              (i.e. calculator results)
    pub update: RefCell<Option<Box<dyn Fn(&str) -> bool>>>,

    // The function used to update async ui elements
    //              (i.e. bulk text results, mpris_tiles)
    pub async_content_update:
        RefCell<Option<Box<dyn Fn(&str) -> Pin<Box<dyn Future<Output = ()> + 'static>> + 'static>>>,

    /// Whether the tile shuold take the keyword as context
    pub keyword_aware: Cell<bool>,

    /// * **actions**: Additional actions this tile has
    pub actions: RefCell<Vec<ApplicationAction>>,

    /// * **num_actions**: Number of additional actions
    pub num_actions: Cell<usize>,

    /// * **terminal**: If this tile should be executed using the terminal
    pub terminal: Cell<bool>,

    pub binds: Rc<RefCell<Vec<SherlockRowBind>>>,
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
