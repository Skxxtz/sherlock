use std::cell::RefCell;
use std::rc::Rc;

use gio::ActionEntry;
use gtk4::{prelude::*, Application, ApplicationWindow};
use gtk4::{Builder, Stack};
use gtk4_layer_shell::{Layer, LayerShell};

use crate::application::util::reload_content;
use crate::CONFIG;

use super::tiles::util::TextViewTileBuilder;

pub fn window(application: &Application) -> (ApplicationWindow, Stack, Rc<RefCell<String>>) {
    // 617 with, 593 without notification bar
    let (width, height, opacity) = CONFIG.get().map_or_else(
        || (900, 593, 1.0),
        |config| {
            (
                config.appearance.width,
                config.appearance.height,
                config.appearance.opacity,
            )
        },
    );

    let current_stack_page = Rc::new(RefCell::new(String::from("search-page")));

    let window: ApplicationWindow = ApplicationWindow::builder()
        .application(application)
        .default_width(width)
        .default_height(height)
        .resizable(false)
        .opacity(opacity.clamp(0.1, 1.0))
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);

    //Build main fame here that holds logic for stacking
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/window.ui");
    let stack: Stack = builder.object("stack").unwrap();

    // Setup action to close the window
    let page_clone = Rc::clone(&current_stack_page);
    let action_close = ActionEntry::builder("close")
        .activate(move |window: &ApplicationWindow, _, _| {
            if let Some(c) = CONFIG.get() {
                match c.behavior.daemonize {
                    true => {
                        window.hide();
                        let _ = gtk4::prelude::WidgetExt::activate_action(
                            window,
                            "win.clear-search",
                            None,
                        );
                    }
                    false => window.destroy(),
                }
            };
        })
        .build();

    // Setup action to open the window
    let stack_clone = stack.clone();
    let action_open = ActionEntry::builder("open")
        .activate(move |window: &ApplicationWindow, _, _| {
            if let Some(c) = CONFIG.get() {
                match c.behavior.daemonize {
                    true => {
                        reload_content(window, &stack_clone, &page_clone);
                        window.present();
                    }
                    false => window.present(),
                }
            };
        })
        .build();

    // Setup action to switch to a specific stack page
    let stack_clone = stack.clone();
    let page_clone = Rc::clone(&current_stack_page);
    let action_stack_switch = ActionEntry::builder("switch-page")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(move |_: &ApplicationWindow, _, parameter| {
            let parameter = parameter.and_then(|p| p.get::<String>());

            if let Some(parameter) = parameter {
                stack_clone.set_transition_type(gtk4::StackTransitionType::SlideRight);
                stack_clone.set_visible_child_name(&parameter);
                *page_clone.borrow_mut() = parameter
            }
        })
        .build();

    // Setup action to add a stackpage
    let stack_clone = stack.clone();
    let action_next_page = ActionEntry::builder("add-page")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(move |_: &ApplicationWindow, _, parameter| {
            if let Some(parameter) = parameter.and_then(|p| p.get::<String>()) {
                let builder = TextViewTileBuilder::new("/dev/skxxtz/sherlock/ui/text_view_tile.ui");
                builder.content.set_wrap_mode(gtk4::WrapMode::Word);
                let buf = builder.content.buffer();
                buf.set_text(parameter.as_ref());
                stack_clone.add_named(&builder.object, Some("next-page"));
                stack_clone.set_transition_type(gtk4::StackTransitionType::SlideLeft);
                stack_clone.set_visible_child_name("next-page");
            }
        })
        .build();

    window.add_action_entries([
        action_close,
        action_open,
        action_stack_switch,
        action_next_page,
    ]);

    window.set_child(Some(&stack));
    return (window, stack, current_stack_page);
}
