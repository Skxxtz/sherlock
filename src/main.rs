use api::call::ApiCall;
use futures::join;
use gio::glib::idle_add_local;
use gio::prelude::*;
use gtk4::prelude::{GtkApplicationExt, WidgetExt};
use gtk4::{glib, Application};
use loader::pipe_loader::PipedData;
use once_cell::sync::OnceCell;
use simd_json::prelude::ArrayTrait;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::RwLock;
use std::time::Instant;
use std::{env, process};

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
use application::lock::LockFile;
use loader::Loader;
use utils::{
    config::SherlockConfig,
    errors::{SherlockError, SherlockErrorType},
};

use crate::loader::icon_loader::{CustomIconTheme, IconThemeGuard};
use crate::utils::config::ConfigGuard;

const SOCKET_PATH: &str = "/tmp/sherlock_daemon.sock";
const SOCKET_DIR: &str = "/tmp/";
const LOCK_FILE: &str = "/tmp/sherlock.lock";

static CONFIG: OnceCell<RwLock<SherlockConfig>> = OnceCell::new();
static ICONS: OnceCell<RwLock<CustomIconTheme>> = OnceCell::new();

#[tokio::main]
async fn main() {
    let t0 = Instant::now();
    // Save original GSK_RENDERER to ORIGINAL_GSK_RENDERER as a temporary variable
    let original_gsk_renderer = env::var("GSK_RENDERER").unwrap_or_default();
    env::set_var("ORIGINAL_GSK_RENDERER", original_gsk_renderer);

    let setup = setup().await;
    let t01 = Instant::now();
    setup.app.connect_activate(move |app| {
        let sherlock = Rc::new(RefCell::new(api::api::SherlockAPI::new(app)));
        let t1 = Instant::now();
        if let Ok(timing_enabled) = std::env::var("TIMING") {
            if timing_enabled == "true" {
                println!("GTK Activation took {:?}", t01.elapsed());
            }
        }
        let mut errors = setup.errors.clone();
        let warnings = setup.warnings.clone();

        if setup.config.behavior.use_xdg_data_dir_icons {
            glib::MainContext::default().spawn_local({
                let sherlock = sherlock.clone();
                async move {
                    if let Some(e) = Loader::load_icon_theme().await {
                        let _ = sherlock
                            .borrow_mut()
                            .await_request(ApiCall::SherlockWarning(e));
                    }
                }
            });
        }

        if let Err(error) = Loader::load_css(true, None) {
            errors.push(error);
        }

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
            }
        });

        // Add closing logic
        app.set_accels_for_action("win.close", &["<Ctrl>W"]);

        // Significantly better id done here
        if let Some(obf) = setup.config.runtime.input {
            sherlock
                .borrow_mut()
                .request(ApiCall::SwitchMode(SherlockModes::Input(obf)));
        } else {
            sherlock
                .borrow_mut()
                .request(ApiCall::Show("all".to_string()));
        }

        // Initialize error backend
        let error_backend = ui::error_view::ErrorBackend::new();
        sherlock
            .borrow_mut()
            .errors
            .replace(error_backend.model.downgrade());

        // Show search frame
        match ui::search::search(&window, &current_stack_page, Rc::clone(&sherlock)) {
            Ok(search_frame) => {
                stack.add_named(&search_frame, Some("search-page"));
            }
            Err(e) => {
                errors.push(e);
            }
        };

        // Lazy load error view
        idle_add_local({
            let backend = error_backend;
            let stack = stack.downgrade();
            let stack_page = current_stack_page.clone();
            move || {
                if let Some(stack) = stack.upgrade() {
                    let error_stack = ui::error_view::errors(&backend, &stack_page);
                    stack.add_named(&error_stack, Some("error-page"));
                }
                false.into()
            }
        });

        // Mode switching
        // Logic for the Error-View
        let error_view_active = if !setup.config.debug.try_suppress_errors {
            let show_errors = !errors.is_empty();
            let show_warnings = !setup.config.debug.try_suppress_warnings && !warnings.is_empty();
            show_errors || show_warnings
        } else {
            false
        };
        {
            let mut sherlock = sherlock.borrow_mut();
            let pipe = Loader::load_pipe_args();
            let mut mode: Option<SherlockModes> = None;
            if !pipe.is_empty() {
                if setup.config.runtime.display_raw {
                    let pipe = String::from_utf8_lossy(&pipe).to_string();
                    mode = Some(SherlockModes::DisplayRaw(pipe));
                } else if let Some(mut data) = PipedData::new(&pipe) {
                    if let Some(settings) = data.settings.take() {
                        settings.into_iter().for_each(|request| {
                            sherlock.await_request(request);
                        });
                    }
                    mode = data
                        .elements
                        .take()
                        .map(|elements| SherlockModes::Pipe(elements));
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
                // Insert errors and show error page
                errors.into_iter().for_each(|err| {
                    let request = ApiCall::SherlockError(err);
                    sherlock.await_request(request);
                });
                warnings.into_iter().for_each(|warn| {
                    let request = ApiCall::SherlockWarning(warn);
                    sherlock.await_request(request);
                });
                let mode = SherlockModes::Error;
                let request = ApiCall::SwitchMode(mode);
                sherlock.await_request(request);
            }
            sherlock.flush();
        }

        // Spawn api listener
        let _server = SherlockServer::listen(sherlock);

        // Logic for handling the daemonization
        if setup.config.runtime.daemonize {
            // Used to cache render
            if let Some(window) = open_win.upgrade() {
                let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.close", None);
            }
        }

        // Print Timing
        if let Ok(timing_enabled) = std::env::var("TIMING") {
            if timing_enabled == "true" {
                println!("Window creation took {:?}", t1.elapsed());
                println!("Start to Finish took: {:?}", t0.elapsed());
            }
        }

        idle_add_local({
            move || {
                // Do cleanup after window is shown
                post_startup();
                false.into()
            }
        });
    });
    setup.app.run();
    drop(setup.lock);
}

