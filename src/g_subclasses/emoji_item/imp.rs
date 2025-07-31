use gio::glib::subclass::Signal;
use gio::glib::{SignalHandlerId, WeakRef};
use gtk4::subclass::prelude::*;
use gtk4::{glib, GestureClick};
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use crate::g_subclasses::emoji_item::EmojiRaw;
use crate::launcher::emoji_picker::EmojiPicker;

/// ## Fields:
#[derive(Default, Debug)]
pub struct EmojiObject {
    pub launcher: Rc<RefCell<EmojiPicker>>,
    pub emoji: RefCell<EmojiRaw>,
    pub parent: RefCell<Option<WeakRef<gtk4::Box>>>,

    // Internal
    pub gesture: OnceCell<GestureClick>,
    pub signal_id: RefCell<Option<SignalHandlerId>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for EmojiObject {
    const NAME: &'static str = "EmojiObject";
    type Type = super::EmojiObject;
    type ParentType = glib::Object;
}

// Trait shared by all GObjects
impl ObjectImpl for EmojiObject {
    fn constructed(&self) {
        self.parent_constructed();
    }
    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
        // Signal used to activate actions connected to the Emoji
        SIGNALS.get_or_init(|| vec![Signal::builder("emoji-should-activate").build()])
    }
}
