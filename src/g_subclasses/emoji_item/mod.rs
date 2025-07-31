mod imp;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, SignalHandlerId, WeakRef};
use glib::Object;
use gtk4::{
    glib,
    prelude::{GestureSingleExt, WidgetExt},
    Box, GestureClick,
};
use serde::Deserialize;

use crate::actions::{execute_from_attrs, get_attrs_map};

glib::wrapper! {
    pub struct EmojiObject(ObjectSubclass<imp::EmojiObject>)
        @extends gtk4::Box;
}
/// For deserialization
#[derive(Deserialize)]
pub struct EmojiRaw {
    emoji: String,
    name: String,
    skin: u8,
}

impl EmojiObject {
    // Setters
    pub fn set_parent(&self, parent: WeakRef<Box>) {
        let imp = self.imp();
        if let Some(gesture) = imp.gesture.get() {
            if let Some(old_parent) = imp.parent.borrow().as_ref().and_then(|tmp| tmp.upgrade()) {
                old_parent.remove_controller(gesture);
            }
            parent
                .upgrade()
                .map(|tmp| tmp.add_controller(gesture.clone()));
        }
        *self.imp().parent.borrow_mut() = Some(parent);
    }
    fn unset_parent(&self) {
        let imp = self.imp();
        if let Some(gesture) = imp.gesture.get() {
            if let Some(parent) = imp.parent.borrow().as_ref().and_then(|tmp| tmp.upgrade()) {
                parent.remove_controller(gesture);
            }
        }
        *self.imp().parent.borrow_mut() = None;
    }
    pub fn set_signal_id(&self, signal: SignalHandlerId) {
        self.unset_signal_id();
        *self.imp().signal_id.borrow_mut() = Some(signal);
    }
    fn unset_signal_id(&self) {
        // Take the previous signal if it exists and disconnect it
        if let Some(old_id) = self.imp().signal_id.borrow_mut().take() {
            self.disconnect(old_id);
        }
    }
    pub fn attach_event(&self) {
        let imp = self.imp();
        let signal_id = self.connect_local("emoji-should-activate", false, {
            let emoji = self.emoji();
            let parent = imp.parent.clone();
            move |_attrs| {
                let attrs = get_attrs_map(vec![("method", Some("copy")), ("result", Some(&emoji))]);
                let parent = parent.borrow().clone().and_then(|tmp| tmp.upgrade())?;
                execute_from_attrs(&parent, &attrs, None);
                None
            }
        });
        self.set_signal_id(signal_id);
    }
    pub fn clean(&self) {
        self.unset_parent();
        self.unset_signal_id();
    }

    // Getters
    pub fn title(&self) -> String {
        self.imp().title.borrow().to_string()
    }
    pub fn emoji(&self) -> String {
        self.imp().emoji.borrow().to_string()
    }

    pub fn from(emoji_data: EmojiRaw) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();

        imp.gesture.get_or_init(|| {
            let gesture = GestureClick::new();
            let obj = obj.downgrade();
            gesture.set_button(0);
            gesture.connect_pressed({
                move |_, n_clicks, _, _| {
                    if n_clicks >= 2 {
                        if let Some(obj) = obj.upgrade() {
                            obj.emit_by_name::<()>("emoji-should-activate", &[]);
                        }
                    }
                }
            });
            gesture
        });

        *imp.title.borrow_mut() = emoji_data.name;
        *imp.emoji.borrow_mut() = emoji_data.emoji;
        obj
    }
    pub fn new() -> Self {
        Object::builder().build()
    }
}
