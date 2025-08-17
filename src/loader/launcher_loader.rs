use gio::glib::{idle_add, MainContext};
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use serde::de::IntoDeserializer;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;

use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;

use crate::actions::util::read_from_clipboard;
use crate::launcher::audio_launcher::AudioLauncherFunctions;
use crate::launcher::bookmark_launcher::BookmarkLauncher;
use crate::launcher::calc_launcher::{CalculatorLauncher, Currency, CURRENCIES};
use crate::launcher::category_launcher::CategoryLauncher;
use crate::launcher::emoji_picker::{EmojiPicker, SkinTone};
use crate::launcher::event_launcher::EventLauncher;
use crate::launcher::file_launcher::FileLauncher;
use crate::launcher::pomodoro_launcher::{Pomodoro, PomodoroStyle};
use crate::launcher::process_launcher::ProcessLauncher;
use crate::launcher::theme_picker::ThemePicker;
use crate::launcher::weather_launcher::WeatherLauncher;
use crate::launcher::{
    app_launcher, bulk_text_launcher, clipboard_launcher, system_cmd_launcher, web_launcher,
    Launcher, LauncherType,
};
use crate::loader::util::CounterReader;
use crate::ui::tiles::calc_tile::CalcTileHandler;
use crate::utils::cache::BinaryCache;
use crate::utils::config::{ConfigGuard, ConstantDefaults};
use crate::utils::errors::SherlockError;
use crate::utils::errors::SherlockErrorType;
use crate::utils::files::{expand_path, home_dir};

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
use crate::sherlock_error;

pub static COLOR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(rgb|hsl)*\(?(\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3})\)?|\(?(\s*\d{1,3}\s*,\s*\d{1,3}%\s*,\s*\d{1,3}\s*%\w*)\)?|^#([a-fA-F0-9]{6,8})$").unwrap()
});

