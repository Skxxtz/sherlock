use std::sync::Arc;

use serde::Deserialize;

use crate::{
    launcher::{LauncherProvider, variant_type::LauncherType},
    loader::{
        resolve_icon_path,
        utils::{AppData, RawLauncher},
    },
    ui::widgets::RenderableChild,
};

#[derive(Clone, Debug, Deserialize)]
pub struct MessageLauncher {}
impl LauncherProvider for MessageLauncher {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Message(Self {})
    }
    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        _ctx: &crate::loader::LoadContext,
        _opts: Arc<serde_json::Value>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        Ok(vec![RenderableChild::App {
            launcher: Arc::clone(&launcher),
            inner: AppData::new()
                .with_name("Show Messages".into())
                .with_search_string("messages;errors;warnings;show;")
                .with_icon_opt(resolve_icon_path("sherlock-devtools")),
        }])
    }
}
