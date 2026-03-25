pub mod app_launcher;
pub mod audio_launcher;
pub mod bookmark_launcher;
pub mod calc_launcher;
pub mod category_launcher;
pub mod children;
pub mod clipboard_launcher;
pub mod emoji_launcher;
pub mod event_launcher;
pub mod system_cmd_launcher;
pub mod utils;
pub mod weather_launcher;
pub mod web_launcher;
// Integrate later: TODO
// pub mod clipboard_launcher;
// pub mod bulk_text_launcher;
// pub mod pipe_launcher;
// pub mod emoji_picker;
// pub mod file_launcher;
// pub mod pomodoro_launcher;
// pub mod process_launcher;
// pub mod theme_picker;

use serde::de::IntoDeserializer;
use std::{collections::HashMap, sync::Arc, vec};

use crate::{
    launcher::{
        children::{
            RenderableChild, calc_data::CalcData, clip_data::ClipData,
            emoji_data::set_selected_skin_tone,
        },
        clipboard_launcher::ClipboardLauncher,
        emoji_launcher::{EmojiData, SkinTone},
        weather_launcher::WeatherData,
    },
    loader::{
        Loader,
        application_loader::parse_priority,
        resolve_icon_path,
        utils::{
            AppData, ApplicationAction, ContextMenuAction, RawLauncher, deserialize_named_appdata,
        },
    },
    ui::launcher::{LauncherMode, views::NavigationViewType},
    utils::{config::HomeType, intent::Capabilities},
};

use app_launcher::AppLauncher;
use audio_launcher::MusicPlayerLauncher;
use bookmark_launcher::BookmarkLauncher;
use calc_launcher::CalculatorLauncher;
use category_launcher::CategoryLauncher;
use emoji_launcher::EmojiPicker;
use event_launcher::EventLauncher;
use gpui::SharedString;
use serde_json::Value;
use system_cmd_launcher::CommandLauncher;
use weather_launcher::WeatherLauncher;
use web_launcher::WebLauncher;

// Integrate later: TODO
// use bulk_text_launcher::BulkTextLauncher;
// use clipboard_launcher::ClipboardLauncher;
// use emoji_picker::EmojiPicker;
// use file_launcher::FileLauncher;
// use pomodoro_launcher::Pomodoro;
// use process_launcher::ProcessLauncher;
// use theme_picker::ThemePicker;

#[derive(Clone, Debug, Default)]
pub enum LauncherType {
    App(AppLauncher),
    Bookmark(BookmarkLauncher),
    Calc(CalculatorLauncher),
    Category(CategoryLauncher),
    Clipboard(ClipboardLauncher),
    Command(CommandLauncher),
    Event(EventLauncher),
    MusicPlayer(MusicPlayerLauncher),
    Weather(WeatherLauncher),
    Web(WebLauncher),
    Emoji(EmojiPicker),
    #[default]
    Empty,
    // Integrate later: TODO
    // Pipe(PipeLauncher),
    // Api(BulkTextLauncher),
    // File(FileLauncher),
    // Pomodoro(Pomodoro),
    // Process(ProcessLauncher),
    // Theme(ThemePicker),
}

