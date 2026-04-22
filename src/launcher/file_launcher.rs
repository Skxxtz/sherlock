use std::sync::Arc;

use gpui::SharedString;
use serde::Deserialize;

use crate::{
    launcher::{Launcher, LauncherProvider, variant_type::LauncherType},
    loader::{
        resolve_icon_path,
        utils::{AppData, RawLauncher},
    },
    ui::{model::file::FileSearchBackend, widgets::RenderableChild},
    utils::errors::SherlockMessage,
};

#[derive(Clone, Debug, Deserialize)]
pub struct FileLauncher {
    pub max_results: usize,
    pub poll_interval: u64,
    pub backend: FileSearchBackend,
}

impl LauncherProvider for FileLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        let backend = raw
            .args
            .get("backend")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let poll_interval = raw
            .args
            .get("poll_interval")
            .and_then(|v| v.as_u64())
            .unwrap_or(50);

        let max_results = raw
            .args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50);

        LauncherType::Files(Self {
            backend,
            poll_interval,
            max_results,
        })
    }

    fn objects(
        &self,
        launcher: Arc<Launcher>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let mut inner = AppData::new();
        inner.name = launcher.name.as_ref().map(SharedString::from);
        inner.search_string = "file;file search".into();
        inner.icon = resolve_icon_path("folder");

        let child = RenderableChild::App { launcher, inner };

        Ok(vec![child])
    }
}
