use futures::{StreamExt, stream::FuturesUnordered};
use gpui::{
    App, AppContext, AsyncApp, Bounds, Entity, Focusable, ListAlignment, ListState, Size,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::{Layer, LayerShellOptions},
    point, px,
};
use std::sync::Arc;
use tokio::net::UnixListener;

use crate::{
    SOCKET_PATH,
    launcher::children::{LauncherValues, RenderableChild},
    loader::{LauncherLoadResult, Loader, SetupResult},
    ui::{
        error::view::{DismissErrorEvent, ErrorView},
        launcher::{LauncherMode, LauncherView},
        search_bar::{EmptyBackspace, TextInput},
        workspace::{LauncherErrorEvent, SherlockWorkspace, WorkspaceView},
    },
    utils::{
        config::{ConfigGuard, ConfigWatcher},
        errors::SherlockError,
    },
};

mod bindings;
mod updates;

pub fn run_app(cx: &mut App, result: SetupResult) {
    let SetupResult {
        config_dir,
        mut errors,
        mut warnings,
    } = result;
    let watcher = ConfigWatcher::new(config_dir);

    bindings::register_bindings(cx);

    let data: Entity<Arc<Vec<RenderableChild>>> = cx.new(|_| Arc::new(Vec::new()));
    let modes = load_modes(cx, &data, &mut errors, &mut warnings);

    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH).unwrap();
    let initial_errors = errors;
    let initial_warnings = warnings;

    cx.spawn(|cx: &mut AsyncApp| {
        let cx = cx.clone();
        async move {
            updates::run_event_loop(
                cx,
                data,
                modes,
                watcher,
                listener,
                initial_errors,
                initial_warnings,
            )
            .await;
        }
    })
    .detach();
}

fn load_modes(
    cx: &mut App,
    data: &Entity<Arc<Vec<RenderableChild>>>,
    errors: &mut Vec<SherlockError>,
    warnings: &mut Vec<SherlockError>,
) -> Arc<[LauncherMode]> {
    match Loader::load_launchers(cx, data.clone()) {
        Ok(LauncherLoadResult {
            modes,
            warnings: warns,
        }) => {
            warnings.extend(warns);
            modes
        }
        Err(e) => {
            errors.push(e);
            Arc::from([])
        }
    }
}

pub async fn run_async_updates(
    cx: AsyncApp,
    data: Entity<Arc<Vec<RenderableChild>>>,
    new_win: WindowHandle<SherlockWorkspace>,
    current_generation: u64,
    this_generation: u64,
) {
    let data_items = data.read_with(&cx, |this, _| this.clone()).ok();
    let Some(items) = data_items else { return };

    let mut futures: FuturesUnordered<_> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.is_async())
        .map(|(idx, item)| async move { (idx, item.clone().update_async().await) })
        .collect();

    while let Some((idx, result)) = futures.next().await {
        let Some(update) = result else { continue };
        let _ = cx.update(|cx| {
            if current_generation != this_generation {
                return;
            }
            data.update(cx, |items_arc, _| {
                Arc::make_mut(items_arc)[idx] = update;
            });
            let _ = new_win.update(cx, |view, _, cx| {
                view.launcher.update(cx, |launcher, cx| {
                    launcher.last_query = None;
                    launcher.filter_and_sort(cx);
                });
            });
        });
    }
}

fn spawn_launcher(
    cx: &mut App,
    data: Entity<Arc<Vec<RenderableChild>>>,
    modes: Arc<[LauncherMode]>,
    initial_warnings: Vec<SherlockError>,
    initial_errors: Vec<SherlockError>,
) -> WindowHandle<SherlockWorkspace> {
    let has_errors = !initial_errors.is_empty();
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
