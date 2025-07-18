use api::call::ApiCall;
use gio::prelude::*;
use gtk4::prelude::{GtkApplicationExt, WidgetExt};
use gtk4::{glib, Application};
use loader::pipe_loader::PipedData;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;
use std::{env, process};
use utils::config::SherlockFlags;

mod actions;
mod api;
mod application;
mod daemon;
mod g_subclasses;
mod launcher;
mod loader;
pub mod prelude;
mod ui;
mod utils;

use api::api::SherlockModes;
use api::server::SherlockServer;
use application::lock::{self, LockFile};
use loader::Loader;
use utils::{
    config::SherlockConfig,
    errors::{SherlockError, SherlockErrorType},
};

const SOCKET_PATH: &str = "/tmp/sherlock_daemon.socket";
const SOCKET_DIR: &str = "/tmp/";
const LOCK_FILE: &str = "/tmp/sherlock.lock";

static CONFIG: OnceLock<SherlockConfig> = OnceLock::new();

#[tokio::main]
async fn main() {
    let t0 = Instant::now();
    let (application, startup_errors, non_breaking, sherlock_flags, app_config, lock) =
        startup_loading().await;
    let t01 = Instant::now();
    application.connect_activate(move |app| {
        let sherlock = Rc::new(RefCell::new(api::api::SherlockAPI::new(app)));
        let t1 = Instant::now();
        if let Ok(timing_enabled) = std::env::var("TIMING") {
            if timing_enabled == "true" {
                println!("GTK Activation took {:?}", t01.elapsed());
            }
        }
        let errors = startup_errors.clone();
        let mut warnings = non_breaking.clone();


        // Main logic for the Search-View
        let (window, stack, current_stack_page, open_win) = ui::window::window(app);
        {
            let mut sherlock = sherlock.borrow_mut();
            sherlock.window = Some(window.downgrade());
            sherlock.open_window = Some(open_win.clone());
            sherlock.stack = Some(stack.downgrade());
        }
        window.connect_show({
            let t0 = t0.clone();
            move |_| {
            if let Ok(timing_enabled) = std::env::var("TIMING") {
                if timing_enabled == "true" {
                    println!("Window shown after {:?}", t0.elapsed());
                }
            }
        }});

        // Add closing logic
        app.set_accels_for_action("win.close", &["<Ctrl>W"]);

        // Significantly better id done here
        sherlock.borrow_mut().request(ApiCall::Show);

        // Print messages if icon parsers aren't installed
        let types: HashSet<String> = gdk_pixbuf::Pixbuf::formats().into_iter().filter_map(|f| f.name()).map(|s|s.to_string()).collect();
        if !types.contains("svg") {
            warnings.push(sherlock_error!(
                SherlockErrorType::MissingIconParser(String::from("svg")),
                format!("Icon parser for svg not found.\n\
                This could hinder Sherlock from rendering .svg icons.\n\
                Please ensure that \"librsvg\" is installed correctly.")
            ));
        }
        if !types.contains("png") {
            warnings.push(sherlock_error!(
                SherlockErrorType::MissingIconParser(String::from("png")),
                format!("Icon parser for png not found.\n\
                This could hinder Sherlock from rendering .png icons.\n\
                Please ensure that \"gdk-pixbuf2\" is installed correctly.")
            ));
        }

        glib::MainContext::default().spawn_local({
            let sherlock = Rc::clone(&sherlock);
            async move {

                // Either show user-specified content or show normal search
                let (error_stack, error_model) = ui::error_view::errors(&errors, &warnings, &current_stack_page, Rc::clone(&sherlock));
                let (search_frame, _handler) = match ui::search::search(&window, &current_stack_page, error_model.clone(), Rc::clone(&sherlock)) {
                    Ok(r) => r,
                    Err(e) => {
                        error_model.upgrade().map(|stack| stack.append(&e.tile("ERROR")));
                        return
                    }
                };
                stack.add_named(&search_frame, Some("search-page"));
                stack.add_named(&error_stack, Some("error-page"));


                // Notify the user about the value not having any effect to avoid confusion
                if let Some(c) = CONFIG.get() {
                    let opacity = c.appearance.opacity;
                    if !(0.1..=1.0).contains(&opacity) {
                        warnings.push(sherlock_error!(
                            SherlockErrorType::ConfigError(Some(format!(
                                "The opacity value of {} exceeds the allowed range (0.1 - 1.0) and will be automatically set to {}.",
                                opacity,
                                opacity.clamp(0.1, 1.0)
                            ))),
                            ""
                        ));
                    }
                }

                // Mode switching
                // Logic for the Error-View
                let error_view_active = if !app_config.debug.try_suppress_errors {
                    let show_errors = !errors.is_empty();
                    let show_warnings = !app_config.debug.try_suppress_warnings && !warnings.is_empty();
                    show_errors || show_warnings
                } else {
                    false
                };
                {
                    let mut sherlock = sherlock.borrow_mut();
                    let pipe = Loader::load_pipe_args();
                    let mut mode: Option<SherlockModes> = None;
                    if !pipe.is_empty() {
                        if sherlock_flags.display_raw {
                            let pipe = String::from_utf8_lossy(&pipe).to_string();
                            mode = Some(SherlockModes::DisplayRaw(pipe));
                        } else if let Some(mut data) = PipedData::new(&pipe){
                            if let Some(settings) = data.settings.take(){
                                settings.into_iter().for_each(|request| {
                                    sherlock.await_request(request);
                                });
                            }
                            mode = data.elements.take().map(|elements| SherlockModes::Pipe(elements));
                        }
                    };
                    if let Some(mode) = mode {
                        let request = ApiCall::SwitchMode(mode);
                        sherlock.await_request(request);
                    } else {
                        let mode = SherlockModes::Search;
                        let request = ApiCall::SwitchMode(mode);
                        sherlock.await_request(request);
                    }
                    if error_view_active {
                        let mode = SherlockModes::Error;
                        let request = ApiCall::SwitchMode(mode);
                        sherlock.await_request(request);
                    }
                    sherlock.flush();
                }

                // Load Icon theme
                if let Some(warning) = Loader::load_icon_theme() {
                    let _result = warning.insert(false);
                }

                // Load CSS Stylesheet
                if let Err(error) = Loader::load_css(true) {
                    let _result = error.insert(false);
                }
            }
        });

        // Logic for handling the daemonization
        if app_config.behavior.daemonize {
            // Used to cache render
            if let Some(window) = open_win.upgrade() {
                let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.open", None);
                let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.close", None);
            }
        }

        // Spawn api listener
        let _server = SherlockServer::listen(sherlock);

        // Print Timing
        if let Ok(timing_enabled) = std::env::var("TIMING") {
            if timing_enabled == "true" {
                println!("Window creation took {:?}", t1.elapsed());
                println!("Start to Finish took: {:?}", t0.elapsed());
            }
        }
    });
    application.run();
    drop(lock);
}