async fn setup() -> StartupResponse {
    let t0 = Instant::now();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let _ = sher_log!("New instance started");

    let lock = LockFile::single_instance(LOCK_FILE).unwrap_or_else(|_| process::exit(1));

    let (flags_result, app) = join!(async { Loader::load_flags() }, async {
        let app = Application::builder()
            .flags(gio::ApplicationFlags::NON_UNIQUE | gio::ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();
        app.connect_command_line(|app, _| {
            app.activate();
            0
        });
        app
    },);

    if let Err(e) = Loader::load_resources() {
        errors.push(e);
    }

    let mut flags = flags_result.map_err(|e| errors.push(e)).unwrap_or_default();

    let config = flags.to_config().map_or_else(
        |e| {
            errors.push(e);
            let defaults = SherlockConfig::default();
            SherlockConfig::apply_flags(&mut flags, defaults)
        },
        |(cfg, non_crit)| {
            warnings.extend(non_crit);
            cfg
        },
    );

    // Setup custom icons
    let _ = ICONS.set(RwLock::new(CustomIconTheme::new()));
    config.appearance.icon_paths.iter().for_each(|path| {
        if let Err(e) = IconThemeGuard::add_path(path) {
            errors.push(e);
        }
    });

    // Create global config
    let _ = CONFIG.set(RwLock::new(config.clone())).map_err(|_| {
        errors.push(sherlock_error!(SherlockErrorType::ConfigError(None), ""));
    });

    // Set GSK_RENDERER
    if let Ok(config) = ConfigGuard::read() {
        env::set_var("GSK_RENDERER", &config.appearance.gsk_renderer);
    }

    if let Ok(timing_enabled) = std::env::var("TIMING") {
        if timing_enabled == "true" {
            println!("Initial Setup took {:?}", t0.elapsed());
        }
    }

    StartupResponse {
        app,
        errors,
        warnings,
        config,
        lock,
    }
}

struct StartupResponse {
    app: Application,
    errors: Vec<SherlockError>,
    warnings: Vec<SherlockError>,
    config: SherlockConfig,
    lock: LockFile,
}

fn post_startup() {
    // Restore original GSK_RENDERER from temporary variable
    let original_gsk_renderer = env::var("ORIGINAL_GSK_RENDERER").unwrap_or_default();
    env::set_var("GSK_RENDERER", original_gsk_renderer);
    // Remove temporary variable
    env::remove_var("ORIGINAL_GSK_RENDERER");

    // Print messages if icon parsers aren't installed
    let available: HashSet<String> = gdk_pixbuf::Pixbuf::formats()
        .into_iter()
        .filter_map(|f| f.name())
        .map(|s| s.to_string())
        .collect();

    let required = vec![("svg", "librsvg"), ("png", "gdk-pixbuf2")];
    required
        .into_iter()
        .filter(|(t, _)| !available.contains(*t))
        .for_each(|(t, d)| {
            let _ = sherlock_error!(
                SherlockErrorType::MissingIconParser(t.to_string()),
                format!(
                    "Icon parser for {} not found.\n\
                    This could hinder Sherlock from rendering .{} icons.\n\
                    Please ensure that \"{}\" is installed correctly.",
                    t, t, d
                )
            )
            .insert(false);
        });

    // Notify the user about the value not having any effect to avoid confusion
    if let Ok(c) = ConfigGuard::read() {
        let opacity = c.appearance.opacity;
        if !(0.1..=1.0).contains(&opacity) {
            let _ = sherlock_error!(
                SherlockErrorType::ConfigError(Some(format!(
                    "The opacity value of {} exceeds the allowed range (0.1 - 1.0) and will be automatically set to {}.",
                    opacity,
                    opacity.clamp(0.1, 1.0)
                ))),
                ""
            ).insert(false);
        }
    }
}
