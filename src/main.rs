use futures::{StreamExt, stream::FuturesUnordered};
use once_cell::sync::OnceCell;
use simd_json::prelude::ArrayTrait;
use std::{
    collections::HashMap,
    io::Write,
    path::Path,
    sync::{Arc, OnceLock, RwLock},
};
use tokio::net::UnixListener;

use gpui::{
    layer_shell::{Layer, LayerShellOptions},
    *,
};

use crate::{
    launcher::children::{LauncherValues, RenderableChild},
    loader::{CustomIconTheme, IconThemeGuard, Loader, assets::Assets},
    ui::{
        UIFunction,
        error::{
            view::{DismissErrorEvent, ErrorView},
            window::spawn_error_window,
        },
        launcher::{LauncherMode, NextVar, OpenContext, PrevVar},
        search_bar::{EmptyBackspace, actions::ShortcutAction},
        workspace::{LauncherErrorEvent, SherlockWorkspace, WorkspaceView},
    },
    utils::{
        config::{ConfigGuard, ConfigWatcher, SherlockConfig},
        errors::SherlockErrorType,
    },
};

mod launcher;
mod loader;
mod prelude;
mod ui;
mod utils;

use ui::launcher::{Execute, FocusNext, FocusPrev, LauncherView, Quit};
use ui::search_bar::{
    TextInput,
    actions::{Backspace, Copy, Cut, Delete, DeleteAll, End, Home, Left, Paste, Right, SelectAll},
};

use utils::errors::SherlockError;

/// Holds the icon cache, containing all known icon names and their file locations.
static ICONS: OnceCell<RwLock<CustomIconTheme>> = OnceCell::new();
/// Holed the global config struct for user-specified config values.
static CONFIG: OnceCell<RwLock<SherlockConfig>> = OnceCell::new();
/// Holds the string used to show and hide the context menu.
static CONTEXT_MENU_BIND: OnceLock<String> = OnceLock::new();

pub struct SetupResult {
    pub config_dir: Box<Path>,
    pub errors: Vec<SherlockError>,
    pub warnings: Vec<SherlockError>,
}

/// Parses application flags and user config.
///
/// This function first parses and handles the application flags, then loads
fn setup() -> Result<SetupResult, SherlockError> {
    let mut flags = Loader::load_flags()?;
    let mut warnings: Vec<SherlockError> = Vec::new();
    let mut errors: Vec<SherlockError> = Vec::new();

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

    let _ = ICONS.set(RwLock::new(CustomIconTheme::new()));
    config.appearance.icon_paths.iter().for_each(|path| {
        if let Err(e) = IconThemeGuard::add_path(path) {
            warnings.push(e);
        }
    });

    CONFIG
        .set(RwLock::new(config.clone()))
        .map_err(|_| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;

    let config_dir: Box<Path> = config
        .files
        .config
        .parent()
        .ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::DirReadError("Config Root Dir".into()),
                "Failed to read config root dir."
            )
        })?
        .into();

    Ok(SetupResult {
        config_dir,
        errors,
        warnings,
    })
}

