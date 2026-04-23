use ::tokio::net::UnixStream;
use gpui::{App, Application, QuitMode};
use once_cell::sync::OnceCell;
use std::sync::{OnceLock, RwLock};

use crate::{
    app::{bindings::ShortcutKeyMod, run_app},
    loader::{CustomIconTheme, Loader, assets::Assets},
    tokio_utils::{AsyncSizedMessage, SizedMessageObj},
    utils::{clipboard::spawn_clipboard_watcher, config::SherlockConfig},
};

mod app;
mod launcher;
mod loader;
mod prelude;
mod tokio_utils;
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
static SOCKET_PATH: &str = "/tmp/sherlock.sock";

#[tokio::main]
async fn main() {
    let socket_path = "/tmp/sherlock.sock";
    if let Ok(mut stream) = UnixStream::connect(socket_path).await {
        let flags = Loader::load_flags();
        if let Ok(flags_bin) = SizedMessageObj::from_struct(&flags) {
            let _ = stream.write_sized(flags_bin).await;
        }
        return;
    }

    spawn_clipboard_watcher();

    // This top part
    let plat = gpui_platform::current_platform(false);
    let app = Application::with_platform(plat)
        .with_assets(Assets)
        .with_quit_mode(QuitMode::Explicit);

    app.run(|cx: &mut App| run_app(cx, Loader::setup()));
}
