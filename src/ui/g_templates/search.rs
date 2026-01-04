mod imp {
    use gtk4::CompositeTemplate;
    use gtk4::subclass::prelude::*;
    use gtk4::{Box as GtkBox, Label};
    use gtk4::{Entry, Image, ListView, ScrolledWindow, glib};

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

use gtk4::{glib, prelude::WidgetExt, subclass::prelude::ObjectSubclassIsExt};
glib::wrapper! {
    pub struct SearchUiObj(ObjectSubclass<imp::SearchUiObj>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl SearchUiObj {
    pub fn new() -> Self {
        let ui = glib::Object::new::<Self>();
        let imp = ui.imp();
        imp.search_icon_holder.add_css_class("search");
        imp.results.set_focusable(false);
        ui
    }
}
