use serde::de::IntoDeserializer;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use std::env::home_dir;
use std::fs::File;
use std::path::PathBuf;

use crate::actions::util::{parse_default_browser, read_from_clipboard};
use crate::launcher::audio_launcher::AudioLauncherFunctions;
use crate::launcher::bookmark_launcher::BookmarkLauncher;
use crate::launcher::calc_launcher::{CalculatorLauncher, Currency, CURRENCIES};
use crate::launcher::category_launcher::CategoryLauncher;
use crate::launcher::emoji_picker::EmojiPicker;
use crate::launcher::event_launcher::EventLauncher;
use crate::launcher::file_launcher::FileLauncher;
use crate::launcher::process_launcher::ProcessLauncher;
use crate::launcher::theme_picker::ThemePicker;
use crate::launcher::weather_launcher::WeatherLauncher;
use crate::launcher::{
    app_launcher, bulk_text_launcher, clipboard_launcher, system_cmd_launcher, web_launcher,
    Launcher, LauncherType,
};
use crate::loader::util::{CounterReader, JsonCache};
use crate::utils::errors::SherlockError;
use crate::utils::errors::SherlockErrorType;

use app_launcher::AppLauncher;
use bulk_text_launcher::BulkTextLauncher;
use clipboard_launcher::ClipboardLauncher;
use simd_json;
use simd_json::prelude::ArrayTrait;
use system_cmd_launcher::CommandLauncher;
use web_launcher::WebLauncher;

use super::application_loader::parse_priority;
use super::util::deserialize_named_appdata;
use super::util::AppData;
use super::util::RawLauncher;
use super::Loader;
use crate::{sherlock_error, CONFIG};

impl Loader {
    #[sherlock_macro::timing("Loading launchers")]
    pub fn load_launchers() -> Result<(Vec<Launcher>, Vec<SherlockError>), SherlockError> {
        let config = CONFIG
            .get()
            .ok_or_else(|| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;

        // Read fallback data here:
        let (raw_launchers, n) = parse_launcher_configs(&config.files.fallback)?;

        // Read cached counter file
        let counter_reader = CounterReader::new()?;
        let counts: HashMap<String, f32> =
            JsonCache::read(&counter_reader.path).unwrap_or_default();
        let max_decimals = counts
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, v)| v.to_string().len())
            .unwrap_or(0) as i32;

