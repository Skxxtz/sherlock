use gpui::{App, Entity};
use simd_json::prelude::ArrayTrait;
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Arc};

use crate::{
    launcher::{Launcher, LauncherType, children::RenderableChild},
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
        let (raw_launchers, mut warnings) = parse_launcher_configs(&config.files.fallback)?;

        // Read cached counter file
        let ctx = LoadContext::new()?;

        let submenu = config
            .runtime
            .sub_menu
            .clone()
            .unwrap_or(String::from("all"));
        // Parse the launchers
        let mut launchers: Vec<(Arc<Launcher>, Arc<serde_json::Value>)> = raw_launchers
            .into_iter()
            .filter_map(|raw| {
                // Logic to restrict in submenu mode
                if submenu != "all" && raw.alias.as_ref() != Some(&submenu) {
                    return None;
                }

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
                let launcher = Arc::new(Launcher::from_raw(raw, method, launcher_type, icon));

                Some((launcher, opts))
            })
            .collect();

        launchers.sort_by_key(|(l, _)| l.priority);
        let mut modes = Vec::with_capacity(launchers.len());
        let renders: Vec<RenderableChild> = launchers
            .into_iter()
            .filter_map(|(launcher, opts)| {
                // insert modes
                if let Some((alias, name)) = launcher.alias.as_ref().zip(launcher.name.as_ref()) {
                    modes.push(LauncherMode::Alias {
                        short: alias.into(),
                        name: name.into(),
                    });
                }

                match launcher
                    .launcher_type
                    .get_render_obj(Arc::clone(&launcher), &ctx, opts)
                {
                    Ok(vec) if !vec.is_empty() => Some(vec),
                    Err(e) => {
                        warnings.push(e);
                        None
                    }
                    _ => None,
                }
            })
            .flatten()
            .collect();

        // Get errors and launchers
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

        data_handle.update(cx, |items, cx| {
            *items = Arc::new(renders);
            cx.notify();
        });

        Ok(LauncherLoadResult {
            modes: Arc::from(modes),
            warnings,
        })
    }
}

fn parse_launcher_configs(
    fallback_path: &PathBuf,
) -> Result<(Vec<RawLauncher>, Vec<SherlockError>), SherlockError> {
    // Reads all the configurations of launchers. Either from fallback.json or from default
    // file.

    let mut non_breaking: Vec<SherlockError> = Vec::new();

    fn load_user_fallback(fallback_path: &PathBuf) -> Result<Vec<RawLauncher>, SherlockError> {
        // Tries to load the user-specified launchers. If it failes, it returns a non breaking
        // error.
        match File::open(&fallback_path) {
            Ok(f) => simd_json::from_reader(f).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileParseError(fallback_path.clone()),
                    e.to_string()
                )
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(sherlock_error!(
                SherlockErrorType::FileReadError(fallback_path.clone()),
                e.to_string()
            )),
        }
    }

    let config = match load_user_fallback(fallback_path)
        .map_err(|e| non_breaking.push(e))
        .ok()
    {
        Some(v) => v,
        None => Vec::new(),
    };

    return Ok((config, non_breaking));
}
