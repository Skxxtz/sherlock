use gpui::{App, AppContext, SharedString};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    ensure_func,
    launcher::{LauncherProvider, LauncherType, LoadContext, variant_type::InnerFunction},
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::widgets::{
        RenderableChild,
        script::{ScriptData, ScriptDataUpdateEntity},
    },
    utils::errors::{SherlockMessage, types::SherlockErrorType},
};

#[derive(Debug, Clone, Copy, PartialEq, strum::VariantNames, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum ScriptFunctions {
    Run,
}

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
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
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

        Ok(vec![RenderableChild::ScriptLike {
            launcher,
            inner: ScriptData {
                command,
                args,
                update_entity: cx.new(|_| ScriptDataUpdateEntity::default()),
            },
        }])
    }
    fn execute_function<C: AppContext>(
        &self,
        func: super::variant_type::InnerFunction,
        child: &RenderableChild,
        cx: &mut C,
    ) -> Result<bool, SherlockMessage> {
        let func = ensure_func!(func, InnerFunction::Script);
        match func {
            ScriptFunctions::Run => {
                if let RenderableChild::ScriptLike { inner, .. } = child {
                    inner.update_async(cx);
                }
            }
        }
        Ok(false)
    }
}
