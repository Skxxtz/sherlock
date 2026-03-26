use gpui::{App, Entity};
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Arc};

use crate::{
    launcher::{Launcher, children::RenderableChild, variant_type::LauncherType},
    loader::utils::RawLauncher,
    sherlock_error,
    ui::launcher::LauncherMode,
    utils::{
        cache::BinaryCache,
        config::ConfigGuard,
        errors::{SherlockError, SherlockErrorType},
    },
};

use super::Loader;
use super::utils::CounterReader;

pub struct LoadContext {
    pub counts: HashMap<String, u32>,
    pub max_decimals: i32,
    pub path: PathBuf,
}
impl LoadContext {
    fn new() -> Result<Self, SherlockError> {
        let counter_reader = CounterReader::new()?;
        let counts: HashMap<String, u32> =
            BinaryCache::read(&counter_reader.path).unwrap_or_default();

        // Construct max decimal count
        let max_count = counts.values().max().cloned().unwrap_or(0);
        let max_decimals = if max_count == 0 {
            0
        } else {
            (max_count as f32).log10().floor() as i32 + 1
        };

        Ok(Self {
            counts,
            max_decimals,
            path: counter_reader.path,
        })
    }
}

pub struct LauncherLoadResult {
    pub modes: Arc<[LauncherMode]>,
    pub warnings: Vec<SherlockError>,
}
impl Loader {
    pub fn load_launchers(
        cx: &mut App,
        data_handle: Entity<Arc<Vec<RenderableChild>>>,
    ) -> Result<LauncherLoadResult, SherlockError> {
        // read config
        let config = ConfigGuard::read()?;

        // Read fallback data here:
        let (raw_launchers, mut warnings) = parse_launcher_configs(&config.files.fallback);

        // Read cached counter file
        let ctx = LoadContext::new()?;

        // Parse the launchers
        let mut launchers: Vec<(Arc<Launcher>, Arc<serde_json::Value>)> = raw_launchers
            .into_iter()
            .map(|raw| {
                let method = raw
                    .on_return
                    .clone()
                    .unwrap_or_else(|| raw.r#type.to_string());

                let launcher_type: LauncherType = raw.r#type.into_launcher_type(&raw);

                let icon = raw
                    .args
                    .get("icon")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());

                let opts = Arc::clone(&raw.args);

                (
                    Arc::new(Launcher::from_raw(raw, method, launcher_type, icon)),
                    opts,
                )
            })
            .collect();

        launchers.sort_by_key(|(l, _)| l.priority);

        let mut modes = Vec::with_capacity(launchers.len());
        let renders: Vec<RenderableChild> = launchers
            .into_iter()
            .inspect(|(launcher, _)| {
                // Collect modes
                if let Some((alias, name)) = launcher.alias.as_ref().zip(launcher.name.as_ref()) {
                    modes.push(LauncherMode::Alias {
                        short: alias.into(),
                        name: name.into(),
                    });
                }
            })
            .filter_map(|(launcher, opts)| {
                match launcher
                    .launcher_type
                    .get_render_obj(Arc::clone(&launcher), &ctx, opts)
                {
                    Ok(vec) => (!vec.is_empty()).then_some(vec),
                    Err(e) => {
                        warnings.push(e);
                        None
                    }
                }
            })
            .flatten()
            .collect();

        Self::sync_cache_if_empty(&ctx, &renders, &mut warnings);

        data_handle.update(cx, |items, cx| {
            *items = Arc::new(renders);
            cx.notify();
        });

        Ok(LauncherLoadResult {
            modes: Arc::from(modes),
            warnings,
        })
    }

    fn sync_cache_if_empty(
        ctx: &LoadContext,
        renders: &[RenderableChild],
        warnings: &mut Vec<SherlockError>,
    ) {
        if ctx.counts.is_empty() {
            let counts: HashMap<String, u32> = renders
                .iter()
                .filter_map(|render| render.get_exec())
                .map(|exec| (exec, 0))
                .collect();
            if let Err(e) = BinaryCache::write(&ctx.path, &counts) {
                warnings.push(e)
            };
        }
    }
}

fn parse_launcher_configs(path: &PathBuf) -> (Vec<RawLauncher>, Vec<SherlockError>) {
    let mut warnings = Vec::new();
    let config = File::open(&path)
        .map_err(|e| {
            if e.kind() != std::io::ErrorKind::NotFound {
                warnings.push(sherlock_error!(
                    SherlockErrorType::FileReadError(path.clone()),
                    e.to_string()
                ));
            }
        })
        .ok()
        .and_then(|f| {
            simd_json::from_reader(f)
                .map_err(|e| {
                    warnings.push(sherlock_error!(
                        SherlockErrorType::FileParseError(path.clone()),
                        e.to_string()
                    ));
                })
                .ok()
        })
        .unwrap_or_default();

    (config, warnings)
}
