use gio::glib::object::ObjectExt;
use gio::glib::subclass::Signal;
use gio::glib::{SignalHandlerId, WeakRef};
use gtk4::prelude::{BoxExt, GestureSingleExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{GestureClick, Label, glib, prelude::*};
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::g_subclasses::emoji_item::EmojiObject;

/// ## Fields:
#[derive(Default)]
pub struct EmojiContextAction {
    /// The command for this action
    pub exec: RefCell<String>,
    /// If the command should be executed using the terminal
    pub terminal: Cell<bool>,
    /// Internal handler for gestures
    pub gesture: OnceCell<GestureClick>,
    pub signal_id: RefCell<Option<SignalHandlerId>>,

    pub index: Cell<u8>,
    pub tones: RefCell<Vec<WeakRef<Label>>>,

    pub parent: OnceCell<WeakRef<EmojiObject>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for EmojiContextAction {
    const NAME: &'static str = "EmojiContextAction";
    type Type = super::EmojiContextAction;
    type ParentType = gtk4::Box;
}

// Trait shared by all GObjects
impl ObjectImpl for EmojiContextAction {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();
        obj.set_spacing(10);

        obj.set_homogeneous(true);
        obj.set_halign(gtk4::Align::Fill);
        obj.set_valign(gtk4::Align::Center);
        obj.set_vexpand(true);
        obj.set_spacing(10);

        for _ in 0..5 {
            // Title label
            let label = Label::builder().wrap(false).single_line_mode(true).build();
            self.tones.borrow_mut().push(label.downgrade());
            obj.append(&label);
        }

        // Only install gesture once
        self.gesture.get_or_init(|| {
            let gesture = GestureClick::new();
            gesture.set_button(0);

            let obj = obj.downgrade();
            gesture.connect_pressed(move |_, n_clicks, _, _| {
                if n_clicks >= 2
                    && let Some(obj) = obj.upgrade()
                {
                    let exit: u8 = 0;
                    obj.emit_by_name::<()>("context-action-should-activate", &[&exit, &""]);
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
            vec![
                Signal::builder("context-action-should-activate")
                    .param_types([u8::static_type()])
                    .build(),
            ]
        })
    }
}

// Make SherlockRow function with `IsA widget and ListBoxRow`
impl WidgetImpl for EmojiContextAction {}
impl BoxImpl for EmojiContextAction {}
