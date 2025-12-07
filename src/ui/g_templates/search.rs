mod imp {
    use std::cell::Cell;

    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{glib, Entry, Image, ListView, ScrolledWindow};
    use gtk4::{Box as GtkBox, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/search.ui")]
    pub struct SearchUiObj {
        #[template_child(id = "split-view")]
        pub all: TemplateChild<GtkBox>,

        #[template_child(id = "preview_box")]
        pub preview_box: TemplateChild<GtkBox>,

        #[template_child(id = "search-bar")]
        pub search_bar: TemplateChild<Entry>,

        #[template_child(id = "scrolled-window")]
        pub result_viewport: TemplateChild<ScrolledWindow>,

        #[template_child(id = "category-type-holder")]
        pub mode_title_holder: TemplateChild<GtkBox>,

        #[template_child(id = "category-type-label")]
        pub mode_title: TemplateChild<Label>,

        #[template_child(id = "search-icon-holder")]
        pub search_icon_holder: TemplateChild<GtkBox>,

        #[template_child(id = "search-icon")]
        pub search_icon: TemplateChild<Image>,

        #[template_child(id = "search-icon-back")]
        pub search_icon_back: TemplateChild<Image>,

        #[template_child(id = "result-frame")]
        pub results: TemplateChild<ListView>,

        #[template_child(id = "arg-holder")]
        pub arg_holder: TemplateChild<GtkBox>,

        #[template_child(id = "search-bar-holder")]
        pub search_bar_holder: TemplateChild<GtkBox>,

        pub bar_index: Cell<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchUiObj {
        const NAME: &'static str = "SearchUI";
        type Type = super::SearchUiObj;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchUiObj {}
    impl WidgetImpl for SearchUiObj {}
    impl BoxImpl for SearchUiObj {}
}

use std::{cell::RefCell, rc::Rc, usize};

use gio::glib::object::Cast;
use gtk4::{
    glib,
    prelude::{BoxExt, EditableExt, EntryExt, WidgetExt},
    subclass::prelude::ObjectSubclassIsExt,
    Entry,
};

use crate::ui::{event_port::EventPort, g_templates::ArgBar};
glib::wrapper! {
    pub struct SearchUiObj(ObjectSubclass<imp::SearchUiObj>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl SearchUiObj {
    pub fn new() -> Self {
        let ui = glib::Object::new::<Self>();
        let imp = ui.imp();
        imp.search_icon_holder.add_css_class("search");
        imp.results.set_focusable(false);
        imp.search_bar_holder.set_focusable(true);
        imp.search_bar_holder.set_can_focus(true);
        if let Some(placeholder) = imp.search_bar.placeholder_text() {
            imp.search_bar.set_max_width_chars(placeholder.len() as i32);
        }
        ui
    }
    pub fn add_arg_bar(&self, index: u8, placeholder: &str, event_port: Rc<RefCell<EventPort>>) {
        let imp = self.imp();
        let entry = ArgBar::new(index, placeholder, event_port);
        imp.arg_holder.append(&entry);
    }
    pub fn add_pwd_bar(&self, index: u8, placeholder: &str, event_port: Rc<RefCell<EventPort>>) {
        let imp = self.imp();
        let entry = ArgBar::new(index, placeholder, event_port).as_pwd();
        imp.arg_holder.append(&entry);
    }
    pub fn remove_arg_bars(&self) {
        let imp = self.imp();
        while let Some(child) = imp.arg_holder.last_child() {
            imp.arg_holder.remove(&child);
        }
        imp.bar_index.set(0);
    }
    pub fn bar_index(&self) -> usize {
        self.imp().bar_index.get() as usize
    }
    pub fn set_bar_index(&self, index: u8) {
        self.imp().bar_index.set(index);
    }
    pub fn current_bar(&self) -> Option<Entry> {
        let imp = self.imp();
        let index = self.bar_index();
        if index == 0 {
            Some(imp.search_bar.get())
        } else if index > 0 {
            let mut child = imp.arg_holder.first_child();
            for _ in 0..index - 1 {
                child = child.and_then(|c| c.next_sibling());
            }
            child?.downcast::<ArgBar>().ok()?.entry()
        } else {
            None
        }
    }
    pub fn focus_next_arg_bar(&self) {
        let index = self.bar_index();
        if index == u8::MAX as usize {
            return;
        }
        self.focus_nth_bar(index + 1);
    }
    pub fn focus_prev_arg_bar(&self) {
        let index = self.bar_index();
        if index == 0 {
            return;
        }
        self.focus_nth_bar(self.bar_index() - 1);
    }
    pub fn focus_nth_bar(&self, index: usize) {
        let imp = self.imp();
        let index = (index as u8).clamp(0, u8::MAX);
        if index == 0 {
            // focus search bar
            imp.search_bar.grab_focus();
            imp.bar_index.set(0);
        } else if index > 0 {
            let mut child = imp.arg_holder.first_child();
            for _ in 0..index - 1 {
                child = child.and_then(|c| c.next_sibling());
            }
            if let Some(child) = child {
                if let Ok(arg) = child.downcast::<ArgBar>() {
                    if let Some(entry) = arg.entry() {
                        entry.grab_focus();
                        imp.bar_index.set(index);
                    }
                }
            }
        }
    }
}