#[tokio::main]
async fn main() {
    // connect to existing socket
    let socket_path = "/tmp/sherlock.sock";
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
        let _ = stream.write_all(b"open");
        return;
    }

    let setup_result = setup();

    // start primary instance
    let app = Application::new()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::Explicit);

    app.run(|cx: &mut App| {
        match setup_result {
            Err(e) => {
                spawn_error_window(cx, e.to_string());
            }
            Ok(SetupResult {
                config_dir,
                errors,
                warnings,
            }) => {
                let mut watcher = ConfigWatcher::new(config_dir);

                register_bindings(cx);

                let socket_path = "/tmp/sherlock.sock";
                let data: Entity<Arc<Vec<RenderableChild>>> = cx.new(|_| Arc::new(Vec::new()));
                let modes = match Loader::load_launchers(cx, data.clone()) {
                    Ok(modes) => modes,
                    Err(e) => {
                        eprintln!("{e}");
                        return;
                    }
                };

                // listen for open requests
                let _ = std::fs::remove_file(socket_path);
                let listener = UnixListener::bind(socket_path).unwrap();

                let initial_errors = errors;
                let initial_warnings = warnings;

                cx.spawn(|cx: &mut AsyncApp| {
                    let cx = cx.clone();
                    async move {
                        let mut win: Option<WindowHandle<SherlockWorkspace>> = None;
                        let mut current_generation: u64 = 0;
                        let mut active_update_task: Option<gpui::Task<()>> = None;
                        loop {
                            if let Ok((_stream, _)) = listener.accept().await {
                                // check if config files changed
                                if let Ok(audit) = watcher.audit() {
                                    if !audit.is_empty() {
                                        // let _result = data.update(&mut cx, |myself, cx|{

                                        // });
                                        println!("Changed config files: {:?}", audit);
                                    }
                                }

                                // to prevent never read warning while also dropping previous task
                                if let Some(task) = active_update_task.take() {
                                    drop(task)
                                }

                                current_generation += 1;
                                let this_generation = current_generation;

                                // Create new window
                                let new_win_handle = cx.update(|cx| {
                                    if let Some(old_win) = win.take() {
                                        let _ = old_win.update(cx, |_, win, _| {
                                            win.remove_window();
                                        });
                                    }

                                    let new_win = spawn_launcher(
                                        cx,
                                        data.clone(),
                                        Arc::clone(&modes),
                                        initial_warnings.clone(),
                                        initial_errors.clone(),
                                    );
                                    win = Some(new_win.clone());
                                    new_win
                                });

                                // update content async
                                if let Ok(new_win) = new_win_handle {
                                    let cx_inner = cx.clone();
                                    let data_clone = data.clone();

                                    active_update_task =
                                        Some(cx.spawn(move |_cx: &mut AsyncApp| async move {
                                            let data_items = data_clone
                                                .read_with(&cx_inner, |this, _| this.clone())
                                                .ok();
                                            if let Some(items) = data_items {
                                                let mut futures: FuturesUnordered<_> = items
                                                    .iter()
                                                    .enumerate()
                                                    .filter(|(_, item)| item.is_async())
                                                    .map(|(idx, item)| async move {
                                                        (idx, item.clone().update_async().await)
                                                    })
                                                    .collect();

                                                while let Some((idx, result)) = futures.next().await
                                                {
                                                    let Some(update) = result else { continue };
                                                    let _ = cx_inner.update(|cx| {
                                                        if current_generation != this_generation {
                                                            return;
                                                        }
                                                        data_clone.update(cx, |items_arc, _cx| {
                                                            Arc::make_mut(items_arc)[idx] = update;
                                                        });
                                                        let _ =
                                                            new_win.update(cx, |view, _, cx| {
                                                                view.launcher.update(
                                                                    cx,
                                                                    |launcher, cx| {
                                                                        launcher.last_query = None;
                                                                        launcher
                                                                            .filter_and_sort(cx);
                                                                    },
                                                                );
                                                            });
                                                    });
                                                }
                                            }
                                        }));
                                }
                            } else {
                                eprintln!("Broken UNIX Socket.");
                            }
                        }
                    }
                })
                .detach();
            }
        };
    });
}

fn spawn_launcher(
    cx: &mut App,
    data: Entity<Arc<Vec<RenderableChild>>>,
    modes: Arc<[LauncherMode]>,
    initial_warnings: Vec<SherlockError>,
    initial_errors: Vec<SherlockError>,
) -> WindowHandle<SherlockWorkspace> {
    let has_errors = initial_errors.len() > 0;
    let window = cx
        .open_window(get_window_options(), |_, cx| {
            // Build launcher view
            let text_input = cx.new(|cx| TextInput::builder().placeholder("Search").build(cx));
            let launcher = cx.new(|cx| {
                let data_len = data.read(cx).len();
                let sub = cx.observe(&text_input, move |this: &mut LauncherView, _ev, cx| {
                    this.selected_index = 0;
                    this.filter_and_sort(cx);
                });
                let backspace_sub =
                    cx.subscribe(&text_input, |this, _, _ev: &EmptyBackspace, cx| {
                        if this.mode != LauncherMode::Home {
                            this.mode = LauncherMode::Home;
                            this.last_query = None;
                            this.selected_index = 0;
                            this.filter_and_sort(cx);
                        }
                    });
                text_input.update(cx, |this, _cx| {
                    this._sub = Some(backspace_sub);
                });
                let list_state = ListState::new(data_len, ListAlignment::Top, px(48.));
                let mut view = LauncherView {
                    text_input,
                    focus_handle: cx.focus_handle(),
                    list_state,
                    _subs: vec![sub],
                    selected_index: 0,
                    mode: LauncherMode::Home,
                    modes,
                    context_idx: None,
                    context_actions: Arc::new([]),
                    variable_input: Vec::new(),
                    active_bar: 0,
                    data,
                    error_count: (initial_warnings.len(), initial_errors.len()),
                    deferred_render_task: None,
                    last_query: None,
                    filtered_indices: (0..data_len).collect(),
                    config_initialized: ConfigGuard::is_initialized(),
                };
                view.filter_and_sort(cx);
                view
            });

            // Build error view
            let error = cx.new(|cx| ErrorView {
                errors: initial_errors,
                warnings: initial_warnings,
                focus_handle: cx.focus_handle(),
            });

            // Build workspace, wire up error subscription
            cx.new(|cx| {
                let error_handle = error.clone();
                let sub = cx.subscribe(
                    &launcher,
                    move |this: &mut SherlockWorkspace, _, ev: &LauncherErrorEvent, cx| {
                        match ev {
                            LauncherErrorEvent::Push(e) => {
                                error_handle.update(cx, |view, cx| {
                                    view.push(e.clone(), cx);
                                });
                            }
                            LauncherErrorEvent::ShowErrors => {
                                this.transition_to(WorkspaceView::Error, 300, cx);
                            }
                        }
                        cx.notify();
                    },
                );
                let error_sub = cx.subscribe(&error, move |this, _, _: &DismissErrorEvent, cx| {
                    this.transition_to(WorkspaceView::Launcher, 300, cx);
                });

                SherlockWorkspace {
                    launcher,
                    error,
                    workspace: if has_errors {
                        WorkspaceView::Error
                    } else {
                        WorkspaceView::Launcher
                    },
                    _subs: vec![sub, error_sub],

                    opacity: 1.0,
                    transition_task: None,
                    pending_focus: false,
                }
            })
        })
        .unwrap();

    window
        .update(cx, |view, window, cx| {
            let focus = match view.workspace {
                WorkspaceView::Launcher => view.launcher.read(cx).text_input.focus_handle(cx),
                WorkspaceView::Error => view.error.read(cx).focus_handle(cx),
            };
            window.on_next_frame(move |window, _cx| {
                window.focus(&focus);
            });
            cx.activate(true);
        })
        .unwrap();

    window
}

