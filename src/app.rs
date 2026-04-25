use gpui::{
    App, AppContext, AsyncApp, Bounds, Entity, Size, WeakEntity, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::{Layer, LayerShellOptions},
    point, px,
};
use std::{
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};
use tokio::net::UnixListener;

use crate::{
    SOCKET_PATH,
    app::theme::{ActiveTheme, ThemeData},
    loader::{LauncherLoadResult, Loader, SetupResult},
    ui::{
        launcher::{
            LauncherMode, LauncherView,
            views::{NavigationStack, NavigationViewType},
        },
        model::Model,
        search_bar::{EmptyBackspace, TextInput},
        widgets::RenderableChild,
    },
    utils::{
        config::{ConfigGuard, ConfigWatcher},
        errors::SherlockMessage,
    },
};

pub mod bindings;
pub mod theme;
mod updates;

pub static LAUNCH_GENERATION: AtomicU32 = AtomicU32::new(0);
pub fn reset_generation() {
    LAUNCH_GENERATION.fetch_add(1, Ordering::Relaxed);
}

pub type RenderableChildEntity = Entity<Rc<Vec<RenderableChild>>>;
pub type RenderableChildWeak = WeakEntity<Rc<Vec<RenderableChild>>>;

pub fn run_app(cx: &mut App, result: SetupResult) {
    let SetupResult {
        config_dir,
        mut messages,
    } = result;
    let watcher = ConfigWatcher::new(config_dir);

    bindings::register_bindings(cx);

    let theme = ActiveTheme(Arc::new(ThemeData::dark()));
    cx.set_global(theme);

    let data: RenderableChildEntity = cx.new(|_| Rc::new(Vec::new()));
    let modes = load_modes(cx, &data, &mut messages);

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
    data: &RenderableChildEntity,
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
pub async fn run_async_updates(cx: &mut AsyncApp, win: WindowHandle<LauncherView>) {
    let _ = win.update(cx, |this, _win, cx| {
        this.update_async(cx);
    });
}

fn spawn_launcher(
    cx: &mut App,
    data: RenderableChildEntity,
    modes: Arc<[LauncherMode]>,
    initial_messages: Vec<SherlockMessage>,
) -> WindowHandle<LauncherView> {
    cx.open_window(get_window_options(), |_, cx| {
        let text_input = cx.new(|cx| TextInput::builder().placeholder("Search").build(cx));

        cx.new(|cx| {
            let data_len = data.read(cx).len();

            let sub = cx.observe(&text_input, |this: &mut LauncherView, _, cx| {
                this.context_idx = None;
                this.navigation.current_mut().reset_selected_index();
                this.filter_and_sort(cx);
            });

            let backspace_sub = cx.subscribe(&text_input, |this, _, _: &EmptyBackspace, cx| {
                if this.navigation.current_kind() != NavigationViewType::Home {
                    this.navigation.set_prev_and_cleanup();
                    if let Some(c) = this.navigation.with_model(cx, |mdl| mdl.last_query()) {
                        this.text_input.update(cx, |ipt, _| ipt.set_text(c));
                    }
                    this.filter_and_sort(cx);
                } else if this.mode != LauncherMode::Home {
                    this.mode = LauncherMode::Home;
                    this.navigation.with_model_mut(cx, |mdl, _| {
                        if let Model::Standard { last_query, .. } = mdl {
                            *last_query = None;
                        }
                    });
                    this.navigation.current_mut().reset_selected_index();
                    this.filter_and_sort(cx);
                }
                cx.notify();
            });

            text_input.update(cx, |this, _| this._sub = Some(backspace_sub));

            let mut navigation = NavigationStack::new(data, initial_messages, data_len, cx);

            let mode = ConfigGuard::read_with(|c| {
                c.runtime
                    .sub_menu
                    .as_deref()
                    .and_then(|submenu| {
                        modes.iter().find(|m| matches!(
                            m,
                            LauncherMode::Alias { short, .. } if short.eq_ignore_ascii_case(submenu)
                        ))
                    })
                    .cloned()
                    .unwrap_or(LauncherMode::Home)
            })
            .unwrap_or(LauncherMode::Home);

            if let LauncherMode::Alias { launcher, .. } = &mode
                && let Ok(view) = NavigationViewType::try_from(&launcher.launcher_type)
            {
                navigation.push(view.create_view(launcher.clone(), cx));
            }

            LauncherView {
                text_input,
                focus_handle: cx.focus_handle(),
                _subs: vec![sub],
                mode,
                modes,
                context_idx: None,
                has_actions: false,
                context_actions: Arc::new([]),
                variable_input: Vec::new(),
                active_bar: 0,
                navigation,
                config_initialized: ConfigGuard::is_initialized(),
                active_update_task: None,
            }
        })
    })
    .unwrap()
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
