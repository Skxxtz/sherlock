mod imp {
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Image, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/event_tile.ui")]
    pub struct EventTile {
        #[template_child(id = "title-label")]
        pub title: TemplateChild<Label>,

        #[template_child(id = "time-label")]
        pub start_time: TemplateChild<Label>,

        #[template_child(id = "end-time-label")]
        pub end_time: TemplateChild<Label>,

        #[template_child(id = "icon-name")]
        pub icon: TemplateChild<Image>,

        #[template_child(id = "shortcut-holder")]
        pub shortcut_holder: TemplateChild<GtkBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EventTile {
        const NAME: &'static str = "EventTile";
        type Type = super::EventTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EventTile {}
    impl WidgetImpl for EventTile {}
    impl BoxImpl for EventTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct EventTile(ObjectSubclass<imp::EventTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl EventTile {
    pub fn new() -> Self {
        let obj = glib::Object::new::<Self>();
        obj
    }
}
