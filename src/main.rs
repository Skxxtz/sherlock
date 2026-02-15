use futures::future::join_all;
use once_cell::sync::OnceCell;
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
        main_window::{LauncherMode, NextVar, OpenContext, PrevVar},
        search_bar::{EmptyBackspace, ShortcutAction},
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

use ui::main_window::{Execute, FocusNext, FocusPrev, Quit, SherlockMainWindow};
use ui::search_bar::{
    Backspace, Copy, Cut, Delete, DeleteAll, End, Home, Left, Paste, Right, SelectAll, TextInput,
};

use utils::errors::SherlockError;

static ICONS: OnceCell<RwLock<CustomIconTheme>> = OnceCell::new();
static CONFIG: OnceCell<RwLock<SherlockConfig>> = OnceCell::new();

static CONTEXT_MENU_BIND: OnceLock<String> = OnceLock::new();

fn setup() -> Result<Box<Path>, SherlockError> {
    let mut flags = Loader::load_flags()?;

    let config = flags.to_config().map_or_else(
        |e| {
            eprintln!("{e}");
            let defaults = SherlockConfig::default();
            SherlockConfig::apply_flags(&mut flags, defaults)
        },
        |(cfg, non_crit)| {
            if !non_crit.is_empty() {
                eprintln!("{:?}", non_crit);
            }
            cfg
        },
    );

    // Load custom icons
    let _ = ICONS.set(RwLock::new(CustomIconTheme::new()));
    config.appearance.icon_paths.iter().for_each(|path| {
        if let Err(e) = IconThemeGuard::add_path(path) {
            eprintln!("{:?}", e);
        }
    });

    // Create global config
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

    Ok(config_dir)
}

