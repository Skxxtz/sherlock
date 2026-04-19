use gpui::{App, Application, QuitMode};
use once_cell::sync::OnceCell;
use std::{
    io::Write,
    sync::{OnceLock, RwLock},
};

use crate::{
    app::{bindings::ShortcutKeyMod, run_app},
    loader::{CustomIconTheme, Loader, assets::Assets},
    utils::{clipboard::spawn_clipboard_watcher, config::SherlockConfig},
};

mod app;
mod launcher;
mod loader;
mod prelude;
mod ui;
mod utils;

/// Holds the icon cache, containing all known icon names and their file locations.
static ICONS: OnceCell<RwLock<CustomIconTheme>> = OnceCell::new();
/// Holed the global config struct for user-specified config values.
static CONFIG: OnceCell<RwLock<SherlockConfig>> = OnceCell::new();
/// Holds the string used to show and hide the context menu.
static CONTEXT_MENU_BIND: OnceLock<String> = OnceLock::new();
/// Holds the modifier key char
static SHORTCUT_MOD: OnceLock<ShortcutKeyMod> = OnceLock::new();
/// Holds the socket location for the sherlock socket
static SOCKET_PATH: &'static str = "/tmp/sherlock.sock";

#[tokio::main]
async fn main() {
    let socket_path = "/tmp/sherlock.sock";
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
        let _ = stream.write_all(b"open");
        return;
    }

    spawn_clipboard_watcher();

    let plat = gpui_platform::current_platform(false);
    let app = Application::with_platform(plat)
        .with_assets(Assets)
        .with_quit_mode(QuitMode::Explicit);

    app.run(|cx: &mut App| run_app(cx, Loader::setup()));
}
