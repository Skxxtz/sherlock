use gpui::SharedString;
use serde_json::Value;

use crate::{
    launcher::{
        LauncherProvider, LauncherType,
        children::{RenderableChild, clip_data::ClipData},
    },
    loader::utils::RawLauncher,
    utils::intent::Capabilities,
};

#[derive(Clone, Debug, Default)]
pub struct ClipboardLauncher;
impl LauncherProvider for ClipboardLauncher {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Clipboard(ClipboardLauncher {})
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::Launcher>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
    ) -> Result<Vec<super::children::RenderableChild>, crate::utils::errors::SherlockError> {
        let capabilities: Vec<String> = match opts.get("capabilities") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => vec![String::from("calc.math"), String::from("calc.units")],
        };
        let caps = Capabilities::from_strings(&capabilities);
        let inner = ClipData::new(caps, SharedString::from(""));

        Ok(vec![RenderableChild::ClipLike { launcher, inner }])
    }
}
