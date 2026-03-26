use crate::{
    launcher::{LauncherProvider, LauncherType, children::RenderableChild},
    loader::{
        resolve_icon_path,
        utils::{AppData, RawLauncher},
    },
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
pub struct WebLauncher {
    #[serde(rename = "search_engine")]
    pub engine: String,
    pub browser: Option<String>,
}

impl LauncherProvider for WebLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        match serde_json::from_value::<WebLauncher>(raw.args.as_ref().clone()) {
            Ok(launcher) => LauncherType::Web(launcher),
            Err(_) => LauncherType::Empty,
        }
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::Launcher>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
    ) -> Result<Vec<super::children::RenderableChild>, crate::utils::errors::SherlockMessage> {
        let mut inner = AppData::new();
        inner.icon = opts
            .get("icon")
            .and_then(Value::as_str)
            .and_then(|i| resolve_icon_path(i));

        Ok(vec![RenderableChild::AppLike { launcher, inner }])
    }
}
