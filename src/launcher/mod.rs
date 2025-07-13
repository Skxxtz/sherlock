use std::{collections::HashSet, rc::Rc};

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
    g_subclasses::{
        sherlock_row::{SherlockRow, SherlockRowBind},
        tile_item::{TileItem, UpdateHandler},
    },
    loader::util::{AppData, ApplicationAction, RawLauncher},
    ui::tiles::{
        app_tile::AppTileHandler, calc_tile::CalcTileHandler, pomodoro_tile::PomodoroTileHandler,
        web_tile::WebTileHandler, Tile,
    },
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
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::Cast, property::PropertySet};
use gtk4::Widget;
use pomodoro_launcher::Pomodoro;
use process_launcher::ProcessLauncher;
use simd_json::prelude::ArrayTrait;
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
impl Default for LauncherType {
    fn default() -> Self {
        Self::Empty
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
    pub binds: Option<Vec<SherlockRowBind>>,
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
            binds: raw.binds,
        }
    }
}

impl Launcher {
    // TODO: tile method recreates already stored data...
    pub fn bind_obj(&self, launcher: Rc<Launcher>) -> Vec<TileItem> {
        match self.launcher_type {
            LauncherType::App(_)
            | LauncherType::Bookmark(_)
            | LauncherType::Category(_)
            | LauncherType::Command(_)
            | LauncherType::Emoji(_)
            | LauncherType::File(_)
            | LauncherType::Theme(_) => {
                // Get app data value
                let Some(inner) = self.inner() else {
                    return vec![];
                };
                inner
                    .iter()
                    .enumerate()
                    .map(|(i, _app)| {
                        let base = TileItem::new();
                        base.set_index(i);
                        base.set_launcher(launcher.clone());

                        base
                    })
                    .collect()
            }

            LauncherType::Calc(_) | LauncherType::Pomodoro(_) => {
                let base = TileItem::new();
                base.set_launcher(launcher.clone());
                let handler = CalcTileHandler::default();
                base.imp()
                    .update_handler
                    .set(UpdateHandler::Calculator(handler));
                vec![base]
            }
            LauncherType::Web(_) => {
                let base = TileItem::new();
                base.set_launcher(launcher.clone());
                vec![base]
            }
            _ => vec![],
        }
    }
    pub fn inner(&self) -> Option<&Vec<AppData>> {
        match &self.launcher_type {
            LauncherType::App(app) => Some(&app.apps),
            LauncherType::Bookmark(bkm) => Some(&bkm.bookmarks),
            LauncherType::Category(cat) => Some(&cat.categories),
            LauncherType::Command(cmd) => Some(&cmd.commands),
            LauncherType::Emoji(emj) => Some(&emj.data),
            LauncherType::File(f) => Some(&f.data),
            LauncherType::Theme(thm) => Some(&thm.themes),
            _ => None,
        }
    }
    // LauncherType::Clipboard((clp, calc))
    // LauncherType::Event(evl) => Tile::event_tile(launcher, &evl).await,
    // LauncherType::Process(proc) => Tile::process_tile(launcher, &proc).await,
    // LauncherType::Web(web) => Tile::web_tile(launcher, &web).await,
    pub fn get_tile(
        &self,
        index: Option<u16>,
        launcher: Rc<Launcher>,
        item: &TileItem,
    ) -> Option<(Widget, UpdateHandler)> {
        match &self.launcher_type {
            //
            // App Tile Based
            //
            LauncherType::App(_)
            | LauncherType::Bookmark(_)
            | LauncherType::Category(_)
            | LauncherType::Command(_)
            | LauncherType::Emoji(_)
            | LauncherType::File(_)
            | LauncherType::Theme(_) => {
                // Get app data value
                let inner = self.inner()?;
                let value = inner.get(index? as usize)?;
                let tile = Tile::app(value, launcher, item);
                let update = UpdateHandler::AppTile(AppTileHandler::new(&tile));
                Some((tile.upcast::<Widget>(), update))
            }

            LauncherType::Calc(_) => {
                let tile = Tile::calculator();
                let update = UpdateHandler::Calculator(CalcTileHandler::new(&tile));
                Some((tile.upcast::<Widget>(), update))
            }
            LauncherType::Pomodoro(pmd) => {
                let tile = Tile::pomodoro(launcher);
                let update = UpdateHandler::Pomodoro(PomodoroTileHandler::new(&tile, &pmd));
                Some((tile.upcast::<Widget>(), update))
            }
            LauncherType::Web(web) => {
                let tile = Tile::web(launcher, &web);
                let update = UpdateHandler::WebTile(WebTileHandler::new(&tile));
                Some((tile.upcast::<Widget>(), update))
            }

            _ => None,
        }
    }
    pub async fn get_patch(launcher_instance: Rc<Self>) -> Vec<SherlockRow> {
        let launcher = Rc::clone(&launcher_instance);
        match &launcher_instance.launcher_type {
            LauncherType::Clipboard((clp, calc)) => {
                Tile::clipboard_tile(launcher, &clp, &calc).await
            }
            LauncherType::Event(evl) => Tile::event_tile(launcher, &evl).await,
            LauncherType::Process(proc) => Tile::process_tile(launcher, &proc).await,

            // Async tiles
            LauncherType::BulkText(bulk_text) => Tile::bulk_text_tile(launcher, &bulk_text).await,
            LauncherType::MusicPlayer(mpris) => Tile::mpris_tile(launcher, &mpris).await,
            LauncherType::Weather(_) => Tile::weather_tile_loader(launcher).await,
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
