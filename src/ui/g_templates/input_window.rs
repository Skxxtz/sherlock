mod imp {
    use std::cell::RefCell;

    use futures::channel::oneshot::Sender;
    use gtk4::CompositeTemplate;
    use gtk4::subclass::prelude::*;
    use gtk4::{ApplicationWindow, Entry, glib};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/input_window.ui")]
    pub struct InputWindow {
        #[template_child(id = "input")]
        pub input: TemplateChild<Entry>,
        pub completion: RefCell<Option<Sender<String>>>,
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

use futures::channel::oneshot::{Receiver, channel};
use gio::glib::object::ObjectExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{
    EventControllerKey,
    gdk::Key,
    glib,
    prelude::{EditableExt, EntryExt, EventControllerExt, GtkWindowExt, WidgetExt},
};
use gtk4_layer_shell::LayerShell;

glib::wrapper! {
    pub struct InputWindow(ObjectSubclass<imp::InputWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager, gio::ActionGroup, gio::ActionMap;
}

impl InputWindow {
    pub fn new(obfuscate: bool, placeholder: Option<&str>) -> (Self, Receiver<String>) {
        let obj = glib::Object::new::<Self>();
        let imp = obj.imp();

        obj.init_layer_shell();
        obj.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
        obj.set_layer(gtk4_layer_shell::Layer::Overlay);

        imp.input.set_visibility(!obfuscate);
        imp.input.set_placeholder_text(placeholder);

        // Create oneshot channel
        let (sender, receiver) = channel::<String>();
        imp.completion.replace(Some(sender));

        let event_controller = EventControllerKey::new();
        event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

        {
            let obj = obj.downgrade();
            let input = imp.input.downgrade();

            event_controller.connect_key_pressed({
                move |_, key, _, _mods| match key {
                    Key::Escape => {
                        if let Some(win) = obj.upgrade() {
                            let _ = win.imp().completion.borrow_mut().take();
                            win.close();
                        }
                        true.into()
                    }
                    Key::Return => {
                        if let (Some(input), Some(win)) = (input.upgrade(), obj.upgrade()) {
                            let text = input.text().to_string();
                            if let Some(sender) = obj
                                .upgrade()
                                .and_then(|o| o.imp().completion.borrow_mut().take())
                            {
                                let _ = sender.send(text);
                            }
                            win.close();
                        }
                        true.into()
                    }
                    _ => false.into(),
                }
            });
        }

        imp.input.add_controller(event_controller);

        obj.connect_map(move |myself| {
            let imp = myself.imp();
            imp.input.grab_focus();
        });

        (obj, receiver)
    }
}
