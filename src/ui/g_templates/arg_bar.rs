use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{glib, Box as GtkBox, Entry};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct ArgBar {
        pub container: RefCell<Option<GtkBox>>,
        pub entry: RefCell<Option<Entry>>,
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
    pub fn new(placeholder: &str) -> Self {
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

        // Pack everything
        container.append(&entry);
        obj.append(&container);
        Self::adjust_width(&entry, placeholder);

        // Save references in the subclass (optional, if you need them later)
        let imp = obj.imp();
        imp.container.borrow_mut().replace(container);
        imp.entry.borrow_mut().replace(entry);

        obj.add_changed_event();

        obj
    }

    /// Call this whenever text changes
    pub fn adjust_width(entry: &Entry, text: &str) {
        let text = if text.is_empty() {
            entry
                .placeholder_text()
                .map(|s| s.to_string())
                .unwrap_or_default()
        } else {
            text.to_string()
        };

        // Keep the layout in a variable to extend its lifetime
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

    /// Access the internal entry if needed
    pub fn entry(&self) -> Option<Entry> {
        self.imp().entry.borrow().clone()
    }
}