impl LauncherType {
    pub fn get_render_obj(
        &self,
        launcher: Arc<Launcher>,
        opts: Arc<Value>,
        counts: &HashMap<String, u32>,
        decimals: i32,
    ) -> Option<Vec<RenderableChild>> {
        match self {
            Self::App(app) => {
                Loader::load_applications(Arc::clone(&launcher), counts, decimals, app.use_keywords)
                    .map(|ad| {
                        ad.into_iter()
                            .map(|inner| RenderableChild::AppLike {
                                launcher: Arc::clone(&launcher),
                                inner,
                            })
                            .collect()
                    })
                    .ok()
            }

            Self::Bookmark(bkm) => {
                BookmarkLauncher::find_bookmarks(&bkm.target_browser, Arc::clone(&launcher))
                    .map(|ad| {
                        ad.into_iter()
                            .map(|inner| RenderableChild::AppLike {
                                launcher: Arc::clone(&launcher),
                                inner,
                            })
                            .collect()
                    })
                    .ok()
            }

            Self::Calc(_) => {
                let capabilities: Vec<String> = match opts.get("capabilities") {
                    Some(Value::Array(arr)) => arr
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                    _ => vec![String::from("calc.math"), String::from("calc.units")],
                };
                let caps = Capabilities::from_strings(&capabilities);
                let inner = CalcData::new(caps);

                Some(vec![RenderableChild::CalcLike { launcher, inner }])
            }

            Self::Category(_) => {
                let cmds = opts.get("categories")?;
                let app_data =
                    deserialize_named_appdata(cmds.clone().into_deserializer()).unwrap_or_default();

                let children: Vec<RenderableChild> = app_data
                    .into_iter()
                    .map(|mut inner| {
                        let count = inner
                            .exec
                            .as_deref()
                            .and_then(|exec| counts.get(exec))
                            .copied()
                            .unwrap_or(0u32);
                        inner.icon = inner
                            .icon
                            .and_then(|i| i.to_str().and_then(resolve_icon_path));
                        inner.priority =
                            Some(parse_priority(launcher.priority as f32, count, decimals));
                        inner.actions = inner
                            .actions
                            .iter()
                            .map(|action_arc| {
                                match action_arc.as_ref() {
                                    ContextMenuAction::App(app_action) => {
                                        // 1. Resolve the path
                                        let resolved_icon = app_action
                                            .icon
                                            .as_deref()
                                            .and_then(|p| p.to_str())
                                            .and_then(|s| resolve_icon_path(s));

                                        Arc::new(ContextMenuAction::App(ApplicationAction {
                                            icon: resolved_icon,
                                            ..app_action.clone()
                                        }))
                                    }
                                    ContextMenuAction::Emoji(_) => Arc::clone(action_arc),
                                }
                            })
                            .collect();

                        RenderableChild::AppLike {
                            launcher: Arc::clone(&launcher),
                            inner,
                        }
                    })
                    .collect();

                Some(children)
            }

            Self::Clipboard(_) => {
                let capabilities: Vec<String> = match opts.get("capabilities") {
                    Some(Value::Array(arr)) => arr
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                    _ => vec![String::from("calc.math"), String::from("calc.units")],
                };
                let caps = Capabilities::from_strings(&capabilities);
                let inner = ClipData::new(caps, SharedString::from(""));

                Some(vec![RenderableChild::ClipLike { launcher, inner }])
            }

            Self::Command(_) => {
                let cmds = opts.get("commands")?;
                let app_data =
                    deserialize_named_appdata(cmds.clone().into_deserializer()).unwrap_or_default();
                let children: Vec<RenderableChild> = app_data
                    .into_iter()
                    .map(|mut inner| {
                        let count = inner
                            .exec
                            .as_deref()
                            .and_then(|exec| counts.get(exec))
                            .copied()
                            .unwrap_or(0u32);
                        inner.icon = inner
                            .icon
                            .and_then(|i| i.to_str().and_then(resolve_icon_path));
                        inner.priority =
                            Some(parse_priority(launcher.priority as f32, count, decimals));
                        RenderableChild::AppLike {
                            launcher: Arc::clone(&launcher),
                            inner,
                        }
                    })
                    .collect();

                Some(children)
            }

            Self::Emoji(_) => {
                let mut inner = AppData::new();
                inner.name = launcher.name.as_ref().map(SharedString::from);
                inner.search_string = "emoji".into();
                inner.icon = resolve_icon_path("sherlock-emoji");

                let default_skin_tone: SkinTone = opts
                    .get("default_skin_color")
                    .and_then(|s| serde_json::from_value(s.clone()).ok())
                    .unwrap_or(SkinTone::Simpsons);
                set_selected_skin_tone(default_skin_tone, 0);

                let child = RenderableChild::AppLike { launcher, inner };

                Some(vec![child])
            }

            Self::MusicPlayer(_) => {
                let inner = utils::MprisState {
                    raw: None,
                    image: None,
                };
                Some(vec![RenderableChild::MusicLike { launcher, inner }])
            }

            Self::Weather(wttr) => {
                match WeatherData::from_cache(wttr) {
                    Some(inner) => Some(vec![RenderableChild::WeatherLike { launcher, inner }]),
                    None => {
                        // Return None or a "Loading" placeholder for now
                        Some(vec![RenderableChild::WeatherLike {
                            launcher: Arc::clone(&launcher),
                            inner: WeatherData::uninitialized(),
                        }])
                    }
                }
            }

            Self::Web(_) => {
                let mut inner = AppData::new();
                inner.icon = opts
                    .get("icon")
                    .and_then(Value::as_str)
                    .and_then(|i| resolve_icon_path(i));

                Some(vec![RenderableChild::AppLike { launcher, inner }])
            }

            _ => None,
        }
    }
}

