mod imp;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, variant::ToVariant, SignalHandlerId, WeakRef};
use glib::Object;
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
    pub fn focus_next(&self) -> Option<()> {
        let imp = self.imp();
        let new_index = imp.index.get().checked_add(1)?.clamp(0, 5);
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
        Some(())
    }
    pub fn focus_prev(&self) -> Option<()> {
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
        Some(())
    }
    pub fn new(parent: WeakRef<EmojiObject>, color_index: u8, default_skin_tone: u8) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();

        // Construct raw emoji
        let mut emoji_raw = parent
            .upgrade()
            .map(|i| i.imp().emoji.borrow().emoji())
            .unwrap_or_default();
        let mut index = 0;
        let mut count = 0;
        let pattern = "{skin_tone}";
        while let Some(pos) = emoji_raw[index..].find(pattern) {
            let abs_pos = index + pos;
            if count == color_index {
                let tone = "{current}";
                emoji_raw.replace_range(abs_pos..abs_pos + pattern.len(), tone);
                index = abs_pos + tone.len(); // skip over inserted tone
            } else {
                emoji_raw.replace_range(abs_pos..abs_pos + pattern.len(), "");
                index = abs_pos; // continue from the same index since we removed the pattern
            }
            count += 1;
        }

        imp.index.set(default_skin_tone);
        let _ = imp.parent.set(parent);
        let tones = [
            "",
            "\u{1F3FB}",
            "\u{1F3FC}",
            "\u{1F3FD}",
            "\u{1F3FE}",
            "\u{1F3FF}",
        ];
        imp.tones
            .borrow()
            .iter()
            .filter_map(|i| i.upgrade())
            .enumerate()
            .for_each(|(i, label)| {
                if i as u8 == default_skin_tone {
                    label.add_css_class("active");
                }
                label.set_text(&emoji_raw.replace("{current}", tones.get(i).unwrap_or(&"")));
            });

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
