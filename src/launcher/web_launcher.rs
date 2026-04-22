use crate::{
    launcher::{LauncherProvider, LauncherType},
    loader::{
        resolve_icon_path,
        utils::{AppData, RawLauncher},
    },
    ui::widgets::RenderableChild,
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
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let mut inner = AppData::new();
        inner.icon = opts
            .get("icon")
            .and_then(Value::as_str)
            .and_then(resolve_icon_path);

        Ok(vec![RenderableChild::App { launcher, inner }])
    }
}
