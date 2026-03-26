use gpui::SharedString;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

use crate::utils::errors::types::SherlockErrorType;

pub mod types;

/// Dispatches a structured message to the Sherlock logging and error system.
///
/// This macro captures the current file and line number automatically and
/// constructs a [`SherlockMessage`]. It immediately triggers a call to
/// `sher_log!` before returning the message object.
///
/// # Arguments
///
/// * `$level` - The [`SherlockMessageLevel`] variant (e.g., `Info`, `Warning`, `Error`).
///   Note: This is passed as an identifier, not a string.
/// * `$errtype` - A variant of [`SherlockErrorType`] describing the category of the event.
/// * `$source` - Any type implementing [`Display`](std::fmt::Display) that provides
///   the specific context or "cause" of the message.
///
/// # Examples
///
/// ```
/// // Log a non-critical configuration issue
/// let msg = sherlock_msg!(Warning, SherlockErrorType::ConfigError, "Key 'theme' missing");
///
/// // Handle a critical IO failure
/// let path = std::path::PathBuf::from("config.json");
/// let err = sherlock_msg!(Error, SherlockErrorType::FileError(FileAction::Read, path), "Permission denied");
/// ```
///
/// # Side Effects
///
/// Calling this macro executes the `sher_log!` macro internally, which usually
/// performs a write operation to the system log file.
#[macro_export]
macro_rules! sherlock_msg {
    ($level:ident, $errtype:expr, $source:expr) => {
        $crate::utils::errors::SherlockMessage::new(
            $crate::utils::errors::SherlockMessageLevel::$level,
            $errtype,
            $source,
            file!(),
            line!(),
        )
    };
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum SherlockMessageLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SherlockMessage {
    pub error_type: SherlockErrorType,
    pub level: SherlockMessageLevel,
    pub traceback: SharedString,
}

impl SherlockMessage {
    pub fn new<T: Display>(
        level: SherlockMessageLevel,
        error_type: SherlockErrorType,
        source: T,
        file: &str,
        line: u32,
    ) -> Self {
        let msg = format!("[{:?}] {} - {}", level, error_type.as_ref(), source);
        let _ = crate::sher_log!(msg, file, line);

        Self {
            error_type,
            level,
            traceback: format!("Location: {file}:{line}\n─────────────────────────\n{source}")
                .into(),
        }
    }
}
