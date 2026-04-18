use gpui::Task;

use crate::utils::{
    errors::SherlockMessage,
    intent::{Intent, IntentResult},
};

#[derive(Default)]
pub(super) enum ApiStatus {
    #[default]
    Uninitialized,
    Pending,
    Done {
        res: IntentResult,
    },
    Error {
        msg: SherlockMessage,
    },
}
impl From<Result<IntentResult, SherlockMessage>> for ApiStatus {
    fn from(value: Result<IntentResult, SherlockMessage>) -> Self {
        match value {
            Ok(res) => Self::Done { res },
            Err(msg) => Self::Error { msg },
        }
    }
}

#[derive(Default)]
pub(super) struct TranslationResult {
    pub intent: Option<Intent>,
    pub api: ApiStatus,
    pub task: Option<Task<()>>,
}
