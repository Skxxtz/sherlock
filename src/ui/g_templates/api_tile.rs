mod imp {
    use gtk4::CompositeTemplate;
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::{Box as GtkBox, Image, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/bulk_text_tile.ui")]
    pub struct ApiTile {
        #[template_child(id = "launcher-type")]
        pub category: TemplateChild<Label>,

        #[template_child(id = "icon-name")]
        pub icon: TemplateChild<Image>,

        #[template_child(id = "content-title")]
        pub content_title: TemplateChild<Label>,

        #[template_child(id = "content-body")]
        pub content_body: TemplateChild<Label>,

        #[template_child(id = "shortcut-holder")]
        pub shortcut_holder: TemplateChild<GtkBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApiTile {
        const NAME: &'static str = "ApiTile";
        type Type = super::ApiTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ApiTile {}
    impl WidgetImpl for ApiTile {}
    impl BoxImpl for ApiTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct ApiTile(ObjectSubclass<imp::ApiTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl ApiTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}
