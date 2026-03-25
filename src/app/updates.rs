use gpui::{AsyncApp, Entity, WindowHandle};
use std::sync::Arc;
use tokio::net::UnixListener;

use crate::{
    app::{run_async_updates, spawn_launcher},
    launcher::children::RenderableChild,
    ui::{launcher::LauncherMode, workspace::SherlockWorkspace},
    utils::{
        config::{ConfigWatcher, reload},
        errors::SherlockError,
    },
};

pub(super) async fn run_event_loop(
    cx: AsyncApp,
    data: Entity<Arc<Vec<RenderableChild>>>,
    mut modes: Arc<[LauncherMode]>,
    mut watcher: ConfigWatcher,
    listener: UnixListener,
    mut initial_errors: Vec<SherlockError>,
    mut initial_warnings: Vec<SherlockError>,
) {
    let mut win: Option<WindowHandle<SherlockWorkspace>> = None;
    let mut current_generation: u64 = 0;
    let mut active_update_task: Option<gpui::Task<()>> = None;

    loop {
        if let Ok((_stream, _)) = listener.accept().await {
            if let Ok(audit) = watcher.audit() {
                if !audit.is_empty() {
                    if let Some(new_modes) = reload(
                        &cx,
                        &data,
                        &mut initial_errors,
                        &mut initial_warnings,
                        audit,
                    )
                    .await
                    {
                        modes = new_modes;
                    }
                }
            }

            drop(active_update_task.take());
            current_generation += 1;
            let this_generation = current_generation;

            let new_win_handle = cx.update(|cx| {
                if let Some(old_win) = win.take() {
                    let _ = old_win.update(cx, |_, win, _| win.remove_window());
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

            let cx_inner = cx.clone();
            let data_clone = data.clone();
            active_update_task = Some(cx.spawn(move |_: &mut AsyncApp| {
                run_async_updates(
                    cx_inner,
                    data_clone,
                    new_win_handle,
                    current_generation,
                    this_generation,
                )
            }));
        } else {
            eprintln!("Broken UNIX Socket.");
        }
    }
}