#[tokio::main]
async fn main() {
    // connect to existing socket
    let socket_path = "/tmp/sherlock.sock";
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
        let _ = stream.write_all(b"open");
        return;
    }

    let mut watcher = match setup() {
        Ok(root) => ConfigWatcher::new(root),
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    // start primary instance
    let app = Application::new().with_assets(Assets);
    app.with_quit_mode(QuitMode::Explicit).run(|cx: &mut App| {
        let mut final_bindings: HashMap<String, KeyBinding> = HashMap::new();

        let mut add_binding = |key: &str, binding: KeyBinding| {
            final_bindings.insert(key.to_string(), binding);
        };

        // default binds
        add_binding("backspace", KeyBinding::new("backspace", Backspace, None));
        add_binding("delete", KeyBinding::new("delete", Delete, None));
        add_binding(
            "ctrl-backspace",
            KeyBinding::new("ctrl-backspace", DeleteAll, None),
        );
        add_binding("ctrl-a", KeyBinding::new("ctrl-a", SelectAll, None));
        add_binding("ctrl-v", KeyBinding::new("ctrl-v", Paste, None));
        add_binding("ctrl-c", KeyBinding::new("ctrl-c", Copy, None));
        add_binding("ctrl-x", KeyBinding::new("ctrl-x", Cut, None));
        add_binding("escape", KeyBinding::new("escape", Quit, None));

        add_binding("home", KeyBinding::new("home", Home, None));
        add_binding("end", KeyBinding::new("end", End, None));
        add_binding("left", KeyBinding::new("left", Left, None));
        add_binding("right", KeyBinding::new("right", Right, None));
        add_binding("down", KeyBinding::new("down", FocusNext, None));
        add_binding("up", KeyBinding::new("up", FocusPrev, None));
        add_binding("enter", KeyBinding::new("enter", Execute, None));
        add_binding("tab", KeyBinding::new("tab", NextVar, None));
        add_binding("shift-tab", KeyBinding::new("shift-tab", PrevVar, None));
        add_binding("ctrl-l", KeyBinding::new("ctrl-l", OpenContext, None));

        if let Ok(config) = ConfigGuard::read() {
            for (key, action_type) in &config.keybinds {
                if *action_type == UIFunction::Shortcut && key.contains("<digit>") {
                    for i in 0..=9 {
                        let actual_key = key.replace("<digit>", &i.to_string());
                        add_binding(
                            &actual_key,
                            KeyBinding::new(&actual_key, ShortcutAction { index: i }, None),
                        );
                    }
                } else if let Some(binding) = action_type.into_bind(key) {
                    add_binding(key, binding);
                }
            }
        }

        cx.bind_keys(final_bindings.into_values().collect::<Vec<_>>());

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

        cx.spawn(|cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                let mut win: Option<WindowHandle<SherlockMainWindow>> = None;
                let mut current_generation: u64 = 0;
                let mut active_update_task: Option<gpui::Task<()>> = None;
                loop {
                    if let Ok((_stream, _)) = listener.accept().await {
                        // check if config files changed
                        if let Ok(audit) = watcher.audit() {
                            if !audit.is_empty() {
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

                            let new_win = spawn_launcher(cx, data.clone(), Arc::clone(&modes));
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
                                        let update_futures = items
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, item)| item.is_async())
                                            .map(|(idx, item)| async move {
                                                (idx, item.clone().update_async().await)
                                            });

                                        let updates = join_all(update_futures).await;

                                        let _ = cx_inner.update(|cx| {
                                            if current_generation != this_generation {
                                                return;
                                            }

                                            if !updates.is_empty() {
                                                data_clone.update(cx, |items_arc, _cx| {
                                                    let items_vec = Arc::make_mut(items_arc);
                                                    for (idx, update) in updates
                                                        .into_iter()
                                                        .filter_map(|(i, u)| u.map(|v| (i, v)))
                                                    {
                                                        items_vec[idx] = update;
                                                    }
                                                });

                                                let _ = new_win.update(cx, |view, _, cx| {
                                                    view.last_query = None; // forces update
                                                    view.filter_and_sort(cx);
                                                });
                                            }
                                        });
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
    });
}

fn spawn_launcher(
    cx: &mut App,
    data: Entity<Arc<Vec<RenderableChild>>>,
    modes: Arc<[LauncherMode]>,
) -> WindowHandle<SherlockMainWindow> {
    // For now load application here
    let window = cx
        .open_window(get_window_options(), |_, cx| {
            let text_input = cx.new(|cx| TextInput {
                focus_handle: cx.focus_handle(),
                content: "".into(),
                placeholder: "Search:".into(),
                variable: None,
                selected_range: 0..0,
                selection_reversed: false,
                marked_range: None,
                last_layout: None,
                last_bounds: None,
                is_selecting: false,
            });
            cx.new(|cx| {
                let data_len = data.read(cx).len();
                let sub = cx.observe(
                    &text_input,
                    move |this: &mut SherlockMainWindow, _ev, cx| {
                        this.selected_index = 0;
                        this.filter_and_sort(cx);
                    },
                );
                let backspace_sub =
                    cx.subscribe(&text_input, |this, _, _ev: &EmptyBackspace, cx| {
                        if this.mode != LauncherMode::Home {
                            this.mode = LauncherMode::Home;

                            // Propagate changes to ui
                            this.last_query = None;
                            this.selected_index = 0;
                            this.filter_and_sort(cx);
                        }
                    });

                let list_state = ListState::new(data_len, ListAlignment::Top, px(48.));

                let mut view = SherlockMainWindow {
                    text_input,
                    focus_handle: cx.focus_handle(),
                    list_state,
                    _subs: vec![sub, backspace_sub],
                    selected_index: 0,
                    // modes
                    mode: LauncherMode::Home,
                    modes,
                    // context menu
                    context_idx: None,
                    context_actions: Arc::new([]),
                    // variable inputs
                    variable_input: Vec::new(),
                    active_bar: 0,
                    // Data model
                    data,
                    deferred_render_task: None,
                    last_query: None,
                    filtered_indices: (0..data_len).collect(),
                };
                view.filter_and_sort(cx);

                view
            })
        })
        .unwrap();

    window
        .update(cx, |view, window, cx| {
            window.focus(&view.text_input.focus_handle(cx));
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
