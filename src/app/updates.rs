use gpui::{AsyncApp, Entity, WindowHandle};
use std::sync::Arc;
use tokio::net::UnixListener;

use crate::{
    app::{run_async_updates, spawn_launcher},
    launcher::children::RenderableChild,
    ui::launcher::{LauncherMode, LauncherView},
    utils::{
        config::{ConfigWatcher, reload},
        errors::SherlockMessage,
    },
};

pub(super) async fn run_event_loop(
    cx: AsyncApp,
    data: Entity<Arc<Vec<RenderableChild>>>,
    mut modes: Arc<[LauncherMode]>,
    mut watcher: ConfigWatcher,
    listener: UnixListener,
    mut initial_messages: Vec<SherlockMessage>,
) {
    let mut win: Option<WindowHandle<LauncherView>> = None;
    let mut active_update_task: Option<gpui::Task<()>> = None;

    loop {
        if let Ok((_stream, _)) = listener.accept().await {
            if let Ok(audit) = watcher.audit() {
                if !audit.is_empty() {
                    if let Some(new_modes) = reload(&cx, &data, &mut initial_messages, audit).await
                    {
                        modes = new_modes;
                    }
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
                win = Some(new_win.clone());
                new_win
            });

            let cx_inner = cx.clone();
            let _ = run_async_updates(cx_inner, new_win_handle).await;
        } else {
            eprintln!("Broken UNIX Socket.");
        }
    }
}
