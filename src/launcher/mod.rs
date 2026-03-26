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

use crate::{
    launcher::{
        children::{
            RenderableChild,
            emoji_data::{apply_skin_tones, get_selected_skin_tones},
        },
        clipboard_launcher::ClipboardLauncher,
    },
    loader::{
        LoadContext, resolve_icon_path,
        utils::{AppData, ApplicationAction, RawLauncher},
    },
    ui::launcher::{LauncherMode, context_menu::ContextMenuAction, views::NavigationViewType},
    utils::{config::HomeType, errors::SherlockError},
};
use std::{path::Path, sync::Arc, vec};

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

pub trait LauncherProvider {
    fn parse(raw: &RawLauncher) -> LauncherType;
    fn objects(
        &self,
        launcher: Arc<Launcher>,
        ctx: &LoadContext,
        opts: Arc<serde_json::Value>,
    ) -> Result<Vec<RenderableChild>, SherlockError>;
}

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
        ctx: &LoadContext,
        opts: Arc<Value>,
    ) -> Result<Vec<RenderableChild>, SherlockError> {
        match self {
            Self::App(app) => app.objects(launcher, ctx, opts),
            Self::Bookmark(bkm) => bkm.objects(launcher, ctx, opts),
            Self::Calc(calc) => calc.objects(launcher, ctx, opts),
            Self::Category(cat) => cat.objects(launcher, ctx, opts),
            Self::Clipboard(clip) => clip.objects(launcher, ctx, opts),
            Self::Command(cmd) => cmd.objects(launcher, ctx, opts),
            Self::Emoji(emj) => emj.objects(launcher, ctx, opts),
            Self::MusicPlayer(mus) => mus.objects(launcher, ctx, opts),
            Self::Weather(wttr) => wttr.objects(launcher, ctx, opts),
            Self::Web(web) => web.objects(launcher, ctx, opts),
            _ => Ok(vec![]),
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
    pub icon: Option<Arc<Path>>, // nu
    pub alias: Option<String>,
    pub method: String, // nu
    pub exit: bool,     // nu
    pub priority: u32,
    pub r#async: bool,
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
            icon: icon.as_deref().and_then(resolve_icon_path),
            alias: raw.alias,
            method,
            exit: raw.exit,
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
            ContextMenuAction::Emoji(emj) => {
                let emoji = emj.emoji();
                let content = apply_skin_tones(emoji, &get_selected_skin_tones())
                    .as_str()
                    .to_string();

                Self::Copy { content }
            }
        }
    }
}
