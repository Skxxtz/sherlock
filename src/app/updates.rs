use gpui::{AsyncApp, WindowHandle};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    sync::Notify,
};

use crate::{
    app::{RenderableChildEntity, run_async_updates, spawn_launcher},
    sherlock_msg,
    tokio_utils::AsyncSizedMessage,
    ui::launcher::{LauncherMode, LauncherView},
    utils::{
        config::{ConfigGuard, ConfigWatcher, SherlockFlags, reload},
        errors::{
            SherlockMessage,
            types::{SherlockErrorType, SocketAction},
        },
    },
};

pub(super) async fn run_event_loop(
    cx: AsyncApp,
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
    let slot: Arc<Mutex<Option<Result<SherlockFlags, SherlockMessage>>>> =
        Arc::new(Mutex::new(None));
    let notify = Arc::new(Notify::new());

    tokio::spawn({
        let slot = Arc::clone(&slot);
        let notify = Arc::clone(&notify);
        async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let result = process_connection(stream).await;
                        *slot.lock().unwrap() = Some(result);
                        notify.notify_one();
                    }
                    Err(e) => eprintln!("Socket error: {e}"),
                }
            }
        }
    });

    loop {
        notify.notified().await;
        let msg = match slot.lock().unwrap().take() {
            Some(m) => m,
            None => continue,
        };

        match msg {
            Ok(mut flags) => {
                if let Err(e) = ConfigGuard::write_with(|c| c.apply_flags(&mut flags)) {
                    initial_messages.push(e);
                }
            }
            Err(e) => {
                initial_messages.push(e);
            }
        }

        // Config reload
        if let Ok(audit) = watcher.audit()
            && !audit.is_empty()
            && let Some(new_modes) = reload(&cx, &data, &mut initial_messages, audit)
        {
            modes = new_modes;
        }

        drop(active_update_task.take());
        cx.update(|cx| {
            if let Some(old_win) = win.take() {
                let _ = old_win.update(cx, |_, win, _| win.remove_window());
            }
        });

        let new_win_handle = cx.update(|cx| {
            let new_win = spawn_launcher(
                cx,
                data.clone(),
                Arc::clone(&modes),
                initial_messages.clone(),
            );
            win = Some(new_win);
            new_win
        });

        active_update_task = Some(cx.spawn(move |cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let _ = run_async_updates(&mut cx, new_win_handle).await;
            }
        }));
    }
}

// Now returns the parsed result instead of mutating — no cx needed
async fn process_connection(mut stream: UnixStream) -> Result<SherlockFlags, SherlockMessage> {
    match tokio::time::timeout(Duration::from_millis(200), stream.readable()).await {
        Ok(Ok(_)) => {}
        _ => {
            stream.shutdown().await.ok();
            drop(stream);
            return Err(sherlock_msg!(
                Error,
                SherlockErrorType::SocketError(SocketAction::Read),
                "Socket read timeout"
            ));
        }
    }

    let read_result = stream.read_sized::<SherlockFlags>().await;
    stream.shutdown().await.ok();
    drop(stream);
    read_result
}
