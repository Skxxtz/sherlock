use gpui::{App, Entity};
use simd_json::prelude::ArrayTrait;
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Arc};

use crate::{
    launcher::{
        Launcher, LauncherType,
        app_launcher::AppLauncher,
        audio_launcher::MusicPlayerLauncher,
        bookmark_launcher::BookmarkLauncher,
        calc_launcher::{CURRENCIES, CalculatorLauncher, Currency},
        category_launcher::CategoryLauncher,
        children::RenderableChild,
        clipboard_launcher::ClipboardLauncher,
        system_cmd_launcher::CommandLauncher,
        weather_launcher::WeatherLauncher,
        web_launcher::WebLauncher,
    },
    loader::utils::{LauncherVariant, RawLauncher},
    sherlock_error,
    ui::launcher::LauncherMode,
    utils::{
        cache::BinaryCache,
        config::{ConfigGuard, ConstantDefaults},
        errors::{SherlockError, SherlockErrorType},
    },
};

use super::Loader;
use super::utils::CounterReader;

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
        let (raw_launchers, warnings) = parse_launcher_configs(&config.files.fallback)?;

        // Read cached counter file
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

                launcher.launcher_type.get_render_obj(
                    Arc::clone(&launcher),
                    opts,
                    &counts,
                    max_decimals,
                )
            })
            .flatten()
            .collect();

        // Get errors and launchers
        let mut non_breaking = Vec::new();
        if counts.is_empty() {
            let counts: HashMap<String, u32> = renders
                .iter()
                .filter_map(|render| render.get_exec())
                .map(|exec| (exec, 0))
                .collect();
            if let Err(e) = BinaryCache::write(&counter_reader.path, &counts) {
                non_breaking.push(e)
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

fn parse_app_launcher(raw: &RawLauncher) -> LauncherType {
    match serde_json::from_value::<AppLauncher>(raw.args.as_ref().clone()) {
        Ok(launcher) => LauncherType::App(launcher),
        Err(_) => LauncherType::Empty,
    }
}
fn parse_audio_sink_launcher() -> LauncherType {
    LauncherType::MusicPlayer(MusicPlayerLauncher {})
}
fn parse_bookmarks_launcher(launcher: &RawLauncher) -> LauncherType {
    let browser_target = launcher
        .args
        .get("browser")
        .and_then(|s| s.as_str().map(|str| str.to_string()))
        .or_else(|| {
            ConfigGuard::read()
                .ok()
                .and_then(|c| c.default_apps.browser.clone())
        })
        .or_else(|| ConstantDefaults::browser().ok());

    // TODO parse bookmarks later
    if let Some(browser) = browser_target {
        return LauncherType::Bookmark(BookmarkLauncher {
            target_browser: browser,
        });
    }
    LauncherType::Empty
}
fn parse_calculator(raw: &RawLauncher) -> LauncherType {
    // initialize currencies
    let update_interval = raw
        .args
        .get("currency_update_interval")
        .and_then(|interval| interval.as_u64())
        .unwrap_or(60 * 60 * 24);

    tokio::spawn(async move {
        let result = Currency::get_exchange(update_interval).await.ok();
        let _result = CURRENCIES.set(result);
    });

    LauncherType::Calc(CalculatorLauncher {})
}
fn parse_category_launcher() -> LauncherType {
    LauncherType::Category(CategoryLauncher {})
}

fn parse_clipboard_launcher() -> LauncherType {
    LauncherType::Clipboard(ClipboardLauncher {})
}

fn parse_command_launcher() -> LauncherType {
    LauncherType::Command(CommandLauncher {})
}

fn parse_debug_launcher() -> LauncherType {
    LauncherType::Command(CommandLauncher {})
}
fn parse_weather_launcher(raw: &RawLauncher) -> LauncherType {
    match serde_json::from_value::<WeatherLauncher>(raw.args.as_ref().clone()) {
        Ok(launcher) => LauncherType::Weather(launcher),
        Err(_) => LauncherType::Empty,
    }
}

fn parse_web_launcher(raw: &RawLauncher) -> LauncherType {
    match serde_json::from_value::<WebLauncher>(raw.args.as_ref().clone()) {
        Ok(launcher) => LauncherType::Web(launcher),
        Err(_) => LauncherType::Empty,
    }
}

impl LauncherVariant {
    fn into_launcher_type(self, raw: &RawLauncher) -> LauncherType {
        match self {
            Self::AppLauncher => parse_app_launcher(raw),
            Self::AudioSink => parse_audio_sink_launcher(),
            Self::Bookmarks => parse_bookmarks_launcher(raw),
            Self::Calculator => parse_calculator(raw),
            Self::Category => parse_category_launcher(),
            Self::Clipboard => parse_clipboard_launcher(),
            Self::Command => parse_command_launcher(),
            Self::Debug => parse_debug_launcher(),
            Self::Weather => parse_weather_launcher(raw),
            Self::WebLauncher => parse_web_launcher(raw),
            Self::None => LauncherType::Empty,
            _ => LauncherType::Empty,
            // "bulk_text" => parse_bulk_text_launcher(&raw),
            // "emoji_picker" => parse_emoji_launcher(&raw),
            // "files" => parse_file_launcher(&raw),
            // "teams_event" => parse_event_launcher(&raw),
            // "theme_picker" => parse_theme_launcher(&raw),
            // "process" => parse_process_launcher(&raw),
            // "pomodoro" => parse_pomodoro(&raw),
        }
    }
}
