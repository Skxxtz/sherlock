use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::{
    launcher::{LauncherProvider, LauncherType, LoadContext, children::RenderableChild},
    loader::{Loader, utils::RawLauncher},
};

#[derive(Clone, Debug, Deserialize)]
pub struct AppLauncher {
    #[serde(default)]
    pub use_keywords: bool,
}

impl LauncherProvider for AppLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        match serde_json::from_value::<AppLauncher>(raw.args.as_ref().clone()) {
            Ok(launcher) => LauncherType::App(launcher),
            Err(_) => LauncherType::Empty,
        }
    }
    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        ctx: &LoadContext,
        _opts: Arc<Value>,
    ) -> Result<Vec<super::children::RenderableChild>, crate::utils::errors::SherlockError> {
        Loader::load_applications(
            Arc::clone(&launcher),
            &ctx.counts,
            ctx.max_decimals,
            self.use_keywords,
        )
        .map(|ad| {
            ad.into_iter()
                .map(|inner| RenderableChild::AppLike {
                    launcher: Arc::clone(&launcher),
                    inner,
                })
                .collect()
        })
    }
}
