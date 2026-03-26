use std::{fmt::Display, path::PathBuf};

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display as StrumDisplay};

#[derive(Debug, Clone, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "snake_case")]
pub enum SherlockErrorType {
    /// Logical branch that code logic dictates is impossible to reach.
    Unreachable,
    /// General-purpose Input/Output failures from the standard library.
    IO,
    /// Failure while converting internal data structures to a string/byte format.
    SerializationError,
    /// Failure while parsing external data (JSON/YAML) into internal structures.
    DeserializationError,
    /// The requested UI or system action is not recognized or defined.
    InvalidAction,
    /// The specific internal function is not supported by the active launcher.
    InvalidFunction,
    /// A concurrency error where a resource is already mutably borrowed.
    BorrowCongestion,
    /// A placeholder for "no error" (null object pattern).
    None,

    // --- Variants with Data ---
    /// File-specific issues (read/write/permissions) at the given path.
    FileError(FileAction, PathBuf),
    /// Directory-specific issues (creation/deletion/access) at the given path.
    DirError(DirAction, PathBuf),
    /// Errors in logic or syntax within the user's configuration file.
    ConfigError(String),
    /// Failure when executing external shell commands or system binaries.
    CommandError(String),
    /// Missing or malformed system environment variables (e.g., $PATH, $HOME).
    EnvError(String),
    /// Communication failures with the system or session DBus.
    DBusError(DBusAction, String),
    /// Low-level IPC or network socket connection/read/write failures.
    SocketError(SocketAction, String),
    /// Remote request failures (HTTP, timeouts, or DNS issues).
    NetworkError(NetworkAction, String),
    /// SQL or connection failures involving the local SQLite database.
    DatabaseError(DbAction),
    /// Error triggered when a requested feature requires an unsupported web browser.
    UnsupportedBrowser(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay)]
#[strum(serialize_all = "lowercase")]
pub enum FileAction {
    Read,
    Write,
    Parse,
    Remove,
    Find,
    Create,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay)]
#[strum(serialize_all = "lowercase")]
pub enum DirAction {
    Read,
    Create,
    Remove,
    Find,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay)]
#[strum(serialize_all = "lowercase")]
pub enum DBusAction {
    Connect,
    Construct,
    Send,
    Call,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay)]
#[strum(serialize_all = "lowercase")]
pub enum SocketAction {
    Close,
    Connect,
    Write,
    Read,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay)]
#[strum(serialize_all = "lowercase")]
pub enum NetworkAction {
    Get,
    Post,
    Put,
    Update,
    Delete,
}
#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay)]
#[strum(serialize_all = "lowercase")]
pub enum DbAction {
    Connect,
    Query,
    Initialize,
    Migrate,
    Transaction,
}

impl Display for SherlockErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable => write!(f, "It should never come to this"),
            Self::EnvError(var) => write!(f, "Failed to read \"{var}\""),
            Self::FileError(act, path) => {
                write!(f, "Failed to {} file \"{}\"", act, path.display())
            }
            Self::DirError(act, path) => {
                write!(f, "Failed to {} directory \"{}\"", act, path.display())
            }
            Self::ConfigError(msg) => write!(f, "Config error: {}", msg),
            Self::DBusError(act, target) => {
                write!(f, "DBus failed to {act} ({target})")
            }
            Self::SocketError(act, loc) => write!(f, "Failed to {} socket at \"{}\"", act, loc),
            Self::BorrowCongestion => write!(f, "Resource is currently locked/in use"),
            Self::NetworkError(act, loc) => {
                write!(f, "Network failure during {act} from \"{loc}\"")
            }
            Self::DatabaseError(act) => {
                write!(f, "Database failure during operation: {act}")
            }
            // Fallback for simple variants
            _ => write!(f, "{}", self.as_ref()),
        }
    }
}
