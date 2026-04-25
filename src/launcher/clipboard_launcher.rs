use gpui::SharedString;
use serde_json::Value;

use crate::{
    launcher::{LauncherProvider, LauncherType},
    loader::utils::RawLauncher,
    ui::widgets::{RenderableChild, clipboard::ClipData},
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
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let capabilities: Vec<String> = match opts.get("capabilities") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => vec![String::from("calc.math"), String::from("calc.units")],
        };
        let caps = Capabilities::from_strings(&capabilities);
        let inner = ClipData::new(caps, SharedString::from(""));

        Ok(vec![RenderableChild::Clip { launcher, inner }])
    }
}
