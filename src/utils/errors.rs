use std::fmt::Display;
use std::{fmt::Debug, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::sher_log;

#[macro_export]
macro_rules! sherlock_error {
    ($errtype:expr, $source:expr) => {
        $crate::SherlockError::new($errtype, $source, file!(), line!())
    };
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SherlockError {
    pub error: SherlockErrorType,
    pub traceback: String,
}
impl SherlockError {
    pub fn new<T: AsRef<str> + Display>(
        error: SherlockErrorType,
        source: T,
        file: &str,
        line: u32,
    ) -> Self {
        let _ = sher_log!(format!("{} - {}", error, source), file, line);
        Self {
            error,
            traceback: format!(
                "Location: {}:{}\n─────────────────────────\n{}",
                file,
                line,
                source.as_ref()
            ),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SherlockErrorType {
    // Debug
    DebugError(String),
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

    // Filesystem - Lazy
    IO,

    // Config & Flags
    ConfigError(Option<String>),
    FlagLoadError,
    FallbackError,

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

    // Lockfile
    LockfileExistsError,

    // Networking
    HttpRequestError(String),

    // Sockets
    SocketRemoveError(String),
    SocketConnectError(String),
    SocketWriteError(String),
    SocketReadError(String),
    InvalidMessageLength,

    // Sqlite
    SqlConnectionError(),

    // (De-) Serialization
    SerializationError,
    DeserializationError,

    // Apps
    UnsupportedBrowser(String),

    // Icons
    MissingIconParser(String),

    // Actions
    InvalidAction,

    // Abort
    Abort(String),

    None,
}
impl std::fmt::Display for SherlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (title, message) = self.error.get_message();
        write!(f, "{}\n{}\n{}", title, message, self.traceback)
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
            // Debug
            SherlockErrorType::DebugError(msg) => msg.to_string(),
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

            SherlockErrorType::IO => "Unspecified IO Error".into(),

            // Config & Flags
            SherlockErrorType::ConfigError(val) => {
                if let Some(v) = val {
                    v.into()
                } else {
                    "It should never come to this".into()
                }
            }
            SherlockErrorType::FlagLoadError => "Failed to load flags".into(),
            SherlockErrorType::FallbackError => "Failed to load user fallback".into(),

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

            // Lockfile
            SherlockErrorType::LockfileExistsError => {
                format!("Lockfile already exists in the specified location")
            }

            // Networking
            SherlockErrorType::HttpRequestError(resource) => {
                format!("Failed to get requested source \"{}\"", resource)
            }

            // Sockets
            SherlockErrorType::SocketRemoveError(socket) => socket_msg("close", socket),
            SherlockErrorType::SocketConnectError(socket) => socket_msg("connect", socket),
            SherlockErrorType::SocketWriteError(socket) => socket_msg("send message to", socket),
            SherlockErrorType::SocketReadError(socket) => socket_msg("read message from", socket),
            SherlockErrorType::InvalidMessageLength => {
                format!("The api request exceeds u32::MAX.")
            }

            // Sqlite
            SherlockErrorType::SqlConnectionError() => {
                format!("Failed to estblish database connection.")
            }

            // (De-) Serialization
            SherlockErrorType::SerializationError => {
                format!("Failed to serialize content.")
            }
            SherlockErrorType::DeserializationError => {
                format!("Failed to deserialize content.")
            }

            // Apps
            SherlockErrorType::UnsupportedBrowser(browser) => {
                format!(r#"Unsupported Broser: {}"#, browser)
            }

            // Icon Parsers
            SherlockErrorType::MissingIconParser(parser) => {
                format!(r#"Missing Icon Parser for <i>"{}"</i>"#, parser)
            }

            // Actions
            SherlockErrorType::InvalidAction => {
                format!(r#"Invalid Action Defined"#)
            }

            // Abort
            SherlockErrorType::Abort(what) => {
                format!(r#"Aborted Action "{}""#, what)
            }

            SherlockErrorType::None => String::new(),
        };
        (variant_name(self), message)
    }
}
impl std::fmt::Display for SherlockErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (title, message) = self.get_message();
        write!(f, "Error: {} - {}", title, message)
    }
}
impl AsRef<SherlockError> for SherlockError {
    fn as_ref(&self) -> &SherlockError {
        self
    }
}
