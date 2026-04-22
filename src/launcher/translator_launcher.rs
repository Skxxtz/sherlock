use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::{
    launcher::{LauncherProvider, LauncherType, LoadContext},
    loader::utils::RawLauncher,
    ui::widgets::{RenderableChild, translator::TranslationData},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Translator {}

impl LauncherProvider for Translator {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Translator(Translator {})
    }
    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        _ctx: &LoadContext,
        _opts: Arc<Value>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        Ok(vec![RenderableChild::Translator {
            launcher,
            inner: TranslationData::new(cx),
        }])
    }
}
