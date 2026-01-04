mod imp {
    use gtk4::CompositeTemplate;
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::{Box as GtkBox, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/error_tile.ui")]
    pub struct ErrorTile {
        #[template_child(id = "app-name")]
        pub title: TemplateChild<Label>,

        #[template_child(id = "content-title")]
        pub content_title: TemplateChild<Label>,

        #[template_child(id = "content-body")]
        pub content_body: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorTile {
        const NAME: &'static str = "ErrorTile";
        type Type = super::ErrorTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ErrorTile {}
    impl WidgetImpl for ErrorTile {}
    impl BoxImpl for ErrorTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct ErrorTile(ObjectSubclass<imp::ErrorTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl ErrorTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}
