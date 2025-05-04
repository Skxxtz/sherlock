use std::{fmt::Debug, path::PathBuf};

use gtk4::prelude::WidgetExt;

use crate::{g_subclasses::sherlock_row::SherlockRow, ui::tiles::util::TileBuilder};

#[derive(Clone, Debug)]
pub struct SherlockError {
    pub error: SherlockErrorType,
    pub traceback: String,
}
impl SherlockError {
    pub fn tile(&self, tile_type: &str) -> SherlockRow {
        let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/error_tile.ui");

        if let Some((class, icon)) = match tile_type {
            "ERROR" => Some(("error", "🚨")),
            "WARNING" => Some(("warning", "⚠️")),
            _ => None,
        } {
            builder.object.set_css_classes(&["error-tile", class]);
            let (name, message) = self.error.get_message();
            builder
                .title
                .as_ref()
                .and_then(|tmp| tmp.upgrade())
                .map(|title| {
                    title.set_text(format!("{:5}{}:  {}", icon, tile_type, name).as_str())
                });
            builder
                .content_title
                .as_ref()
                .and_then(|tmp| tmp.upgrade())
                .map(|title| title.set_text(&message));
            builder
                .content_body
                .as_ref()
                .and_then(|tmp| tmp.upgrade())
                .map(|body| body.set_text(self.traceback.trim()));
        }
        builder.object
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SherlockErrorType {
    // Environment
    EnvVarNotFoundError(String),

    // Filesystem - Files
    FileExistError(PathBuf),
    FileReadError(PathBuf),
    FileWriteError(PathBuf),
    FileParseError(PathBuf),
    FileRemoveError(PathBuf),

    // Filesystem - Directories
    DirReadError(String),
    DirCreateError(String),
    DirRemoveError(String),

    // Config & Flags
    ConfigError(Option<String>),
    FlagLoadError,

    // Resources
    ResourceParseError,
    ResourceLookupError(String),

    // Display / UI
    DisplayError,
    ClipboardError,

    // Regex / Parsing
    RegexError(String),

    // Commands
    CommandExecutionError(String),

    // DBus
    DBusConnectionError,
    DBusMessageConstructError(String),
    DBusMessageSendError(String),

    // Networking
    HttpRequestError(String),

    // Sockets
    SocketRemoveError(String),
    SocketConnectError(String),
    SocketWriteError(String),

    // Sqlite
    SqlConnectionError(),
}
impl std::fmt::Display for SherlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (title, message) = self.error.get_message();
        write!(f, "SherlockError: {} - {}", title, message)
    }
}
impl SherlockErrorType {
    pub fn get_message(&self) -> (String, String) {
        fn variant_name<T: Debug>(e: &T) -> String {
            let full = format!("{:?}", e);
            full.split('(').next().unwrap_or(&full).into()
        }
        fn path_msg(action: &str, path: &std::path::Path) -> String {
            format!("Failed to {} file \"{}\"", action, path.to_string_lossy())
        }
        fn dir_msg(action: &str, dir: &str) -> String {
            format!("Failed to {} dir at location \"{}\"", action, dir)
        }
        fn resource_msg(action: &str, resource: &str) -> String {
            format!("Failed to {} resource \"{}\"", action, resource)
        }
        fn socket_msg(action: &str, socket: &str) -> String {
            format!("Failed to {} socket at location \"{}\"", action, socket)
        }
        let message = match self {
            // Environment
            SherlockErrorType::EnvVarNotFoundError(var) => {
                format!("Failed to unpack environment variable \"{}\"", var)
            }

            // Filesystem - Files
            SherlockErrorType::FileExistError(f) => path_msg("find", f),
            SherlockErrorType::FileReadError(f) => path_msg("read", f),
            SherlockErrorType::FileWriteError(f) => path_msg("write", f),
            SherlockErrorType::FileParseError(f) => path_msg("parse", f),
            SherlockErrorType::FileRemoveError(f) => path_msg("remove", f),

            // Filesystem - Directories
            SherlockErrorType::DirReadError(dir) => dir_msg("read/access", dir),
            SherlockErrorType::DirCreateError(dir) => dir_msg("create", dir),
            SherlockErrorType::DirRemoveError(dir) => dir_msg("remove", dir),

            // Config & Flags
            SherlockErrorType::ConfigError(val) => {
                if let Some(v) = val {
                    v.into()
                } else {
                    "It should never come to this".into()
                }
            }
            SherlockErrorType::FlagLoadError => "Failed to load flags".into(),

            // Resources
            SherlockErrorType::ResourceParseError => "Failed to parse resources".into(),
            SherlockErrorType::ResourceLookupError(resource) => resource_msg("find", resource),

            // Display / UI
            SherlockErrorType::DisplayError => "Failed to connect to a display.".into(),
            SherlockErrorType::ClipboardError => "Failed to get system clipboard".into(),

            // Regex / Parsing
            SherlockErrorType::RegexError(key) => {
                format!("Failed to compile the regular expression for \"{}\"", key)
            }

            // Commands
            SherlockErrorType::CommandExecutionError(cmd) => {
                format!("Failed to execute command \"{}\"", cmd)
            }

            // DBus
            SherlockErrorType::DBusConnectionError => "Failed to connect to system DBus".into(),
            SherlockErrorType::DBusMessageConstructError(message) => {
                format!("Failed to construct Dbus message \"{}\"", message)
            }
            SherlockErrorType::DBusMessageSendError(message) => {
                format!("Failed to send Dbus message \"{}\"", message)
            }

            // Networking
            SherlockErrorType::HttpRequestError(resource) => {
                format!("Failed to get requested source \"{}\"", resource)
            }

            // Sockets
            SherlockErrorType::SocketRemoveError(socket) => socket_msg("close", socket),
            SherlockErrorType::SocketConnectError(socket) => socket_msg("connect", socket),
            SherlockErrorType::SocketWriteError(socket) => socket_msg("send message to", socket),

            // Sqlite
            SherlockErrorType::SqlConnectionError() => {
                format!("Failed to estblish database connection.")
            }
        };
        (variant_name(self), message)
    }
}
