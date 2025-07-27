use gio::glib::object::ObjectExt;
use gio::glib::subclass::Signal;
use gio::glib::{SignalHandlerId, WeakRef};
use gtk4::prelude::{BoxExt, GestureSingleExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{glib, prelude::*, GestureClick, Image, Label};
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::g_subclasses::sherlock_row::SherlockRow;

/// ## Fields:
#[derive(Default)]
pub struct ContextAction {
    /// The command for this action
    pub exec: RefCell<String>,
    /// If the command should be executed using the terminal
    pub terminal: Cell<bool>,
    /// Internal handler for gestures
    pub gesture: OnceCell<GestureClick>,
    pub signal_id: RefCell<Option<SignalHandlerId>>,

    pub icon: OnceCell<WeakRef<Image>>,
    pub modkey: OnceCell<WeakRef<Label>>,
    pub title: OnceCell<WeakRef<Label>>,

    pub parent: OnceCell<WeakRef<SherlockRow>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ContextAction {
    const NAME: &'static str = "ContextAction";
    type Type = super::ContextAction;
    type ParentType = gtk4::Box;
}

// Trait shared by all GObjects
impl ObjectImpl for ContextAction {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();
        obj.set_spacing(10);
        // Modkey label
        let icon = Image::new();
        icon.set_halign(gtk4::Align::Start);
        icon.set_valign(gtk4::Align::Center);
        obj.append(&icon);
        self.icon.set(icon.downgrade()).unwrap();

        // Modkey label
        let modkey = Label::builder()
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Center)
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .vexpand(true)
            .build();
        obj.append(&modkey);
        self.modkey.set(modkey.downgrade()).unwrap();

        // Title label
        let title = Label::builder()
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Center)
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .vexpand(true)
            .build();
        obj.append(&title);
        self.title.set(title.downgrade()).unwrap();

        // Only install gesture once
        self.gesture.get_or_init(|| {
            let gesture = GestureClick::new();
            gesture.set_button(0);

            let obj = obj.downgrade();
            gesture.connect_pressed(move |_, n_clicks, _, _| {
                if n_clicks >= 2 {
                    if let Some(obj) = obj.upgrade() {
                        let exit: u8 = 0;
                        obj.emit_by_name::<()>("context-action-should-activate", &[&exit, &""]);
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
            vec![Signal::builder("context-action-should-activate")
                .param_types([u8::static_type()])
                .build()]
        })
    }
}

// Make SherlockRow function with `IsA widget and ListBoxRow`
impl WidgetImpl for ContextAction {}
impl BoxImpl for ContextAction {}
