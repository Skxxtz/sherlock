use gpui::{App, AppContext, SharedString};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    launcher::{
        LauncherProvider, LauncherType, LoadContext,
        children::{
            RenderableChild,
            script_data::{ScriptData, ScriptDataUpdateEntity},
        },
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    utils::errors::types::SherlockErrorType,
};

#[derive(Clone, Debug, Deserialize)]
pub struct ScriptLauncher {}

impl LauncherProvider for ScriptLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        match serde_json::from_value::<ScriptLauncher>(raw.args.as_ref().clone()) {
            Ok(launcher) => LauncherType::Script(launcher),
            Err(_) => LauncherType::Empty,
        }
    }
    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        _ctx: &LoadContext,
        opts: Arc<Value>,
        cx: &mut App,
    ) -> Result<Vec<super::children::RenderableChild>, crate::utils::errors::SherlockMessage> {
        let exec_command: Option<SharedString> = opts
            .get("exec")
            .and_then(|v| v.as_str())
            .map(|s| SharedString::from(s.to_owned()));

        let args: SharedString = opts
            .get("exec-args")
            .and_then(|v| v.as_str())
            .map(|s| SharedString::from(s.to_owned()))
            .unwrap_or_default();

        let Some(command) = exec_command else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError(format!(
                    "Failed to parse command from launcher configuration of launcher: {launcher}"
                )),
                format!("`exec` key is required. Received arguments: {:?}", opts)
            ));
        };

        Ok(vec![RenderableChild::TextLike {
            launcher,
            inner: ScriptData {
                command,
                args,
                update_entity: cx.new(|_| ScriptDataUpdateEntity::default()),
            },
        }])
    }
}
