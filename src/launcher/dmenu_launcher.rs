use std::sync::Arc;

use crate::{
    launcher::{Launcher, LauncherProvider, LauncherType},
    ui::widgets::RenderableChild,
    utils::errors::SherlockMessage,
};

#[derive(Clone, Debug, Default)]
pub struct DmenuLauncher {}

impl LauncherProvider for DmenuLauncher {
    fn parse(_raw: &crate::loader::utils::RawLauncher) -> super::LauncherType {
        LauncherType::Dmenu(Self {})
    }

    fn objects(
        &self,
        _launcher: Arc<Launcher>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        // Should never be called! This is only from piped input.
        unimplemented!()
    }
}
