mod imp;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, SignalHandlerId};
use glib::Object;
use gtk4::{
    gdk::{Key, ModifierType},
    glib,
    prelude::WidgetExt,
};
use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

use crate::loader::util::ApplicationAction;

glib::wrapper! {
    pub struct SherlockRow(ObjectSubclass<imp::SherlockRow>)
        @extends gtk4::Box, gtk4::Widget;
}

impl SherlockRow {
    pub fn new() -> Self {
        let myself: Self = Object::builder().build();
        myself.add_css_class("tile");
        myself
    }
    // setters
    pub fn set_signal_id(&self, signal: SignalHandlerId) {
        // Take the previous signal if it exists and disconnect it
        if let Some(old_id) = self.imp().signal_id.borrow_mut().replace(signal) {
            self.disconnect(old_id);
        }
    }
    pub fn clear_signal_id(&self) {
        if let Some(old) = self.imp().signal_id.borrow_mut().take() {
            self.disconnect(old);
        }
    }
    pub fn set_actions(&self, actions: Vec<ApplicationAction>) {
        self.imp().num_actions.set(actions.len());
        *self.imp().actions.borrow_mut() = actions;
    }
}

impl Default for SherlockRow {
    fn default() -> Self {
        let row = Self::new();
        row.set_css_classes(&["tile"]);
        row
    }
}

#[derive(Debug, Clone)]
pub struct SherlockRowBind {
    pub key: Option<Key>,
    pub modifier: ModifierType,
    pub callback: String,
    pub exit: Option<bool>,
}
impl<'de> Deserialize<'de> for SherlockRowBind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Temp {
            bind: String,
            callback: String,
            exit: Option<bool>,
        }

        let temp = Temp::deserialize(deserializer)?;

        // Parse bind string like "Ctrl+Shift+S"
        let mut key: Option<Key> = None;
        let mut modifier = ModifierType::empty();

        for token in temp.bind.split('+') {
            if let Some(m) = ModifierType::from_name(token) {
                modifier |= m;
            } else if key.is_none() {
                key = Key::from_name(token);
            } else {
                return Err(de::Error::custom(format!("Unknown bind token: {}", token)));
            }
        }

        Ok(SherlockRowBind {
            key,
            modifier,
            callback: temp.callback,
            exit: temp.exit,
        })
    }
}
impl Serialize for SherlockRowBind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Build the bind string
        let mut bind_parts = Vec::new();

        for name in self.modifier.iter_names() {
            bind_parts.push(name.0.to_string());
        }

        if let Some(key) = &self.key {
            if let Some(name) = key.name() {
                bind_parts.push(name.to_string());
            }
        }

        let bind = bind_parts.join("+");

        // Start serializing
        let mut state = serializer.serialize_struct("SherlockRowBind", 3)?;
        state.serialize_field("bind", &bind)?;
        state.serialize_field("callback", &self.callback)?;
        if let Some(exit) = &self.exit {
            state.serialize_field("exit", exit)?;
        }
        state.end()
    }
}
