mod imp {
    use gtk4::subclass::prelude::*;
    use gtk4::{ApplicationWindow, glib};
    use gtk4::{Box, CompositeTemplate, Label, Spinner, Stack};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/window.ui")]
    pub struct MainWindow {
        #[template_child(id = "stack")]
        pub stack: TemplateChild<Stack>,

        // Status bar and its children
        #[template_child(id = "status-bar")]
        pub status_bar: TemplateChild<Box>,
        #[template_child(id = "context-menu-desc")]
        pub context_action_desc: TemplateChild<Label>,
        #[template_child(id = "context-menu-first")]
        pub context_action_first: TemplateChild<Label>,
        #[template_child(id = "context-menu-second")]
        pub context_action_second: TemplateChild<Label>,
        #[template_child(id = "status-bar-spinner")]
        pub spinner: TemplateChild<Spinner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MainWindow {
        const NAME: &'static str = "MainWindow";
        type Type = super::MainWindow;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MainWindow {}
    impl WidgetImpl for MainWindow {}
    impl WindowImpl for MainWindow {}
    impl ApplicationWindowImpl for MainWindow {}
}

use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{
    Application, glib,
    prelude::{GtkWindowExt, WidgetExt},
};

use crate::ui::util::ConfKeys;
use crate::utils::config::ConfigGuard;

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager, gio::ActionGroup, gio::ActionMap;
}

impl MainWindow {
    pub fn new(application: &Application, width: i32, opacity: f64) -> Self {
        let obj = glib::Object::new::<Self>();
        let imp = obj.imp();

        if let Ok(config) = ConfigGuard::read()
            && config.expand.enable
        {
            obj.set_valign(gtk4::Align::Start);
        }

        let custom_binds = ConfKeys::new();
        if let Some(context_str) = &custom_binds.context_str {
            imp.context_action_first
                .set_text(&custom_binds.context_mod_str);
            imp.context_action_second.set_text(context_str);
        } else {
            imp.context_action_first.set_visible(false);
            imp.context_action_second.set_visible(false);
        }

        obj.set_opacity(opacity);
        obj.set_default_width(width);
        obj.set_application(Some(application));

        obj
    }
}
