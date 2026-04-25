use gpui::{AppContext, AsyncApp, Focusable, WindowHandle};
use serde::de::DeserializeOwned;
use std::{
    collections::VecDeque,
    fmt::Debug,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    sync::Notify,
};

use crate::{
    app::{RenderableChildEntity, run_async_updates, spawn_launcher},
    launcher::Launcher,
    sherlock_msg,
    tokio_utils::AsyncSizedMessage,
    ui::{
        launcher::{LauncherMode, LauncherView, views::NavigationViewType},
        widgets::RenderableChild,
    },
    utils::{
        config::{ConfigGuard, ConfigWatcher, reload},
        errors::{
            SherlockMessage,
            types::{SherlockErrorType, SocketAction},
        },
        networking::ClientMessage,
    },
};

pub(super) async fn run_event_loop(
    mut cx: AsyncApp,
    data: RenderableChildEntity,
    mut modes: Arc<[LauncherMode]>,
    mut watcher: ConfigWatcher,
    listener: UnixListener,
    mut initial_messages: Vec<SherlockMessage>,
) {
    let mut win: Option<WindowHandle<LauncherView>> = None;
    let mut active_update_task: Option<gpui::Task<()>> = None;

    // Bridges the Tokio runtime (which owns the `UnixListener`) and GPUI's internal runtime.
    // `mpsc` channels fail here because GPUI's executor does not continuously poll `rx.recv()`,
    // causing `tx.try_send()` to return `Full` while the receiver waits indefinitely.
    // Moving `accept()` to GPUI's background executor breaks `UnixListener` due to missing
    // Tokio reactor context. Instead, the listener stays in `tokio::spawn`, pushes results
    // into a `Mutex<Option<T>>`, and signals GPUI via `Notify` — which is safe across runtimes
    // because it only uses wakers, unlike reactor-dependent primitives (`sleep`, `TcpStream`).
    // Can be verified by running:
    // ```for i in {1..1024}; do target/release/sherlock; done```
    // against a running sherlock instance.
    let slot: Arc<Mutex<VecDeque<Result<ClientMessage, SherlockMessage>>>> =
        Arc::new(Mutex::new(VecDeque::new()));
    let notify = Arc::new(Notify::new());

    tokio::spawn({
        let slot = Arc::clone(&slot);
        let notify = Arc::clone(&notify);
        async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        // Process all messages until the client disconnects
                        let _ = process_connection(stream, |msg| {
                            let mut q = slot.lock().unwrap();
                            q.push_back(msg);
                            notify.notify_one();
                        })
                        .await;
                    }
                    Err(e) => eprintln!("Socket error: {e}"),
                }
            }
        }
    });

    loop {
        notify.notified().await;
        for result in slot.lock().unwrap().drain(..) {
            let msg = match result {
                Ok(m) => m,
                Err(e) => {
                    initial_messages.push(e);
                    continue;
                }
            };

            // handle new config transfer
            if let ClientMessage::ConfigUpdate(mut flags) = msg {
                if let Err(e) = ConfigGuard::write_with(|c| c.apply_flags(&mut flags)) {
                    initial_messages.push(e);
                }
                continue;
            }

            // Config reload
            if let Ok(audit) = watcher.audit()
                && !audit.is_empty()
                && let Some(new_modes) = reload(&cx, &data, &mut initial_messages, audit)
            {
                modes = new_modes;
            }

            // prepare new window / launcher
            if !win.as_ref().is_some_and(|win| {
                win.update(&mut cx, |_, win, _| win.is_window_active()) // this call is just dummy
                    .is_ok()
            }) {
                refresh_launcher_view(
                    data.clone(),
                    Arc::clone(&modes),
                    &initial_messages,
                    &mut win,
                    &mut cx,
                );
            };

            // Parse potential initial data such as dmenu piped input
            if let ClientMessage::Dmenu(items) = &msg {
                let entity = cx.new(move |_| {
                    let launcher = Arc::new(Launcher::default_dmenu());
                    Rc::new(
                        items
                            .iter()
                            .map(|s| RenderableChild::Dmenu {
                                launcher: launcher.clone(),
                                inner: s.into(),
                            })
                            .collect(),
                    )
                });

                add_data_page(entity, &win, &mut cx);
            }

            // Handle open request
            if let ClientMessage::Open = msg {
                open_window(&win, &mut cx);

                drop(active_update_task.take());
                active_update_task = Some(cx.spawn(move |cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Some(win) = win {
                            let _ = run_async_updates(&mut cx, win).await;
                        }
                    }
                }));
            }
        }
    }
}

async fn process_connection<T, F>(
    mut stream: UnixStream,
    mut on_message: F,
) -> Result<(), SherlockMessage>
where
    T: DeserializeOwned + Debug,
    F: FnMut(Result<T, SherlockMessage>),
{
    loop {
        let read_result = stream.read_sized::<T>().await;

        match read_result {
            Ok(msg) => {
                on_message(Ok(msg));
            }
            Err(e)
                if matches!(
                    e.error_type,
                    SherlockErrorType::SocketError(SocketAction::EoF)
                ) =>
            {
                break;
            }
            _ => {
                let err = sherlock_msg!(
                    Error,
                    SherlockErrorType::SocketError(SocketAction::Read),
                    "Socket error or timeout"
                );
                on_message(Err(err));
            }
        }
    }

    stream.shutdown().await.ok();
    Ok(())
}

fn refresh_launcher_view(
    data: RenderableChildEntity,
    modes: Arc<[LauncherMode]>,
    initial_messages: &[SherlockMessage],
    win: &mut Option<WindowHandle<LauncherView>>,
    cx: &mut AsyncApp,
) {
    // drop old window
    cx.update(|cx| {
        if let Some(old_win) = win.take() {
            let _ = old_win.update(cx, |_, win, _| win.remove_window());
        }
    });

    // create new window
    *win = Some(cx.update(|cx| spawn_launcher(cx, data, modes, initial_messages.to_owned())));
}

fn open_window(win: &Option<WindowHandle<LauncherView>>, cx: &mut AsyncApp) {
    if let Some(current_window) = win {
        // actually open the window
        let _ = current_window.update(cx, |view, window, cx| {
            let focus = view.text_input.focus_handle(cx);
            window.on_next_frame(move |window, cx| window.focus(&focus, cx));
            view.filter_and_sort(cx);
            cx.activate(true);
        });
    }
}

fn add_data_page(
    entity: RenderableChildEntity,
    win: &Option<WindowHandle<LauncherView>>,
    cx: &mut AsyncApp,
) {
    if let Some(current_window) = win {
        let _ = current_window.update(cx, |this, _win, cx| {
            let launcher = Arc::new(Launcher::default_dmenu());
            this.navigation
                .push(NavigationViewType::Dmenu { entity }.create_view(launcher, cx));
        });
    }
}
