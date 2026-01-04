mod imp;

use gio::glib::{SignalHandlerId, WeakRef, object::ObjectExt, property::PropertySet};
use glib::Object;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{
    Box, GestureClick, glib,
    prelude::{GestureSingleExt, WidgetExt},
};
use serde::Deserialize;

use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    launcher::emoji_picker::SkinTone,
};

glib::wrapper! {
    pub struct EmojiObject(ObjectSubclass<imp::EmojiObject>)
        @extends gtk4::Box;
}
/// For deserialization
#[derive(Deserialize, Default, Debug)]
pub struct EmojiRaw {
    emoji: String,
    name: String,
    skin: u8,
}
impl EmojiRaw {
    pub fn reconstruct(&self, skin_tones: &[&str]) -> String {
        let mut result = self.emoji.to_string();
        for tone in skin_tones {
            if let Some(pos) = result.find("{skin_tone}") {
                result.replace_range(pos..pos + "{skin_tone}".len(), tone);
            }
        }
        result
    }
    pub fn emoji(&self) -> String {
        self.emoji.to_string()
    }
}

impl EmojiObject {
    // Setters
    pub fn set_parent(&self, parent: WeakRef<Box>) {
        let imp = self.imp();
        if let Some(gesture) = imp.gesture.get() {
            if let Some(old_parent) = imp.parent.borrow().as_ref().and_then(|tmp| tmp.upgrade()) {
                old_parent.remove_controller(gesture);
            }
            if let Some(tmp) = parent.upgrade() {
                tmp.add_controller(gesture.clone())
            }
        }
        *self.imp().parent.borrow_mut() = Some(parent);
    }
    fn unset_parent(&self) {
        let imp = self.imp();
        if let Some(gesture) = imp.gesture.get()
            && let Some(parent) = imp.parent.borrow().as_ref().and_then(|tmp| tmp.upgrade())
        {
            parent.remove_controller(gesture);
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
                execute_from_attrs(&parent, &attrs, None, None);
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
        self.imp().emoji.borrow().name.to_string()
    }
    /// Skin colors for emojies are defined as:
    /// ["\u{1F3FB}", "\u{1F3FC}", "\u{1F3FD}", "\u{1F3FE}", "\u{1F3FF}"]
    pub fn emoji(&self) -> String {
        let imp = self.imp();
        let default = imp.default_skin_tone.get().get_ascii();
        imp.emoji.borrow().reconstruct(&[default, default])
    }

    pub fn num_actions(&self) -> u8 {
        self.imp().emoji.borrow().skin
    }

    pub fn from(emoji_data: EmojiRaw, skin: &SkinTone) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.default_skin_tone.set(*skin);

        imp.gesture.get_or_init(|| {
            let gesture = GestureClick::new();
            let obj = obj.downgrade();
            gesture.set_button(0);
            gesture.connect_pressed({
                move |_, n_clicks, _, _| {
                    if n_clicks >= 2
                        && let Some(obj) = obj.upgrade()
                    {
                        obj.emit_by_name::<()>("emoji-should-activate", &[]);
                    }
                }
            });
            gesture
        });
        imp.emoji.set(emoji_data);
        obj
    }
    pub fn new() -> Self {
        Object::builder().build()
    }
}
