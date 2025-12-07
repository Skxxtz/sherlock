use std::cell::RefCell;
use std::rc::Rc;

use gtk4::subclass::prelude::*;
use gtk4::{glib, Box as GtkBox, Entry, EventControllerFocus};
use gtk4::{prelude::*, EventControllerKey};

use crate::ui::event_port::EventPort;

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct ArgBar {
        pub container: RefCell<Option<GtkBox>>,
        pub entry: RefCell<Option<Entry>>,
        pub index: Cell<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArgBar {
        const NAME: &'static str = "ArgBar";
        type Type = super::ArgBar;
        type ParentType = GtkBox; // Container as parent
    }

    impl ObjectImpl for ArgBar {}
    impl WidgetImpl for ArgBar {}
    impl BoxImpl for ArgBar {}
}

glib::wrapper! {
    pub struct ArgBar(ObjectSubclass<imp::ArgBar>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl ArgBar {
    pub fn new(index: u8, placeholder: &str, event_port: Rc<RefCell<EventPort>>) -> Self {
        let obj: Self = glib::Object::new::<Self>();

        // Create internal container
        let container = GtkBox::new(gtk4::Orientation::Horizontal, 4);
        container.set_margin_start(6);
        container.set_margin_end(6);
        container.set_margin_top(2);
        container.set_margin_bottom(2);
        container.set_css_classes(&["arg-box"]);

        // Create the entry
        let entry = Entry::builder()
            .placeholder_text(placeholder)
            .css_classes(["arg-bar"])
            .max_width_chars(1)
            .hexpand(false)
            .build();
        entry.set_invisible_char(Some('●'));

        // Pack everything
        container.append(&entry);
        obj.append(&container);
        Self::adjust_width(&entry, placeholder);

        // Save references in the subclass (optional, if you need them later)
        let imp = obj.imp();
        imp.container.borrow_mut().replace(container);
        imp.entry.borrow_mut().replace(entry);
        imp.index.set(index);

        obj.add_changed_event();
        obj.add_key_press(event_port);

        obj
    }

    pub fn as_pwd(self) -> Self {
        self.entry().map(|e| e.set_visibility(false));
        self
    }

    /// Call this whenever text changes
    pub fn adjust_width(entry: &Entry, text: &str) {
        // Keep the layout in a variable to extend its lifetime
        let text = if text.is_empty() {
            entry
                .placeholder_text()
                .map(|s| s.to_string())
                .unwrap_or_default()
        } else {
            if gtk4::prelude::EntryExt::is_visible(entry) {
                text.to_string()
            } else {
                "●".repeat(text.chars().count())
            }
        };

        let layout = entry.create_pango_layout(Some(&text));
        let (w, h) = layout.size(); // use while layout is still alive
        let hpx = w / gtk4::pango::SCALE;
        let vpx = h / gtk4::pango::SCALE;

        entry.set_size_request(hpx + 1, vpx + 1);
    }

    pub fn add_changed_event(&self) {
        if let Some(entry) = self.imp().entry.borrow().as_ref() {
            entry.connect_changed(|bar| {
                Self::adjust_width(bar, bar.text().as_str());
            });
        }
    }
    pub fn add_key_press(&self, event_port: Rc<RefCell<EventPort>>) {
        if let Some(entry) = self.entry() {
            let event_controller = EventControllerKey::new();
            event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
            event_controller.connect_key_pressed({
                let event_port = Rc::clone(&event_port);
                move |_, key, _, mods| {
                    let event_string = EventPort::key_event_string(key, mods);
                    if event_string.is_empty() {
                        return false.into();
                    }
                    let port_clone = event_port.clone();
                    event_port
                        .borrow()
                        .handle_key_event(&event_string, port_clone)
                        .into()
                }
            });
            entry.add_controller(event_controller);

            let event_controller = EventControllerFocus::new();
            event_controller.connect_enter({
                let event_port = Rc::clone(&event_port);
                let index = self.imp().index.get();
                move |_| {
                    let ui = event_port.borrow().ui();
                    if let Some(ui) = ui.upgrade() {
                        ui.set_bar_index(index)
                    }
                }
            });
            entry.add_controller(event_controller);
        }
    }

    /// Access the internal entry if needed
    pub fn entry(&self) -> Option<Entry> {
        self.imp().entry.borrow().clone()
    }
}
