use std::collections::HashSet;

pub mod app_launcher;
pub mod audio_launcher;
pub mod bookmark_launcher;
pub mod bulk_text_launcher;
pub mod calc_launcher;
pub mod category_launcher;
pub mod clipboard_launcher;
pub mod emoji_picker;
pub mod event_launcher;
pub mod file_launcher;
pub mod pomodoro_launcher;
pub mod process_launcher;
pub mod system_cmd_launcher;
pub mod theme_picker;
pub mod utils;
pub mod weather_launcher;
pub mod web_launcher;

use crate::{
    g_subclasses::sherlock_row::SherlockRow,
    loader::util::{ApplicationAction, RawLauncher},
    ui::tiles::Tile,
};

use app_launcher::AppLauncher;
use audio_launcher::MusicPlayerLauncher;
use bookmark_launcher::BookmarkLauncher;
use bulk_text_launcher::{AsyncCommandResponse, BulkTextLauncher};
use calc_launcher::CalculatorLauncher;
use category_launcher::CategoryLauncher;
use clipboard_launcher::ClipboardLauncher;
use emoji_picker::EmojiPicker;
use event_launcher::EventLauncher;
use file_launcher::FileLauncher;
use pomodoro_launcher::Pomodoro;
use process_launcher::ProcessLauncher;
use system_cmd_launcher::CommandLauncher;
use theme_picker::ThemePicker;
use utils::HomeType;
use weather_launcher::{WeatherData, WeatherLauncher};
use web_launcher::WebLauncher;

#[derive(Clone, Debug)]
pub enum LauncherType {
    App(AppLauncher),
    Bookmark(BookmarkLauncher),
    BulkText(BulkTextLauncher),
    Calc(CalculatorLauncher),
    Category(CategoryLauncher),
    Clipboard((ClipboardLauncher, CalculatorLauncher)),
    Command(CommandLauncher),
    Emoji(EmojiPicker),
    Event(EventLauncher),
    File(FileLauncher),
    MusicPlayer(MusicPlayerLauncher),
    Pomodoro(Pomodoro),
    Process(ProcessLauncher),
    Theme(ThemePicker),
    Weather(WeatherLauncher),
    Web(WebLauncher),
    Empty,
}
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
#[derive(Clone, Debug)]
pub struct Launcher {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub alias: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub method: String,
    pub exit: bool,
    pub next_content: Option<String>,
    pub priority: u32,
    pub r#async: bool,
    pub home: HomeType,
    pub launcher_type: LauncherType,
    pub shortcut: bool,
    pub spawn_focus: bool,
    pub actions: Option<Vec<ApplicationAction>>,
    pub add_actions: Option<Vec<ApplicationAction>>,
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
            icon: icon.clone(),
            alias: raw.alias,
            tag_start: raw.tag_start,
            tag_end: raw.tag_end,
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

impl Launcher {
    // TODO: tile method recreates already stored data...
    pub fn get_patch(&mut self) -> Vec<SherlockRow> {
        match &self.launcher_type {
            LauncherType::App(app) => Tile::app_tile(self, &app.apps),
            LauncherType::Bookmark(bmk) => Tile::app_tile(self, &bmk.bookmarks),
            LauncherType::Calc(calc) => Tile::calc_tile(self, &calc),
            LauncherType::Category(ctg) => Tile::app_tile(self, &ctg.categories),
            LauncherType::Clipboard((clp, calc)) => Tile::clipboard_tile(self, &clp, &calc),
            LauncherType::Command(cmd) => Tile::app_tile(self, &cmd.commands),
            LauncherType::Event(evl) => Tile::event_tile(self, evl),
            LauncherType::Emoji(emj) => Tile::app_tile(self, &emj.data),
            LauncherType::File(f) => Tile::app_tile(self, &f.data),
            LauncherType::Theme(thm) => Tile::app_tile(self, &thm.themes),
            LauncherType::Process(proc) => Tile::process_tile(self, proc),
            LauncherType::Pomodoro(pmd) => Tile::pomodoro_tile(self, pmd),
            LauncherType::Web(web) => Tile::web_tile(self, &web),

            // Async tiles
            LauncherType::BulkText(bulk_text) => Tile::bulk_text_tile(self, &bulk_text),
            LauncherType::MusicPlayer(mpris) => Tile::mpris_tile(self, &mpris),
            LauncherType::Weather(_) => Tile::weather_tile_loader(self),
            _ => Vec::new(),
        }
    }
    pub fn get_execs(&self) -> Option<HashSet<String>> {
        // NOTE: make a function to check for exec changes in the caching algorithm
        match &self.launcher_type {
            LauncherType::App(app) => {
                let execs: HashSet<String> =
                    app.apps.iter().filter_map(|v| v.exec.clone()).collect();
                Some(execs)
            }
            LauncherType::Web(web) => {
                let exec = format!("websearch-{}", web.engine);
                let execs: HashSet<String> = HashSet::from([(exec)]);
                Some(execs)
            }
            LauncherType::Command(cmd) => {
                let execs: HashSet<String> =
                    cmd.commands.iter().filter_map(|v| v.exec.clone()).collect();
                Some(execs)
            }
            LauncherType::Category(ctg) => {
                let execs: HashSet<String> = ctg
                    .categories
                    .iter()
                    .filter_map(|v| v.exec.clone())
                    .collect();
                Some(execs)
            }

            // None-Home Launchers
            LauncherType::Calc(_) => None,
            LauncherType::BulkText(_) => None,
            LauncherType::Clipboard(_) => None,
            LauncherType::Event(_) => None,
            _ => None,
        }
    }
    pub async fn get_result(&self, keyword: &str) -> Option<AsyncCommandResponse> {
        match &self.launcher_type {
            LauncherType::BulkText(bulk_text) => bulk_text.get_result(keyword).await,
            _ => None,
        }
    }
    pub async fn get_image(&self) -> Option<(gdk_pixbuf::Pixbuf, bool)> {
        match &self.launcher_type {
            LauncherType::MusicPlayer(mpis) => mpis.get_image().await,
            _ => None,
        }
    }
    pub async fn get_weather(&self) -> Option<(WeatherData, bool)> {
        match &self.launcher_type {
            LauncherType::Weather(wtr) => wtr.get_result().await,
            _ => None,
        }
    }
}