impl Loader {
    #[sherlock_macro::timing(name = "Loading launchers")]
    pub fn load_launchers() -> Result<(Vec<Launcher>, Vec<SherlockError>), SherlockError> {
        let config = ConfigGuard::read()?;

        // Read fallback data here:
        let (raw_launchers, n) = parse_launcher_configs(&config.files.fallback)?;

        // Read cached counter file
        let counter_reader = CounterReader::new()?;
        let counts: HashMap<String, u32> =
            BinaryCache::read(&counter_reader.path).unwrap_or_default();
        let max_decimals = counts
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, v)| v.to_string().len())
            .unwrap_or(0) as i32;

        let submenu = config.runtime.sub_menu.clone();
        // Parse the launchers
        let launchers: Vec<Launcher> = raw_launchers
            .into_par_iter()
            .filter_map(|raw| {
                // Logic to restrict in submenu mode
                if submenu.is_some() {
                    if &submenu != &raw.alias {
                        return None;
                    }
                }
                let launcher_type: LauncherType = match raw.r#type.to_lowercase().as_str() {
                    "app_launcher" => parse_app_launcher(&raw, &counts, max_decimals),
                    "audio_sink" => parse_audio_sink_launcher(),
                    "bookmarks" => parse_bookmarks_launcher(&raw),
                    "bulk_text" => parse_bulk_text_launcher(&raw),
                    "calculation" => parse_calculator(&raw),
                    "categories" => parse_category_launcher(&raw, &counts, max_decimals),
                    "clipboard-execution" => parse_clipboard_launcher(&raw).ok()?,
                    "command" => parse_command_launcher(&raw, &counts, max_decimals),
                    "debug" => parse_debug_launcher(&raw, &counts, max_decimals),
                    "emoji_picker" => parse_emoji_launcher(&raw),
                    "files" => parse_file_launcher(&raw),
                    "teams_event" => parse_event_launcher(&raw),
                    "theme_picker" => parse_theme_launcher(&raw),
                    "process" => parse_process_launcher(&raw),
                    "pomodoro" => parse_pomodoro(&raw),
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
                Some(Launcher::from_raw(raw, method, launcher_type, icon))
            })
            .collect();

        // Get errors and launchers
        let mut non_breaking = Vec::new();
        if counts.is_empty() {
            let counts: HashMap<String, u32> = launchers
                .iter()
                .filter_map(|launcher| launcher.get_execs())
                .flat_map(|exec_set| exec_set.into_iter().map(|exec| (exec, 0)))
                .collect();
            if let Err(e) = BinaryCache::write(&counter_reader.path, &counts) {
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
    counts: &HashMap<String, u32>,
    max_decimals: i32,
) -> Vec<AppData> {
    let data: HashSet<AppData> =
        deserialize_named_appdata(value.clone().into_deserializer()).unwrap_or_default();
    data.into_iter()
        .map(|c| {
            let count = c
                .exec
                .as_ref()
                .and_then(|exec| counts.get(exec))
                .unwrap_or(&0);
            c.with_priority(parse_priority(prio, *count, max_decimals))
        })
        .collect::<Vec<AppData>>()
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_app_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, u32>,
    max_decimals: i32,
) -> LauncherType {
    let apps: Vec<AppData> = ConfigGuard::read().ok().map_or_else(
        || Vec::new(),
        |config| {
            let prio = raw.priority;
            match config.caching.enable {
                true => Loader::load_applications(prio, counts, max_decimals).unwrap_or_default(),
                false => Loader::load_applications_from_disk(None, prio, counts, max_decimals)
                    .unwrap_or_default(),
            }
        },
    );
    LauncherType::App(AppLauncher { apps })
}
#[sherlock_macro::timing(level = "launchers")]
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
#[sherlock_macro::timing(level = "launchers")]
fn parse_bookmarks_launcher(raw: &RawLauncher) -> LauncherType {
    if let Some(browser) = raw
        .args
        .get("browser")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .or_else(|| ConstantDefaults::browser().ok())
    {
        match BookmarkLauncher::find_bookmarks(&browser, raw) {
            Ok(bookmarks) => {
                return LauncherType::Bookmark(BookmarkLauncher { bookmarks });
            }
            Err(err) => {
                let _result = err.insert(false);
            }
        }
    }
    LauncherType::Empty
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_bulk_text_launcher(raw: &RawLauncher) -> LauncherType {
    LauncherType::Api(BulkTextLauncher {
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

    idle_add(move || {
        MainContext::default().spawn_local(async move {
            let result = Currency::get_exchange(update_interval).await.ok();
            let _result = CURRENCIES.set(result);
        });
        false.into()
    });

    LauncherType::Calc(CalculatorLauncher { capabilities })
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_category_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, u32>,
    max_decimals: i32,
) -> LauncherType {
    let prio = raw.priority;
    let value = &raw.args["categories"];
    let categories = parse_appdata(value, prio, counts, max_decimals);
    LauncherType::Category(CategoryLauncher { categories })
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_clipboard_launcher(raw: &RawLauncher) -> Result<LauncherType, SherlockError> {
    let clipboard_content: String = read_from_clipboard()?;

    if clipboard_content.trim().is_empty() {
        return Ok(LauncherType::Empty);
    }

    let capabilities: HashSet<String> = match raw.args.get("capabilities") {
        Some(Value::Array(arr)) => {
            let strings: HashSet<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            strings
        }
        _ => vec!["url", "calc.math", "calc.units", "colors.all"]
            .into_iter()
            .map(String::from)
            .collect::<HashSet<_>>(),
    };

    if clipboard_content.is_empty() {
        Ok(LauncherType::Empty)
    } else {
        // Check if the content is in a suitable format

        let mut has_val = false;
        if capabilities.contains("url") {
            if clipboard_content.starts_with("http://")
                || clipboard_content.starts_with("https://")
                || clipboard_content.starts_with("www.")
            {
                has_val = clipboard_content.find('.').is_some();
            }
        }

        if !has_val
            && capabilities
                .iter()
                .find(|c| c.starts_with("colors."))
                .is_some()
            && clipboard_content.len() <= 20
        {
            has_val = COLOR_RE.is_match(&clipboard_content);
        }

        if !has_val
            && capabilities
                .iter()
                .find(|c| c.starts_with("calc."))
                .is_some()
        {
            let handler = CalcTileHandler::default();
            has_val = handler.based_show(&clipboard_content, &capabilities);
        }

        if !has_val {
            return Ok(LauncherType::Empty);
        }

        Ok(LauncherType::Clipboard(ClipboardLauncher {
            clipboard_content,
            capabilities: capabilities.clone(),
        }))
    }
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_command_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, u32>,
    max_decimals: i32,
) -> LauncherType {
    let prio = raw.priority;
    let value = &raw.args["commands"];
    let commands = parse_appdata(value, prio, counts, max_decimals);
    LauncherType::Command(CommandLauncher { commands })
}

#[sherlock_macro::timing(level = "launchers")]
fn parse_pomodoro(raw: &RawLauncher) -> LauncherType {
    let home = match home_dir() {
        Ok(dir) => dir,
        Err(_) => return LauncherType::Empty,
    };
    let program_raw = raw
        .args
        .get("program")
        .and_then(Value::as_str)
        .unwrap_or("");
    let program = expand_path(program_raw, &home);
    let socket = PathBuf::from(raw.args.get("socket").and_then(Value::as_str).unwrap_or(""));
    let style_raw = raw
        .args
        .get("style")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    let style = PomodoroStyle::from_str(&style_raw).unwrap(); // cant panic
    LauncherType::Pomodoro(Pomodoro {
        program,
        socket,
        style,
    })
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_debug_launcher(
    raw: &RawLauncher,
    counts: &HashMap<String, u32>,
    max_decimals: i32,
) -> LauncherType {
    let prio = raw.priority;
    let value = &raw.args["commands"];
    let commands = parse_appdata(value, prio, counts, max_decimals);
    LauncherType::Command(CommandLauncher { commands })
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_emoji_launcher(raw: &RawLauncher) -> LauncherType {
    let mut app_data = AppData::from_raw_launcher(raw);
    if app_data.icon.is_none() {
        app_data.icon = Some(String::from("sherlock-emoji"))
    }
    let default_skin_tone: SkinTone = raw
        .args
        .get("default_skin_tone")
        .and_then(|val| serde_json::from_value(val.clone()).ok())
        .unwrap_or(SkinTone::Medium);

    LauncherType::Emoji(EmojiPicker {
        rows: 4,
        cols: 5,
        default_skin_tone,
        data: vec![app_data],
    })
}
#[sherlock_macro::timing(level = "launchers")]
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
    match EventLauncher::get_event(date, event_start, event_end) {
        Some(event) => LauncherType::Event(EventLauncher { event, icon }),
        _ => LauncherType::Empty,
    }
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_theme_launcher(raw: &RawLauncher) -> LauncherType {
    let relative = raw
        .args
        .get("location")
        .and_then(Value::as_str)
        .unwrap_or("~/.config/sherlock/themes/");
    let relative = relative.strip_prefix("~/").unwrap_or(relative);
    let home = match home_dir() {
        Ok(dir) => dir,
        Err(_) => return LauncherType::Empty,
    };
    let absolute = home.join(relative);
    ThemePicker::new(absolute, raw)
}
#[sherlock_macro::timing(level = "launchers")]
fn parse_file_launcher(raw: &RawLauncher) -> LauncherType {
    let mut data: Vec<AppData> = Vec::with_capacity(1);
    let mut app_data = AppData::from_raw_launcher(raw);
    if app_data.icon.is_none() {
        app_data.icon = Some(String::from("files"))
    }
    data.push(app_data);
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
#[sherlock_macro::timing(level = "launchers")]
fn parse_process_launcher(raw: &RawLauncher) -> LauncherType {
    let launcher = ProcessLauncher::new(raw.priority);
    if let Some(launcher) = launcher {
        LauncherType::Process(launcher)
    } else {
        LauncherType::Empty
    }
}
#[sherlock_macro::timing(level = "launchers")]
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
#[sherlock_macro::timing(level = "launchers")]
fn parse_web_launcher(raw: &RawLauncher) -> LauncherType {
    let browser = raw
        .args
        .get("browser")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
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
        browser,
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut non_breaking: Vec<SherlockError> = Vec::new();
                let config = match load_default_fallback()
                    .map_err(|e| non_breaking.push(e))
                    .ok()
                {
                    Some(v) => v,
                    None => load_default_fallback()?,
                };
                let content = serde_json::to_string_pretty(&config).unwrap();
                fs::write(fallback_path, content).map_err(|e| {
                    sherlock_error!(
                        SherlockErrorType::FileWriteError(fallback_path.clone()),
                        e.to_string()
                    )
                })?;
                Ok(config)
            }
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
