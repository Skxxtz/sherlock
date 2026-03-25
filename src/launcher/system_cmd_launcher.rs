use std::sync::Arc;

use serde::de::IntoDeserializer;

use crate::{
    launcher::{LauncherProvider, LauncherType, children::RenderableChild},
    loader::{
        application_loader::parse_priority,
        resolve_icon_path,
        utils::{RawLauncher, deserialize_named_appdata},
    },
    sherlock_error,
    utils::errors::SherlockErrorType,
};

#[derive(Clone, Debug)]
pub struct CommandLauncher {}

impl LauncherProvider for CommandLauncher {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Command(CommandLauncher {})
    }

    fn objects(
        &self,
        launcher: std::sync::Arc<super::Launcher>,
        ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
    ) -> Result<Vec<super::children::RenderableChild>, crate::utils::errors::SherlockError> {
        let cmds = opts.get("commands").ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::FallbackError,
                "Command launcher does not contain any commands.".to_string()
            )
        })?;
        let app_data =
            deserialize_named_appdata(cmds.clone().into_deserializer()).unwrap_or_default();
        let children: Vec<RenderableChild> = app_data
            .into_iter()
            .map(|mut inner| {
                let count = inner
                    .exec
                    .as_deref()
                    .and_then(|exec| ctx.counts.get(exec))
                    .copied()
                    .unwrap_or(0u32);
                inner.icon = inner
                    .icon
                    .and_then(|i| i.to_str().and_then(resolve_icon_path));
                inner.priority = Some(parse_priority(
                    launcher.priority as f32,
                    count,
                    ctx.max_decimals,
                ));
                RenderableChild::AppLike {
                    launcher: Arc::clone(&launcher),
                    inner,
                }
            })
            .collect();

        Ok(children)
    }
}