// // Async tiles
// LauncherType::BulkText(bulk_text) => Tile::bulk_text_tile(launcher, &bulk_text).await,
// LauncherType::MusicPlayer(mpris) => Tile::mpris_tile(launcher, &mpris).await,
// LauncherType::Weather(_) => Tile::weather_tile_loader(launcher).await,
/// # Launcher
/// ### Fields:
/// - **name:** Specifies the name of the launcher – such as a category e.g. `App Launcher`
/// - **alias:** Also referred to as `mode` – specifies the mode in which the launcher children should
/// be active in
/// - **tag_start:** Specifies the text displayed in a custom UI Label
/// - **tag_end:** Specifies the text displayed in a custom UI Label
/// - **method:** Specifies the action that should be executed on `row-should-activate` action
/// - **next_content:** Specifies the content to be displayed whenever method is `next`
/// - **priority:** Base priority all children inherit from. Children priority will be a combination
/// of this together with their execution counts and levenshtein similarity
/// - **r#async:** Specifies whether the tile should be loaded/executed asynchronously
/// - **home:** Specifies whether the children should show on the `home` mode (empty
/// search entry & mode == `all`)
/// - **launcher_type:** Used to specify the kind of launcher and subsequently its children
/// - **shortcut:** Specifies whether the child tile should show `modekey + number` shortcuts
/// - **spawn_focus:** Specifies whether the tile should have focus whenever Sherlock launches
/// search entry & mode == `all`)
#[derive(Clone, Debug, Default)]
pub struct Launcher {
    pub name: Option<String>,
    pub display_name: Option<SharedString>,
    pub icon: Option<String>, // nu
    pub alias: Option<String>,
    pub method: String,               // nu
    pub exit: bool,                   // nu
    pub next_content: Option<String>, // nu
    pub priority: u32,
    pub r#async: bool, // nu
    pub home: HomeType,
    pub launcher_type: LauncherType,
    pub shortcut: bool,                              // nu
    pub spawn_focus: bool,                           // nu
    pub actions: Option<Vec<ApplicationAction>>,     // nu
    pub add_actions: Option<Vec<ApplicationAction>>, // nu
}
impl Launcher {
    pub fn from_raw(
        raw: RawLauncher,
        method: String,
        launcher_type: LauncherType,
        icon: Option<String>,
    ) -> Self {
        Self {
            name: raw.name,
            display_name: raw.display_name.map(|n| SharedString::from(n)),
            icon,
            alias: raw.alias,
            method,
            exit: raw.exit,
            next_content: raw.next_content,
            priority: raw.priority as u32,
            r#async: raw.r#async,
            home: raw.home,
            launcher_type,
            shortcut: raw.shortcut,
            spawn_focus: raw.spawn_focus,
            actions: raw.actions,
            add_actions: raw.add_actions,
        }
    }
}

pub enum ExecMode {
    App {
        exec: String,
        terminal: bool,
    },
    Commmand {
        exec: String,
    },
    Category {
        category: LauncherMode,
    },
    View {
        mode: NavigationViewType,
        launcher: Arc<Launcher>,
    },
    CreateBookmark {
        url: String,
        name: String,
    },
    Web {
        engine: Option<String>,
        browser: Option<String>,
        exec: Option<String>,
    },
    Copy {
        content: String,
    },
    None,
}
impl ExecMode {
    pub fn from_appdata(app_data: &AppData, launcher: &Arc<Launcher>) -> Self {
        match &launcher.launcher_type {
            LauncherType::App(_) => Self::App {
                exec: app_data.exec.clone().unwrap_or_default(),
                terminal: app_data.terminal,
            },
            LauncherType::Bookmark(bkm) => Self::Web {
                engine: None,
                browser: Some(bkm.target_browser.clone()),
                exec: app_data.exec.clone(),
            },
            LauncherType::Category(_) => Self::Category {
                category: LauncherMode::Alias {
                    short: app_data
                        .exec
                        .as_ref()
                        .map(SharedString::from)
                        .unwrap_or_default(),
                    name: app_data.name.clone().unwrap_or_default(),
                },
            },
            LauncherType::Command(_) => Self::Commmand {
                exec: app_data.exec.clone().unwrap_or_default(),
            },
            LauncherType::Emoji(_) => Self::View {
                mode: NavigationViewType::Emoji,
                launcher: Arc::clone(launcher),
            },
            LauncherType::Web(web) => Self::Web {
                engine: Some(web.engine.clone()),
                browser: web.browser.clone(),
                exec: app_data.exec.clone(),
            },
            _ => Self::None,
        }
    }
    pub fn from_app_action(action: &ContextMenuAction, _data: &RenderableChild) -> Self {
        match action {
            ContextMenuAction::App(action) => match action.method.as_str() {
                "app_launcher" | "command" => Self::Commmand {
                    exec: action.exec.clone().unwrap_or_default(),
                },

                "create_bookmark" => {
                    if let (Some(exec), Some(name)) = (&action.exec, &action.name) {
                        Self::CreateBookmark {
                            url: exec.to_string(),
                            name: name.to_string(),
                        }
                    } else {
                        Self::None
                    }
                }

                _ => Self::None,
            },
            ContextMenuAction::Emoji(_) => Self::None,
        }
    }
}
