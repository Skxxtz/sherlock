use gpui::{App, Entity};
use std::{collections::HashMap, io::ErrorKind, path::PathBuf, sync::Arc};

use crate::{
    launcher::{Launcher, variant_type::LauncherType},
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::{launcher::LauncherMode, widgets::RenderableChild},
    utils::{
        cache::BinaryCache,
        config::ConfigGuard,
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
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
    fn new() -> Result<Self, SherlockMessage> {
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
    pub messages: Vec<SherlockMessage>,
}
impl Loader {
    pub fn load_launchers(
        cx: &mut App,
        data_handle: Entity<Arc<Vec<RenderableChild>>>,
    ) -> Result<LauncherLoadResult, SherlockMessage> {
        // read config
        let config = ConfigGuard::read()?;

        // Read fallback data here:
        let (raw_launchers, mut messages) = parse_launcher_configs(&config.files.fallback);

        // Read cached counter file
        let ctx = LoadContext::new()?;

        // Parse the launchers
        let mut launchers: Vec<(Arc<Launcher>, Arc<serde_json::Value>)> = raw_launchers
            .into_iter()
            .map(|raw| {
                let launcher_type: LauncherType = raw.r#type.into_launcher_type(&raw);

                let icon = raw
                    .args
                    .get("icon")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());

                let opts = Arc::clone(&raw.args);

                (Arc::new(Launcher::from_raw(raw, launcher_type, icon)), opts)
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
                    .get_render_obj(Arc::clone(&launcher), &ctx, opts, cx)
                {
                    Ok(vec) => (!vec.is_empty()).then_some(vec),
                    Err(e) => {
                        messages.push(e);
                        None
                    }
                }
            })
            .flatten()
            .collect();

        Self::sync_cache_if_empty(&ctx, &renders, &mut messages);

        data_handle.update(cx, |items, cx| {
            *items = Arc::new(renders);
            cx.notify();
        });

        Ok(LauncherLoadResult {
            modes: Arc::from(modes),
            messages,
        })
    }

    fn sync_cache_if_empty(
        ctx: &LoadContext,
        renders: &[RenderableChild],
        warnings: &mut Vec<SherlockMessage>,
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

/// Incrementally parses launchers from the `fallback.json` file.
///
/// Each launcher is deserialized individually. If an entry is invalid—for instance,
/// due to an unknown `LauncherVariant`—a warning is appended to the
/// returned list and the specific launcher is skipped, allowing the rest
/// of the configuration to load.
///
/// # Returns
/// A tuple containing the successfully parsed `Vec<RawLauncher>` and
/// a `Vec<SherlockError>` containing any collected warnings.
fn parse_launcher_configs(path: &PathBuf) -> (Vec<RawLauncher>, Vec<SherlockMessage>) {
    let mut warnings = Vec::new();
    let mut launchers = Vec::new();

    let raw_bytes: Vec<u8> = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            include_bytes!("../../assets/fallback.json").to_vec()
        }
        Err(e) => {
            warnings.push(sherlock_msg!(
                Error,
                SherlockErrorType::FileError(FileAction::Read, path.clone()),
                e
            ));
            return (launchers, warnings);
        }
    };

    let mut buffer = raw_bytes;
    let raw_values: Vec<serde_json::Value> = match simd_json::from_slice(&mut buffer) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(sherlock_msg!(
                Warning,
                SherlockErrorType::DeserializationError,
                e
            ));
            return (launchers, warnings);
        }
    };

    for value in raw_values.into_iter() {
        match serde_json::from_value::<RawLauncher>(value) {
            Ok(launcher) => launchers.push(launcher),
            Err(e) => {
                warnings.push(sherlock_msg!(
                    Warning,
                    SherlockErrorType::ConfigError("Invalid launcher configuration".into()),
                    e
                ));
            }
        }
    }

    (launchers, warnings)
}
