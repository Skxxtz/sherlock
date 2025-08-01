use gtk4::subclass::prelude::ObjectSubclassIsExt;

mod imp {
    use std::cell::RefCell;

    use gio::glib::{SignalHandlerId, SourceId, WeakRef};
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{glib, Picture};
    use gtk4::{Box as GtkBox, Label};

    use crate::g_subclasses::sherlock_row::SherlockRow;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/timer_tile.ui")]
    pub struct TimerTile {
        #[template_child(id = "timer_title")]
        pub timer_title: TemplateChild<Label>,

        #[template_child(id = "remaining_time")]
        pub remaining_label: TemplateChild<Label>,

        #[template_child(id = "animation")]
        pub animation: TemplateChild<Picture>,

        #[template_child(id = "shortcut-holder")]
        pub shortcut_holder: TemplateChild<GtkBox>,

        pub return_action: RefCell<Option<SignalHandlerId>>,
        pub time_out_handle: RefCell<Option<SourceId>>,
        pub parent: RefCell<WeakRef<SherlockRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimerTile {
        const NAME: &'static str = "TimerTile";
        type Type = super::TimerTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TimerTile {}
    impl WidgetImpl for TimerTile {}
    impl BoxImpl for TimerTile {}
}

use gtk4::glib;
glib::wrapper! {
    pub struct TimerTile(ObjectSubclass<imp::TimerTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl TimerTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
    pub fn clear_timeout(&self) {
        let imp = self.imp();
        if let Some(handle) = imp.time_out_handle.borrow_mut().take() {
            handle.remove();
        }
    }
    pub fn clear_action(&self) {
        let imp = self.imp();
        if let Some(parent) = imp.parent.borrow().upgrade() {
            parent.clear_signal_id();
        }
    }
}