async fn startup_loading() -> (
    Application,
    Vec<SherlockError>,
    Vec<SherlockError>,
    SherlockFlags,
    SherlockConfig,
    LockFile,
) {
    let t0 = Instant::now();
    let mut non_breaking: Vec<SherlockError> = Vec::new();
    let mut startup_errors: Vec<SherlockError> = Vec::new();

    // Check for '.lock'-file to only start a single instance
    let lock = lock::ensure_single_instance(LOCK_FILE).unwrap_or_else(|_| {
        process::exit(1);
    });

    let app_future = async {
        // Initialize application
        let application = Application::builder()
            // .application_id("dev.skxxtz.sherlock")
            .flags(gio::ApplicationFlags::NON_UNIQUE | gio::ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();
        // Needed in order start Sherlock without glib flag handling
        application.connect_command_line(|app, _| {
            app.activate();
            0
        });
        application
    };

    // Setup flags
    let sherlock_flags = Loader::load_flags()
        .map_err(|e| startup_errors.push(e))
        .unwrap_or_default();

    // Parse configs from 'config.toml'
    let app_config = SherlockConfig::from_flags(&sherlock_flags).map_or_else(
        |e| {
            startup_errors.push(e);
            let defaults = utils::config::SherlockConfig::default();
            SherlockConfig::apply_flags(&sherlock_flags, defaults)
        },
        |(app_config, n)| {
            non_breaking.extend(n);
            app_config
        },
    );

    CONFIG
        .set(app_config.clone())
        .map_err(|_| {
            startup_errors.push(sherlock_error!(SherlockErrorType::ConfigError(None), ""));
        })
        .ok();

    // Load resources
    Loader::load_resources()
        .map_err(|e| startup_errors.push(e))
        .ok();

    if let Some(config) = CONFIG.get() {
        env::set_var("GSK_RENDERER", &config.appearance.gsk_renderer);
    }

    let application = app_future.await;

    if let Ok(timing_enabled) = std::env::var("TIMING") {
        if timing_enabled == "true" {
            println!("Initial Setup took {:?}", t0.elapsed());
        }
    }

    (
        application,
        startup_errors,
        non_breaking,
        sherlock_flags,
        app_config,
        lock,
    )
}
