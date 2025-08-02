mod imp {
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/calc_tile.ui")]
    pub struct CalcTile {
        #[template_child(id = "equation-holder")]
        pub equation_holder: TemplateChild<Label>,

        #[template_child(id = "result-holder")]
        pub result_holder: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CalcTile {
        const NAME: &'static str = "CalcTile";
        type Type = super::CalcTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CalcTile {}
    impl WidgetImpl for CalcTile {}
    impl BoxImpl for CalcTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct CalcTile(ObjectSubclass<imp::CalcTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl CalcTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}
