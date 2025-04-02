use cli_clipboard::{ClipboardContext, ClipboardProvider};

use crate::loader::util::{SherlockError, SherlockErrorType};

pub fn copy_to_clipboard(string: &str) -> Result<(), SherlockError> {
    let mut ctx = ClipboardContext::new().map_err(|e| SherlockError {
        error: SherlockErrorType::ClipboardError,
        traceback: e.to_string(),
    })?;

    let _ = ctx.set_contents(string.to_string());
    Ok(())
}
pub fn read_from_clipboard() -> Result<String, SherlockError> {
    let mut ctx = ClipboardContext::new().map_err(|e| SherlockError {
        error: SherlockErrorType::ClipboardError,
        traceback: e.to_string(),
    })?;
    Ok(ctx.get_contents().unwrap_or_default().trim().to_string())
}
