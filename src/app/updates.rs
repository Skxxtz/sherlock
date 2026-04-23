use gpui::{AsyncApp, WindowHandle};
use std::sync::Arc;
use tokio::net::UnixListener;

use crate::{
    app::{RenderableChildEntity, run_async_updates, spawn_launcher},
    tokio_utils::AsyncSizedMessage,
    ui::launcher::{LauncherMode, LauncherView},
    utils::{
        config::{ConfigGuard, ConfigWatcher, SherlockFlags, reload},
        errors::SherlockMessage,
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

    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            if let Ok(audit) = watcher.audit()
                && !audit.is_empty()
                && let Some(new_modes) = reload(&cx, &data, &mut initial_messages, audit).await
            {
                modes = new_modes;
            }

            match stream.read_sized::<SherlockFlags>().await {
                Ok(mut flags) => {
                    if let Err(e) = ConfigGuard::write_with(|c| c.apply_flags(&mut flags)) {
                        initial_messages.push(e)
                    }
                }
                Err(e) => {
                    initial_messages.push(e);
                }
            }

            drop(active_update_task.take());
            let new_win_handle = cx.update(|cx| {
                if let Some(old_win) = win.take() {
                    let _ = old_win.update(cx, |_, win, _| win.remove_window());
                }
                let new_win = spawn_launcher(
                    cx,
                    data.clone(),
                    Arc::clone(&modes),
                    initial_messages.clone(),
                );
                win = Some(new_win);
                new_win
            });

            let cx_inner = cx.clone();
            let _ = run_async_updates(cx_inner, new_win_handle).await;
        } else {
            eprintln!("Broken UNIX Socket.");
        }
    }
}
