use gio::glib::WeakRef;
use gio::ActionEntry;
use gtk4::gdk::{Display, Key, Monitor};
use gtk4::{
    prelude::*, Application, ApplicationWindow, EventControllerFocus, EventControllerKey,
    StackTransitionType,
};
use gtk4::{Builder, Stack};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

use crate::daemon::daemon::close_response;
use crate::launcher::emoji_picker::emojies;
use crate::utils::config::SherlockConfig;
use crate::CONFIG;

use super::tiles::util::TextViewTileBuilder;

#[sherlock_macro::timing("Window frame creation")]
pub fn window(
    application: &Application,
) -> (
    ApplicationWindow,
    Stack,
    Rc<RefCell<String>>,
    WeakRef<ApplicationWindow>,
) {
    // 617 with, 593 without notification bar
    let config = match CONFIG.get() {
        Some(c) => c,
        _ => &SherlockConfig::default(),
    };
    let (width, height, opacity) = (
        config.appearance.width,
        config.appearance.height,
        config.appearance.opacity,
    );

    let current_stack_page = Rc::new(RefCell::new(String::from("search-page")));

    let window: ApplicationWindow = ApplicationWindow::builder()
        .application(application)
        .default_width(width)
        .resizable(false)
        .decorated(false)
        .opacity(opacity.clamp(0.1, 1.0))
        .build();

    window.init_layer_shell();
    window.set_namespace("sherlock");
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);

    if !config.expand.enable {
        window.set_default_height(height);
    } else {
        window.set_anchor(gtk4_layer_shell::Edge::Top, true);
        window.set_margin(gtk4_layer_shell::Edge::Top, config.expand.margin);
    }

    if !config.runtime.photo_mode {
        let focus_controller = EventControllerFocus::new();
        focus_controller.connect_leave({
            let window_ref = window.downgrade();
            move |_| {
                if let Some(window) = window_ref.upgrade() {
                    let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.close", None);
                }
            }
        });
        window.add_controller(focus_controller);
    }

    // Handle the key press event
    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Bubble);
    key_controller.connect_key_pressed({
        let window_clone = window.downgrade();
        move |_, keyval, _, _| {
            if keyval == Key::Escape {
                window_clone
                    .upgrade()
                    .map(|win| gtk4::prelude::WidgetExt::activate_action(&win, "win.close", None));
            }
            false.into()
        }
    });
    window.add_controller(key_controller);

    // Make backdrop if config key is set
    let backdrop = if let Some(c) = CONFIG.get() {
        if c.backdrop.enable {
            let edge = match c.backdrop.edge.to_lowercase().as_str() {
                "top" => Edge::Top,
                "bottom" => Edge::Bottom,
                "left" => Edge::Left,
                "right" => Edge::Right,
                _ => Edge::Top,
            };
            make_backdrop(application, &window, c.backdrop.opacity, edge)
        } else {
            None
        }
    } else {
        None
    };

    //Build main fame here that holds logic for stacking
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/window.ui");
    let stack: Stack = builder.object("stack").unwrap();
    let stack_ref = stack.downgrade();

    // Setup action to close the window
    let action_close = ActionEntry::builder("close")
        .activate(move |window: &ApplicationWindow, _, _| {
            if !window.is_visible() {
                return;
            }

            // Send close message to possible instance
            let _result = close_response();

            if let Some(c) = CONFIG.get() {
                match c.behavior.daemonize {
                    true => {
                        window.set_visible(false);
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
    let action_open = ActionEntry::builder("open")
        .activate(move |window: &ApplicationWindow, _, _| {
            window.present();
        })
        .build();

    // Setup action to switch to a specific stack page
    let stack_clone = stack_ref.clone();
    let page_clone = Rc::clone(&current_stack_page);
    let action_stack_switch = ActionEntry::builder("switch-page")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(move |_: &ApplicationWindow, _, parameter| {
            let parameter = parameter
                .and_then(|p| p.get::<String>())
                .unwrap_or_default();

            fn parse_transition(from: &str, to: &str) -> StackTransitionType {
                match (from, to) {
                    ("search-page", "error-page") => StackTransitionType::SlideRight,
                    ("error-page", "search-page") => StackTransitionType::SlideLeft,
                    ("search-page", "emoji-page") => StackTransitionType::SlideLeft,
                    ("emoji-page", "search-page") => StackTransitionType::SlideRight,
                    ("search-page", "display-raw") => StackTransitionType::SlideRight,
                    _ => StackTransitionType::None,
                }
            }
            if let Some((from, to)) = parameter.split_once("->") {
                stack_clone.upgrade().map(|stack| {
                    stack.set_transition_type(parse_transition(&from, &to));
                    if let Some(child) = stack.child_by_name(&to) {
                        stack.set_visible_child(&child);
                        *page_clone.borrow_mut() = to.to_string();
                    }
                });
            }
        })
        .build();

    // Setup action to add a stackpage
    let stack_clone = stack_ref.clone();
    let action_next_page = ActionEntry::builder("add-page")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(move |_: &ApplicationWindow, _, parameter| {
            if let Some(parameter) = parameter.and_then(|p| p.get::<String>()) {
                let builder = TextViewTileBuilder::new("/dev/skxxtz/sherlock/ui/text_view_tile.ui");
                builder
                    .content
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|content| {
                        content.set_wrap_mode(gtk4::WrapMode::Word);
                        let buf = content.buffer();
                        buf.set_text(parameter.as_ref());
                    });
                if let Some(stack_clone) = stack_clone.upgrade() {
                    builder.object.as_ref().map(|obj| {
                        stack_clone.add_named(obj, Some("next-page"));
                    });
                    stack_clone.set_transition_type(gtk4::StackTransitionType::SlideLeft);
                    stack_clone.set_visible_child_name("next-page");
                }
            }
        })
        .build();

    let stack_clone = stack_ref.clone();
    let action_remove_page = ActionEntry::builder("rm-page")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(move |_: &ApplicationWindow, _, parameter| {
            if let Some(parameter) = parameter.and_then(|p| p.get::<String>()) {
                if let Some(stack_clone) = stack_clone.upgrade() {
                    if let Some(child) = stack_clone.child_by_name(&parameter) {
                        stack_clone.remove(&child);
                    }
                }
            }
        })
        .build();

    let emoji_action = ActionEntry::builder("emoji-page")
        .activate({
            let stack_clone = stack_ref.clone();
            let current_stack_page = current_stack_page.clone();
            move |_: &ApplicationWindow, _, _| {
                // Either show user-specified content or show normal search
                let (emoji_stack, _emoji_model) = match emojies(&current_stack_page) {
                    Ok(r) => r,
                    Err(e) => {
                        println!("{:?}", e);
                        return;
                    }
                };
                if let Some(stack) = stack_clone.upgrade() {
                    stack.add_named(&emoji_stack, Some("emoji-page"));
                }
            }
        })
        .build();

    window.set_child(Some(&stack));
    let win_ref = match backdrop {
        Some(backdrop) => {
            backdrop.add_action_entries([action_open]);
            window.add_action_entries([
                action_close,
                action_stack_switch,
                action_next_page,
                emoji_action,
                action_remove_page,
            ]);
            backdrop.downgrade()
        }
        _ => {
            window.add_action_entries([
                action_close,
                action_open,
                action_stack_switch,
                action_next_page,
                emoji_action,
                action_remove_page,
            ]);
            window.downgrade()
        }
    };
    return (window, stack, current_stack_page, win_ref);
}

fn make_backdrop(
    application: &Application,
    main_window: &ApplicationWindow,
    opacity: f64,
    edge: Edge,
) -> Option<ApplicationWindow> {
    let monitor = Display::default()
        .map(|d| d.monitors())
        .and_then(|m| m.item(0).and_downcast::<Monitor>())?;
    let rect = monitor.geometry();
    let backdrop = ApplicationWindow::builder()
        .application(application)
        .decorated(false)
        .title("Backdrop")
        .opacity(opacity)
        .default_width(rect.width()) // Adjust to your screen resolution or use monitor API
        .default_height(rect.height())
        .resizable(false)
        .build();
    // Initialize layershell
    backdrop.set_widget_name("backdrop");
    backdrop.init_layer_shell();
    backdrop.set_namespace("sherlock-backdrop");
    backdrop.set_exclusive_zone(0);
    backdrop.set_layer(gtk4_layer_shell::Layer::Overlay);
    backdrop.set_anchor(edge, true);

    let window_clone = main_window.downgrade();
    let backdrop_clone = backdrop.downgrade();

    backdrop.connect_show({
        let window = window_clone.clone();
        move |_| {
            window.upgrade().map(|win| win.set_visible(true));
        }
    });
    main_window.connect_destroy({
        let backdrop = backdrop_clone.clone();
        move |_| {
            backdrop.upgrade().map(|win| win.close());
        }
    });
    main_window.connect_hide({
        let backdrop = backdrop_clone.clone();
        move |_| {
            backdrop.upgrade().map(|win| win.set_visible(false));
        }
    });

    Some(backdrop)
}
