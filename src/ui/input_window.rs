mod imp {
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{glib, ApplicationWindow, Entry};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/input_window.ui")]
    pub struct InputWindow {
        #[template_child(id = "input")]
        pub input: TemplateChild<Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InputWindow {
        const NAME: &'static str = "InputWindow";
        type Type = super::InputWindow;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InputWindow {}
    impl WidgetImpl for InputWindow {}
    impl WindowImpl for InputWindow {}
    impl ApplicationWindowImpl for InputWindow {}
}

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::object::ObjectExt;
use gtk4::{
    gdk::Key,
    glib,
    prelude::{EditableExt, EntryExt, EventControllerExt, GtkWindowExt, WidgetExt},
    EventControllerKey,
};
use gtk4_layer_shell::LayerShell;

glib::wrapper! {
    pub struct InputWindow(ObjectSubclass<imp::InputWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gtk4::Buildable;
}

impl InputWindow {
    pub fn new(obfuscate: bool) -> Self {
        let obj = glib::Object::new::<Self>();
        let imp = obj.imp();

        obj.init_layer_shell();
        obj.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
        obj.set_layer(gtk4_layer_shell::Layer::Overlay);

        imp.input.set_visibility(obfuscate == false);
        let event_controller = EventControllerKey::new();
        event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
        event_controller.connect_key_pressed({
            let obj = obj.downgrade();
            let input = imp.input.downgrade();
            move |_, key, _, _mods| match key {
                Key::Escape => {
                    if let Some(win) = obj.upgrade() {
                        win.close();
                    }
                    true.into()
                }
                Key::Return => {
                    if let Some(input) = input.upgrade() {
                        print!("{}", input.text());
                    }
                    if let Some(win) = obj.upgrade() {
                        win.close()
                    }
                    true.into()
                }
                _ => false.into(),
            }
        });
        imp.input.add_controller(event_controller);

        obj.connect_map(move |myself| {
            let imp = myself.imp();
            imp.input.grab_focus();
        });

        obj
    }
}
