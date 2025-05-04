use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::{
    env,
    fs::{self, File},
};

use gio::ActionEntry;
use gtk4::{prelude::*, Application, ApplicationWindow, StackTransitionType};
use gtk4::{Builder, Stack};
use gtk4_layer_shell::{Layer, LayerShell};
use serde::Deserialize;

use crate::actions::execute_from_attrs;
use crate::loader::util::JsonCache;
use crate::utils::errors::{SherlockError, SherlockErrorType};
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
    window.set_namespace("sherlock");
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);

    //Build main fame here that holds logic for stacking
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/window.ui");
    let stack: Stack = builder.object("stack").unwrap();

    // Setup action to close the window
    let action_close = ActionEntry::builder("close")
        .activate(move |window: &ApplicationWindow, _, _| {
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
            // Increment Sherlock Execution counter
            let start_count = SherlockCounter::new()
                .and_then(|counter| counter.increment())
                .unwrap_or(0);

            if let Some(c) = CONFIG.get() {
                // parse sherlock actions
                let actions: Vec<SherlockAction> =
                    JsonCache::read(&c.files.actions).unwrap_or_default();
                // activate sherlock actions
                actions
                    .into_iter()
                    .filter(|action| start_count % action.on == 0)
                    .for_each(|action| {
                        let attrs: HashMap<String, String> =
                            HashMap::from([(String::from("method"), action.action)]);
                        execute_from_attrs(window, &attrs);
                    });
                match c.behavior.daemonize {
                    true => {
                        window.present();
                    }
                    false => window.present(),
                }
            };
        })
        .build();

    // Setup action to switch to a specific stack page
    let stack_clone = stack.downgrade().clone();
    let page_clone = Rc::clone(&current_stack_page);
    let action_stack_switch = ActionEntry::builder("switch-page")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(move |_: &ApplicationWindow, _, parameter| {
            let parameter = parameter
                .and_then(|p| p.get::<String>())
                .unwrap_or_default();

            fn parse_transition(from: &str, to: &str) -> StackTransitionType {
                match (from, to) {
                    ("search-page", "error-page") => StackTransitionType::OverRightLeft,
                    ("error-page", "search-page") => StackTransitionType::OverRightLeft,
                    _ => StackTransitionType::None,
                }
            }
            if let Some((from, to)) = parameter.split_once("->") {
                stack_clone.upgrade().map(|stack| {
                    stack.set_transition_type(parse_transition(&from, &to));
                    stack.set_visible_child_name(&to);
                });
                *page_clone.borrow_mut() = to.to_string();
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
                builder
                    .content
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|content| {
                        content.set_wrap_mode(gtk4::WrapMode::Word);
                        let buf = content.buffer();
                        buf.set_text(parameter.as_ref());
                    });
                builder
                    .object
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|obj| {
                        stack_clone.add_named(&obj, Some("next-page"));
                    });
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

#[derive(Debug, Deserialize)]
pub struct SherlockAction {
    pub on: u32,
    pub action: String,
}
pub struct SherlockCounter {
    path: PathBuf,
}
impl SherlockCounter {
    fn new() -> Result<Self, SherlockError> {
        let home = env::var("HOME").map_err(|e| SherlockError {
            error: SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
            traceback: e.to_string(),
        })?;
        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".sherlock/sherlock_count");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| SherlockError {
                error: SherlockErrorType::DirCreateError(".sherlock".to_string()),
                traceback: e.to_string(),
            })?;
        }
        Ok(Self { path })
    }
    fn increment(&self) -> Result<u32, SherlockError> {
        let content = self.read()?.saturating_add(1);
        self.write(content)?;
        Ok(content)
    }
    fn read(&self) -> Result<u32, SherlockError> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(0);
            }
            Err(e) => {
                return Err(SherlockError {
                    error: SherlockErrorType::FileReadError(self.path.clone()),
                    traceback: e.to_string(),
                });
            }
        };
        let mut buf = [0u8; 4];

        file.read_exact(&mut buf).map_err(|e| SherlockError {
            error: SherlockErrorType::FileReadError(self.path.clone()),
            traceback: e.to_string(),
        })?;
        Ok(u32::from_le_bytes(buf))
    }
    fn write(&self, count: u32) -> Result<(), SherlockError> {
        let file = File::create(self.path.clone()).map_err(|e| SherlockError {
            error: SherlockErrorType::FileWriteError(self.path.clone()),
            traceback: e.to_string(),
        })?;

        let mut writer = BufWriter::new(file);
        writer
            .write_all(&count.to_le_bytes())
            .map_err(|e| SherlockError {
                error: SherlockErrorType::FileWriteError(self.path.clone()),
                traceback: e.to_string(),
            })?;

        Ok(())
    }
}