fn get_window_options() -> WindowOptions {
    let (width, height) = ConfigGuard::read()
        .map(|c| (c.appearance.width, c.appearance.height))
        .unwrap_or((900i32, 600i32));

    WindowOptions {
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "sherlock".to_string(),
            layer: Layer::Overlay,
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(width as f32), px(height as f32)),
        })),
        window_background: WindowBackgroundAppearance::Blurred,
        ..Default::default()
    }
}

fn register_bindings(cx: &mut App) {
    let mut bindings: HashMap<String, KeyBinding> = HashMap::new();

    let mut add = |key: &str, binding: KeyBinding| {
        bindings.insert(key.to_string(), binding);
    };

    // default binds
    add("backspace", KeyBinding::new("backspace", Backspace, None));
    add("delete", KeyBinding::new("delete", Delete, None));
    add(
        "ctrl-backspace",
        KeyBinding::new("ctrl-backspace", DeleteAll, None),
    );
    add("ctrl-a", KeyBinding::new("ctrl-a", SelectAll, None));
    add("ctrl-v", KeyBinding::new("ctrl-v", Paste, None));
    add("ctrl-c", KeyBinding::new("ctrl-c", Copy, None));
    add("ctrl-x", KeyBinding::new("ctrl-x", Cut, None));
    add("escape", KeyBinding::new("escape", Quit, None));

    add("home", KeyBinding::new("home", Home, None));
    add("end", KeyBinding::new("end", End, None));
    add("left", KeyBinding::new("left", Left, None));
    add("right", KeyBinding::new("right", Right, None));
    add("down", KeyBinding::new("down", FocusNext, None));
    add("up", KeyBinding::new("up", FocusPrev, None));
    add(
        "variable.tab",
        UIFunction::Complete.into_bind("variable.tab").unwrap(),
    );
    add("enter", KeyBinding::new("enter", Execute, None));
    add("tab", KeyBinding::new("tab", NextVar, None));
    add("shift-tab", KeyBinding::new("shift-tab", PrevVar, None));
    add("ctrl-l", KeyBinding::new("ctrl-l", OpenContext, None));

    if let Ok(config) = ConfigGuard::read() {
        for (key, action_type) in &config.keybinds {
            if *action_type == UIFunction::Shortcut && key.contains("<digit>") {
                for i in 0..=9 {
                    let actual_key = key.replace("<digit>", &i.to_string());
                    add(
                        &actual_key,
                        KeyBinding::new(&actual_key, ShortcutAction { index: i }, None),
                    );
                }
            } else if let Some(binding) = action_type.into_bind(key) {
                add(key, binding);
            }
        }
    }

    cx.bind_keys(bindings.into_values().collect::<Vec<_>>());
}