        // Parse the launchers
        let deserialized_launchers: Vec<Result<Launcher, SherlockError>> = raw_launchers
            .into_iter()
            .map(|raw| {
                let launcher_type: LauncherType = match raw.r#type.as_str() {
                    "app_launcher" => parse_app_launcher(&raw, &counts, max_decimals),
                    "audio_sink" => parse_audio_sink_launcher(),
                    "bookmarks" => parse_bookmarks_launcher(&raw),
                    "bulk_text" => parse_bulk_text_launcher(&raw),
                    "calculation" => parse_calculator(&raw),
                    "categories" => parse_category_launcher(&raw, &counts, max_decimals),
                    "clipboard-execution" => parse_clipboard_launcher(&raw)?,
                    "command" => parse_command_launcher(&raw, &counts, max_decimals),
                    "debug" => parse_debug_launcher(&raw, &counts, max_decimals),
                    "emoji_picker" => parse_emoji_launcher(&raw),
                    "files" => parse_file_launcher(&raw),
                    "teams_event" => parse_event_launcher(&raw),
                    "theme_picker" => parse_theme_launcher(&raw),
                    "process" => parse_process_launcher(&raw),
                    "weather" => parse_weather_launcher(&raw),
                    "web_launcher" => parse_web_launcher(&raw),
                    _ => LauncherType::Empty,
                };
                let method: String = if let Some(value) = &raw.on_return {
                    value.to_string()
                } else {
                    raw.r#type.clone()
                };
                let icon = raw
                    .args
                    .get("icon")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                Ok(Launcher::from_raw(raw, method, launcher_type, icon))
            })
            .collect();

        // Get errors and launchers
        type LauncherResult = Vec<Result<Launcher, SherlockError>>;
        let (oks, errs): (LauncherResult, LauncherResult) =
            deserialized_launchers.into_iter().partition(Result::is_ok);
        let launchers: Vec<Launcher> = oks.into_iter().filter_map(Result::ok).collect();
        let mut non_breaking: Vec<SherlockError> =
            errs.into_iter().filter_map(Result::err).collect();
        if counts.is_empty() {
            let counts: HashMap<String, u32> = launchers
                .iter()
                .filter_map(|launcher| launcher.get_execs())
                .flat_map(|exec_set| exec_set.into_iter().map(|exec| (exec, 0)))
                .collect();
            if let Err(e) = JsonCache::write(&counter_reader.path, &counts) {
                non_breaking.push(e)
            };
        }
        non_breaking.extend(n);
        Ok((launchers, non_breaking))
    }
}
fn parse_appdata(
    value: &Value,
    prio: f32,
    counts: &HashMap<String, f32>,
    max_decimals: i32,
) -> HashSet<AppData> {
    let data: HashSet<AppData> =
        deserialize_named_appdata(value.clone().into_deserializer()).unwrap_or_default();
    data.into_iter()
        .map(|c| {
            let count = c
                .exec
                .as_ref()
                .and_then(|exec| counts.get(exec))
                .unwrap_or(&0.0);
            c.with_priority(parse_priority(prio, *count, max_decimals))
        })
        .collect::<HashSet<AppData>>()
}
fn parse_app_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, f32>,
    max_decimals: i32,
) -> LauncherType {
    let apps: HashSet<AppData> = CONFIG.get().map_or_else(
        || HashSet::new(),
        |config| {
            let prio = raw.priority;
            match config.behavior.caching {
                true => Loader::load_applications(prio, counts, max_decimals).unwrap_or_default(),
                false => Loader::load_applications_from_disk(None, prio, counts, max_decimals)
                    .unwrap_or_default(),
            }
        },
    );
    LauncherType::App(AppLauncher { apps })
}
fn parse_audio_sink_launcher() -> LauncherType {
    AudioLauncherFunctions::new()
        .and_then(|launcher| {
            launcher.get_current_player().and_then(|player| {
                launcher
                    .get_metadata(&player)
                    .and_then(|launcher| Some(LauncherType::MusicPlayer(launcher)))
            })
        })
        .unwrap_or(LauncherType::Empty)
}
fn parse_bookmarks_launcher(raw: &RawLauncher) -> LauncherType {
    if let Some(browser) = CONFIG
        .get()
        .and_then(|c| c.default_apps.browser.clone())
        .or_else(|| parse_default_browser().ok())
    {
        match BookmarkLauncher::find_bookmarks(&browser, raw) {
            Ok(bookmarks) => {
                return LauncherType::Bookmark(BookmarkLauncher { bookmarks });
            }
            Err(err) => {
                let _result = err.insert();
            }
        }
    }
    LauncherType::Empty
}
fn parse_bulk_text_launcher(raw: &RawLauncher) -> LauncherType {
    LauncherType::BulkText(BulkTextLauncher {
        icon: raw
            .args
            .get("icon")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        exec: raw
            .args
            .get("exec")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        args: raw
            .args
            .get("exec-args")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}
fn parse_calculator(raw: &RawLauncher) -> LauncherType {
    let capabilities: HashSet<String> = match raw.args.get("capabilities") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => HashSet::from([String::from("calc.math"), String::from("calc.units")]),
    };

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

    LauncherType::Calc(CalculatorLauncher { capabilities })
}
fn parse_category_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, f32>,
    max_decimals: i32,
) -> LauncherType {
    let prio = raw.priority;
    let value = &raw.args["categories"];
    let categories = parse_appdata(value, prio, counts, max_decimals);
    LauncherType::Category(CategoryLauncher { categories })
}
fn parse_clipboard_launcher(raw: &RawLauncher) -> Result<LauncherType, SherlockError> {
    let clipboard_content: String = read_from_clipboard()?;
    let capabilities: Option<HashSet<String>> = match raw.args.get("capabilities") {
        Some(Value::Array(arr)) => {
            let strings: HashSet<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Some(strings)
        }
        _ => None,
    };
    if clipboard_content.is_empty() {
        Ok(LauncherType::Empty)
    } else {
        if capabilities.is_none() {
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
        }
        Ok(LauncherType::Clipboard((
            ClipboardLauncher {
                clipboard_content,
                capabilities: capabilities.clone(),
            },
            CalculatorLauncher {
                capabilities: capabilities.unwrap_or(HashSet::from([
                    String::from("calc.math"),
                    String::from("calc.units"),
                ])),
            },
        )))
    }
}
fn parse_command_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, f32>,
    max_decimals: i32,
) -> LauncherType {
    let prio = raw.priority;
    let value = &raw.args["commands"];
    let commands = parse_appdata(value, prio, counts, max_decimals);
    LauncherType::Command(CommandLauncher { commands })
}
fn parse_debug_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, f32>,
    max_decimals: i32,
) -> LauncherType {
    let prio = raw.priority;
    let value = &raw.args["commands"];
    let commands = parse_appdata(value, prio, counts, max_decimals);
    LauncherType::Command(CommandLauncher { commands })
}
fn parse_emoji_launcher(raw: &RawLauncher) -> LauncherType {
    let mut data: HashSet<AppData> = HashSet::with_capacity(1);
    let mut app_data = AppData::from_raw_launcher(raw);
    if app_data.icon.is_none() {
        app_data.icon = Some(String::from("sherlock-emoji"))
    }
    data.insert(app_data);
    LauncherType::Emoji(EmojiPicker {
        rows: 4,
        cols: 5,
        data,
    })
}
fn parse_event_launcher(raw: &RawLauncher) -> LauncherType {
    let icon = raw
        .args
        .get("icon")
        .and_then(Value::as_str)
        .unwrap_or("teams")
        .to_string();
    let date = raw
        .args
        .get("event_date")
        .and_then(Value::as_str)
        .unwrap_or("now");
    let event_start = raw
        .args
        .get("event_start")
        .and_then(Value::as_str)
        .unwrap_or("-5 minutes");
    let event_end = raw
        .args
        .get("event_end")
        .and_then(Value::as_str)
        .unwrap_or("+15 minutes");
    let event = EventLauncher::get_event(date, event_start, event_end);
    LauncherType::Event(EventLauncher { event, icon })
}
fn parse_theme_launcher(raw: &RawLauncher) -> LauncherType {
    let relative = raw
        .args
        .get("location")
        .and_then(Value::as_str)
        .unwrap_or("~/.config/sherlock/themes/");
    let relative = relative.strip_prefix("~/").unwrap_or(relative);
    let home = match home_dir() {
        Some(dir) => dir,
        _ => return LauncherType::Empty,
    };
    let absolute = home.join(relative);
    ThemePicker::new(absolute, raw.priority)
}
fn parse_file_launcher(raw: &RawLauncher) -> LauncherType {
    let mut data: HashSet<AppData> = HashSet::with_capacity(1);
    let mut app_data = AppData::from_raw_launcher(raw);
    if app_data.icon.is_none() {
        app_data.icon = Some(String::from("files"))
    }
    data.insert(app_data);
    let value = &raw.args["dirs"];
    match value.as_array() {
        Some(arr) => {
            let dirs: HashSet<PathBuf> = arr
                .into_iter()
                .filter_map(|s| s.as_str())
                .map(|s| PathBuf::from(s))
                .filter(|p| p.exists() && p.is_dir())
                .collect();
            LauncherType::File(FileLauncher {
                dirs,
                data,
                files: None,
            })
        }
        _ => LauncherType::Empty,
    }
}
fn parse_process_launcher(raw: &RawLauncher) -> LauncherType {
    let icon = raw
        .args
        .get("icon")
        .and_then(Value::as_str)
        .unwrap_or("sherlock-process");
    let launcher = ProcessLauncher::new(icon);
    if let Some(launcher) = launcher {
        LauncherType::Process(launcher)
    } else {
        LauncherType::Empty
    }
}
fn parse_weather_launcher(raw: &RawLauncher) -> LauncherType {
    if let Some(location) = raw.args.get("location").and_then(Value::as_str) {
        let update_interval = raw
            .args
            .get("update_interval")
            .and_then(Value::as_u64)
            .unwrap_or(60);
        LauncherType::Weather(WeatherLauncher {
            location: location.to_string(),
            update_interval,
        })
    } else {
        LauncherType::Empty
    }
}
fn parse_web_launcher(raw: &RawLauncher) -> LauncherType {
    LauncherType::Web(WebLauncher {
        display_name: raw.display_name.clone().unwrap_or("".to_string()),
        icon: raw
            .args
            .get("icon")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        engine: raw
            .args
            .get("search_engine")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(sherlock_error!(
                SherlockErrorType::FileExistError(fallback_path.clone()),
                format!(
                    "The file \"{}\" does not exist in the specified location.",
                    fallback_path.to_string_lossy()
                )
            )),
            Err(e) => Err(sherlock_error!(
                SherlockErrorType::FileReadError(fallback_path.clone()),
                e.to_string()
            )),
        }
    }

    fn load_default_fallback() -> Result<Vec<RawLauncher>, SherlockError> {
        // Loads default fallback.json file and loads the launcher configurations within.
        let data = gio::resources_lookup_data(
            "/dev/skxxtz/sherlock/fallback.json",
            gio::ResourceLookupFlags::NONE,
        )
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::ResourceLookupError("fallback.json".to_string()),
                e.to_string()
            )
        })?;
        let string_data = std::str::from_utf8(&data)
            .map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileParseError(PathBuf::from("fallback.json")),
                    e.to_string()
                )
            })?
            .to_string();
        serde_json::from_str(&string_data).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileParseError(PathBuf::from("fallback.json")),
                e.to_string()
            )
        })
    }

    let config = match load_user_fallback(fallback_path)
        .map_err(|e| non_breaking.push(e))
        .ok()
    {
        Some(v) => v,
        None => load_default_fallback()?,
    };

    return Ok((config, non_breaking));
}
