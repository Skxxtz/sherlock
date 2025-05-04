// CRATES
use gio::glib::MainContext;
use gio::prelude::*;
use gtk4::prelude::{GtkApplicationExt, WidgetExt};
use gtk4::Application;
use std::sync::OnceLock;
use std::time::Instant;
use std::{env, process, thread};
use utils::config::SherlockFlags;

// MODS
mod actions;
mod application;
mod daemon;
mod g_subclasses;
mod launcher;
mod loader;
mod ui;
mod utils;

// IMPORTS
use application::lock::{self, LockFile};
use daemon::daemon::SherlockDaemon;
use loader::pipe_loader::deserialize_pipe;
use loader::Loader;
use utils::{
    config::SherlockConfig,
    errors::{SherlockError, SherlockErrorType},
};

const SOCKET_PATH: &str = "/tmp/sherlock_daemon.socket";
const LOCK_FILE: &str = "/tmp/sherlock.lock";

static CONFIG: OnceLock<SherlockConfig> = OnceLock::new();

#[tokio::main]
async fn main() {
    let (application, startup_errors, non_breaking, sherlock_flags, app_config, lock) =
        startup_loading();
    let t0 = Instant::now();
    application.connect_activate(move |app| {
        let t1 = Instant::now();
        if let Ok(timing_enabled) = std::env::var("TIMING") {
            if timing_enabled == "true" {
                println!("Activation took {:?}", t0.elapsed());
            }
        }
        let mut error_list = startup_errors.clone();
        let mut non_breaking = non_breaking.clone();

        // Load custom icons from icon path specified in 'config.toml'
        let n = Loader::load_icon_theme();
        non_breaking.extend(n);

        // Load CSS Stylesheet
        let n = Loader::load_css()
            .map_err(|e| error_list.push(e))
            .unwrap_or_default();
        non_breaking.extend(n);

        // Main logic for the Search-View
        let (window, stack, current_stack_page) = ui::window::window(app);

        // Add closing logic
        app.set_accels_for_action("win.close", &["<Ctrl>W", "Escape"]);

        // Significantly better id done here
        let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.open", None);

        // Either show user-specified content or show normal search
        let (error_stack, error_model) = ui::error_view::errors(&error_list, &non_breaking, &current_stack_page);
        let pipe = Loader::load_pipe_args();
        let (search_stack, handler) = if pipe.is_empty() {
            ui::search::search(&window, &current_stack_page, error_model)
        } else {
            if sherlock_flags.display_raw {
                let pipe = String::from_utf8_lossy(&pipe);
                ui::user::display_raw(pipe, sherlock_flags.center_raw, error_model)
            } else {
                let parsed = deserialize_pipe(pipe);
                if let Some(c) = CONFIG.get() {
                    let method: &str = c.pipe.method.as_deref().unwrap_or("print");
                    ui::user::display_pipe(&window, parsed, method, error_model)
                } else {
                    return;
                }
            }
        };
        stack.add_named(&search_stack, Some("search-page"));

        // Notify the user about the value not having any effect to avoid confusion
        if let Some(c) = CONFIG.get() {
            let opacity = c.appearance.opacity;

            if !(0.1..=1.0).contains(&opacity) {
                non_breaking.push(SherlockError {
                    error: SherlockErrorType::ConfigError(Some(format!(
                        "The opacity value of {} exceeds the allowed range (0.1 - 1.0) and will be automatically set to {}.",
                        opacity,
                        opacity.clamp(0.1, 1.0)
                    ))),
                    traceback: String::new(),
                });
            }
        }

        // Logic for the Error-View
        stack.add_named(&error_stack, Some("error-page"));
        if !app_config.debug.try_suppress_errors {
            let show_errors = !error_list.is_empty();
            let show_warnings = !app_config.debug.try_suppress_warnings && !non_breaking.is_empty();
            if show_errors || show_warnings {
                let _ = gtk4::prelude::WidgetExt::activate_action(
                    &window,
                    "win.switch-page",
                    Some(&String::from("->error-page").to_variant()),
                );
            } else {
                let _ = gtk4::prelude::WidgetExt::activate_action(
                    &window,
                    "win.switch-page",
                    Some(&String::from("->search-page").to_variant()),
                );
            }
        }

        // Logic for handling the daemonization
        if let Some(c) = CONFIG.get() {
            if c.behavior.daemonize {
                // Used to cache render
                let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.open", None);
                let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.close", None);

                window.connect_show({
                    move |_window|{
                    handler.populate();
                }});
                // Create async pipeline
                let (sender, receiver) = async_channel::bounded(1);
                thread::spawn(move || {
                    async_std::task::block_on(async {
                        let _daemon = SherlockDaemon::new(sender).await;
                    });
                });

                // Handle receiving using pipline
                MainContext::default().spawn_local(async move {
                    while let Ok(_msg) = receiver.recv().await {
                        let _ = gtk4::prelude::WidgetExt::activate_action(
                            &window, "win.open", None,
                        );
                    }
                });
            }
        }
        if let Ok(timing_enabled) = std::env::var("TIMING") {
            if timing_enabled == "true" {
                println!("Window creation took {:?}", t1.elapsed());
            }
        }
    });
    application.run();
    drop(lock);
}

#[sherlock_macro::timing("\nContent loading")]
fn startup_loading() -> (
    Application,
    Vec<SherlockError>,
    Vec<SherlockError>,
    SherlockFlags,
    SherlockConfig,
    LockFile,
) {
    let mut non_breaking: Vec<SherlockError> = Vec::new();
    let mut startup_errors: Vec<SherlockError> = Vec::new();

    // Check for '.lock'-file to only start a single instance
    let lock = lock::ensure_single_instance(LOCK_FILE).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

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
            startup_errors.push(SherlockError {
                error: SherlockErrorType::ConfigError(None),
                traceback: String::new(),
            });
        })
        .ok();

    // Load resources
    Loader::load_resources()
        .map_err(|e| startup_errors.push(e))
        .ok();

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

    (
        application,
        startup_errors,
        non_breaking,
        sherlock_flags,
        app_config,
        lock,
    )
}
