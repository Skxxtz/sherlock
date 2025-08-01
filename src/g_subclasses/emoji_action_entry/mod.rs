mod imp;

use std::usize;

use gio::glib::{object::ObjectExt, variant::ToVariant, SignalHandlerId, WeakRef};
use glib::Object;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{glib, prelude::WidgetExt};
use simd_json::prelude::ArrayTrait;

use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    g_subclasses::emoji_item::EmojiObject,
};

glib::wrapper! {
    pub struct EmojiContextAction(ObjectSubclass<imp::EmojiContextAction>)
        @extends gtk4::Box, gtk4::Widget;
}

impl EmojiContextAction {
    pub fn set_signal_id(&self, signal: SignalHandlerId) {
        // Take the previous signal if it exists and disconnect it
        if let Some(old_id) = self.imp().signal_id.borrow_mut().take() {
            self.disconnect(old_id);
        }
        *self.imp().signal_id.borrow_mut() = Some(signal);
    }
    pub fn get_row(&self) -> Option<&WeakRef<EmojiObject>> {
        self.imp().parent.get()
    }
    pub fn index(&self) -> u8 {
        self.imp().index.get()
    }
    pub fn focus_next(&self) -> Option<u8> {
        let imp = self.imp();
        let new_index = imp.index.get().checked_add(1)?.clamp(0, 4);
        for (i, item) in imp
            .tones
            .borrow()
            .iter()
            .filter_map(|i| i.upgrade())
            .enumerate()
        {
            if i as u8 == new_index {
                imp.index.set(new_index);
                item.add_css_class("active");
            } else {
                item.remove_css_class("active");
            }
        }
        Some(new_index)
    }
    pub fn focus_prev(&self) -> Option<u8> {
        let imp = self.imp();
        let new_index = imp.index.get().checked_sub(1)?;
        for (i, item) in imp
            .tones
            .borrow()
            .iter()
            .filter_map(|i| i.upgrade())
            .enumerate()
        {
            if i as u8 == new_index {
                imp.index.set(new_index);
                item.add_css_class("active");
            } else {
                item.remove_css_class("active");
            }
        }
        Some(new_index)
    }
    pub fn update_index(&self, color_index: u8, new_index: u8, uniform: bool) -> Option<usize> {
        let imp = self.imp();
        let parent = imp.parent.get().and_then(|tmp| tmp.upgrade());

        let tones = [
            "\u{1F3FB}",
            "\u{1F3FC}",
            "\u{1F3FD}",
            "\u{1F3FE}",
            "\u{1F3FF}",
        ];
        let default = tones.get(new_index as usize).unwrap_or(&"\u{1F3FD}");

        // Construct raw emoji
        let mut emoji_raw = parent
            .map(|i| i.imp().emoji.borrow().emoji())
            .unwrap_or_default();
        let pattern = "{skin_tone}";
        let mut count = 0;
        let mut result = String::new();
        let mut remaining = emoji_raw.as_str();

        while let Some(pos) = remaining.find(pattern) {
            result.push_str(&remaining[..pos]);
            if uniform {
                if count == color_index {
                    result.push_str(default);
                } else {
                    result.push_str("{current}");
                }
            } else {
                if count == color_index {
                    result.push_str("{current}");
                } else {
                    result.push_str(default);
                }
            }

            remaining = &remaining[pos + pattern.len()..];
            count += 1;
        }

        result.push_str(remaining);
        emoji_raw = result;

        imp.tones
            .borrow()
            .iter()
            .filter_map(|i| i.upgrade())
            .enumerate()
            .for_each(|(i, label)| {
                label.set_text(&emoji_raw.replace("{current}", tones.get(i).unwrap_or(&"")));
            });

        Some(new_index as usize)
    }
    pub fn new(parent: WeakRef<EmojiObject>, color_index: u8, default_skin_tone: u8) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        let _ = imp.parent.set(parent);

        if let Some(new_index) = obj.update_index(color_index, default_skin_tone, false) {
            imp.index.set(default_skin_tone);
            imp.tones
                .borrow()
                .get(new_index)
                .and_then(|tmp| tmp.upgrade())
                .map(|tmp| tmp.add_css_class("active"));
        }

        let signal_id = obj.connect_local("context-action-should-activate", false, {
            move |row| {
                let row = row.first().map(|f| f.get::<EmojiContextAction>().ok())??;
                let attrs = get_attrs_map(vec![("method", Some("copy"))]);
                execute_from_attrs(&row, &attrs, None, None);
                // To reload ui according to mode
                let _ = row.activate_action("win.update-items", Some(&false.to_variant()));
                None
            }
        });
        *imp.signal_id.borrow_mut() = Some(signal_id);

        obj
    }
}
