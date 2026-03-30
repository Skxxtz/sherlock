use gpui::{
    App, AppContext, AsyncApp, Bounds, Entity, Focusable, Hsla, Size, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, hsla,
    layer_shell::{Layer, LayerShellOptions},
    point, px,
};
use std::sync::Arc;
use tokio::net::UnixListener;

use crate::{
    SOCKET_PATH,
    launcher::children::RenderableChild,
    loader::{LauncherLoadResult, Loader, SetupResult},
    ui::{
        launcher::{
            LauncherMode, LauncherView,
            views::{NavigationStack, NavigationViewType},
        },
        model::Model,
        search_bar::{EmptyBackspace, TextInput},
    },
    utils::{
        config::{ConfigGuard, ConfigWatcher},
        errors::SherlockMessage,
    },
};

mod bindings;
mod updates;

#[derive(Clone)]
pub struct ActiveTheme(pub Arc<ThemeData>);

pub struct ThemeData {
    pub bg_selected: Hsla,
    pub bg_idle: Hsla,
    pub mantle: Hsla,
    pub border_selected: Hsla,
    pub border_idle: Hsla,
    pub primary_text: Hsla,
    pub secondary_text: Hsla,

    pub color_warn: Hsla,
    pub color_err: Hsla,
    pub color_succ: Hsla,
}

impl gpui::Global for ActiveTheme {}

impl ThemeData {
    pub fn dark() -> Self {
        Self {
            bg_selected: hsla(0.0, 0.0, 1.0, 0.1),
            bg_idle: hsla(0.0, 0.0, 0.0, 0.0),
            mantle: hsla(0.0, 0.0, 1.0, 0.1),
            border_selected: hsla(0.0, 0.0, 1.0, 0.2),
            border_idle: hsla(0.0, 0.0, 1.0, 0.05),
            primary_text: hsla(0.0, 0.0, 0.95, 1.0),
            secondary_text: hsla(0.0, 0.0, 0.6, 1.0),

            color_warn: hsla(45.0 / 360.0, 0.85, 0.65, 1.0),
            color_err: hsla(0.0 / 360.0, 0.85, 0.65, 1.0),
            color_succ: hsla(145.0 / 360.0, 0.75, 0.60, 1.0),
        }
    }
}

pub fn run_app(cx: &mut App, result: SetupResult) {
    let SetupResult {
        config_dir,
        mut messages,
    } = result;
    let watcher = ConfigWatcher::new(config_dir);

    bindings::register_bindings(cx);

    let theme = ActiveTheme(Arc::new(ThemeData::dark()));
    cx.set_global(theme);

    let data: Entity<Arc<Vec<RenderableChild>>> = cx.new(|_| Arc::new(Vec::new()));
    let modes = load_modes(cx, &data, &mut messages);

    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH).unwrap();
    let initial_messages = messages;

    cx.spawn(|cx: &mut AsyncApp| {
        let cx = cx.clone();
        async move {
            updates::run_event_loop(cx, data, modes, watcher, listener, initial_messages).await;
        }
    })
    .detach();
}

fn load_modes(
    cx: &mut App,
    data: &Entity<Arc<Vec<RenderableChild>>>,
    messages: &mut Vec<SherlockMessage>,
) -> Arc<[LauncherMode]> {
    match Loader::load_launchers(cx, data.clone()) {
        Ok(LauncherLoadResult {
            modes,
            messages: msgs,
        }) => {
            messages.extend(msgs);
            modes
        }
        Err(e) => {
            messages.push(e);
            Arc::from([])
        }
    }
}

#[inline(always)]
pub async fn run_async_updates(mut cx: AsyncApp, win: WindowHandle<LauncherView>) {
    let _ = win.update(&mut cx, |this, _win, cx| {
        this.update_async(cx);
    });
}

fn spawn_launcher(
    cx: &mut App,
    data: Entity<Arc<Vec<RenderableChild>>>,
    modes: Arc<[LauncherMode]>,
    initial_messages: Vec<SherlockMessage>,
) -> WindowHandle<LauncherView> {
    let window = cx
        .open_window(get_window_options(), |_, cx| {
            // Build launcher view
            let text_input = cx.new(|cx| TextInput::builder().placeholder("Search").build(cx));
            let launcher = cx.new(|cx| {
                let data_len = data.read(cx).len();
                let sub = cx.observe(&text_input, move |this: &mut LauncherView, _ev, cx| {
                    this.context_idx = None;
                    this.navigation.current_mut().reset_selected_index();
                    this.filter_and_sort(cx);
                });
                let backspace_sub =
                    cx.subscribe(&text_input, |this, _, _ev: &EmptyBackspace, cx| {
                        if this.navigation.current_kind() != NavigationViewType::Home {
                            this.navigation.set_prev_and_cleanup();
                            let content = this
                                .navigation
                                .with_model(cx, |mdl| mdl.last_query());

                            if let Some(c) = content {
                                this.text_input.update(cx, |ipt, _| ipt.set_text(c));
                            }

                            this.filter_and_sort(cx);
                            cx.notify();
                        } else {
                            if this.mode != LauncherMode::Home {
                                this.mode = LauncherMode::Home;
                                this.navigation.with_model_mut(cx, |mdl, _| {
                                    if let Model::Standard { last_query, .. } = mdl {
                                        *last_query = None
                                    }
                                });
                                this.navigation.current_mut().reset_selected_index();
                                this.filter_and_sort(cx);
                            }
                            cx.notify();
                        }
                    });
                text_input.update(cx, |this, _cx| {
                    this._sub = Some(backspace_sub);
                });
                let mut view = LauncherView {
                    text_input,
                    focus_handle: cx.focus_handle(),
                    _subs: vec![sub],
                    mode: LauncherMode::Home,
                    modes,
                    context_idx: None,
                    has_actions: false,
                    context_actions: Arc::new([]),
                    variable_input: Vec::new(),
                    active_bar: 0,
                    navigation: NavigationStack::new(data, initial_messages, data_len, cx),
                    config_initialized: ConfigGuard::is_initialized(),
                    active_update_task: None,
                };
                view.filter_and_sort(cx);
                view
            });

            launcher
        })
        .unwrap();

    window
        .update(cx, |view, window, cx| {
            let focus = view.text_input.focus_handle(cx);
            window.on_next_frame(move |window, cx| {
                window.focus(&focus, cx);
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
