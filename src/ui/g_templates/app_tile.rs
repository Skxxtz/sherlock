mod imp {
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Image, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/tile.ui")]
    pub struct AppTile {
        #[template_child(id = "app-name")]
        pub title: TemplateChild<Label>,

        #[template_child(id = "launcher-type")]
        pub category: TemplateChild<Label>,

        #[template_child(id = "icon-name")]
        pub icon: TemplateChild<Image>,

        #[template_child(id = "icon-holder")]
        pub icon_holder: TemplateChild<GtkBox>,

        #[template_child(id = "app-name-tag-start")]
        pub tag_start: TemplateChild<Label>,

        #[template_child(id = "app-name-tag-end")]
        pub tag_end: TemplateChild<Label>,

        #[template_child(id = "shortcut-holder")]
        pub shortcut_holder: TemplateChild<GtkBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppTile {
        const NAME: &'static str = "AppTile";
        type Type = super::AppTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppTile {}
    impl WidgetImpl for AppTile {}
    impl BoxImpl for AppTile {}
}

use gtk4::glib;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::utils::config::ConfigGuard;

glib::wrapper! {
    pub struct AppTile(ObjectSubclass<imp::AppTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl AppTile {
    pub fn new() -> Self {
        let obj = glib::Object::new::<Self>();
        if let Ok(config) = ConfigGuard::read() {
            let imp = obj.imp();
            imp.icon.set_pixel_size(config.appearance.icon_size);
        }
        obj
    }
}
