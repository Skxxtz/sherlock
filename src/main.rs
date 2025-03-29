use gio::prelude::*;
use gtk4::{prelude::*, Application, ApplicationWindow};
use gtk4::{Entry, EventController, Stack, Widget};
use loader::util::SherlockErrorType;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
use std::{env, process, thread};

mod actions;
mod daemon;
mod launcher;
mod loader;
mod lock;
mod ui;

const SOCKET_PATH: &str = "/tmp/sherlock_daemon.socket";

use daemon::deamon::SherlockDeamon;
use loader::{
    util::{SherlockConfig, SherlockError},
    Loader,
};
use ui::util::show_stack_page;

struct AppState {
    window: Option<ApplicationWindow>,
    stack: Option<Stack>,
    search_bar: Option<Entry>,
}
impl AppState {
    pub fn add_stack_page<T, U>(&self, child: T, name: U)
    where
        T: IsA<Widget>,
        U: AsRef<str>,
    {
        if let Some(stack) = &self.stack {
            stack.add_named(&child, Some(name.as_ref()));
        }
    }

    pub fn add_event_listener<T: IsA<EventController>>(&self, controller: T) {
        if let Some(window) = &self.window {
            window.add_controller(controller);
        }
    }
    pub fn remove_event_listener<T: IsA<EventController>>(&self, controller: &T) {
        if let Some(window) = &self.window {
            window.remove_controller(controller);
        }
    }
}

thread_local! {
    static APP_STATE: RefCell<Option<Rc<AppState>>> = RefCell::new(None);
}
static CONFIG: OnceLock<SherlockConfig> = OnceLock::new();

#[tokio::main]
async fn main() {
    let mut startup_errors: Vec<SherlockError> = Vec::new();
    let mut non_breaking: Vec<SherlockError> = Vec::new();

    // Check for '.lock'-file to only start a single instance
    let lock_file = "/tmp/sherlock.lock";
    let _ = match lock::ensure_single_instance(lock_file) {
        Ok(lock) => lock,
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    };

    let sherlock_flags = Loader::load_flags()
        .map_err(|e| startup_errors.push(e))
        .unwrap_or_default();

    // Parse configs from 'config.toml'
    let (app_config, n) = Loader::load_config(&sherlock_flags)
        .map_err(|e| startup_errors.push(e))
        .unwrap_or(loader::util::SherlockConfig::default());
    non_breaking.extend(n);

    match CONFIG.set(app_config.clone()) {
        Ok(_) => {}
        Err(_) => {
            startup_errors.push(SherlockError {
                error: SherlockErrorType::ConfigError(None),
                traceback: format!(""),
            });
        }
    };

    let _ = Loader::load_resources().map_err(|e| startup_errors.push(e));

    // Initialize application
    let application = Application::new(
        Some("dev.skxxtz.sherlock"),
        gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );

    if let Some(config) = CONFIG.get() {
        env::set_var("GSK_RENDERER", &config.appearance.gsk_renderer);
    }

    // Needed in order start Sherlock without glib flag handling
    application.connect_command_line(|app, _| {
        app.activate();
        0
    });

    application.connect_activate(move |app| {
        let mut error_list = startup_errors.clone();
        let mut non_breaking = non_breaking.clone();

        // Initialize launchers from 'fallback.json'
        let (launchers, n) = Loader::load_launchers(&sherlock_flags)
            .map_err(|e| error_list.push(e))
            .unwrap_or_default();
        non_breaking.extend(n);

        // Load custom icons from icon path specified in 'config.toml'
        let n = Loader::load_icon_theme();
        non_breaking.extend(n);

        // Load CSS Stylesheet
        let n = Loader::load_css(&sherlock_flags)
            .map_err(|e| error_list.push(e))
            .unwrap_or_default();
        non_breaking.extend(n);

        // Main logic for the Search-View
        let (window, stack) = ui::window::window(&app);

        // creating app state
        let state = Rc::new(AppState {
            window: Some(window),
            stack: Some(stack),
            search_bar: None,
        });
        APP_STATE.with(|app_state| *app_state.borrow_mut() = Some(state));

        // Either show user-specified content or show normal search
        let pipe = Loader::load_pipe_args();
        if pipe.is_empty() {
            ui::search::search(launchers);
        } else {
            if sherlock_flags.display_raw {
                ui::user::display_raw(pipe, sherlock_flags.center_raw);
            } else {
                let lines: Vec<String> = pipe
                    .split("\n")
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
                ui::user::display_pipe(lines);
            }
        };

        // Logic for the Error-View
        if !app_config.debug.try_suppress_errors {
            let show_errors = !error_list.is_empty();
            let show_warnings = !app_config.debug.try_suppress_warnings && !non_breaking.is_empty();
            if show_errors || show_warnings {
                ui::error_view::errors(&error_list, &non_breaking);
                show_stack_page("error-page", None);
            }
        }

        // Logic for handling the daemonization
        if let Some(c) = CONFIG.get() {
            match c.behavior.daemonize {
                true => {
                    // deamonize option

                    // Cache the results
                    ui::window::show_window();
                    ui::window::hide_window(false);

                    thread::spawn(move || {
                        SherlockDeamon::new(SOCKET_PATH);
                    });
                }
                false => {
                    // Show window without daemonizing
                    ui::window::show_window();
                }
            }
        }
    });
    application.run();
}
