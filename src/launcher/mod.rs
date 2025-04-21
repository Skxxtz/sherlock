use std::collections::{HashMap, HashSet};

use gtk4::Box;

pub mod app_launcher;
pub mod audio_launcher;
pub mod bulk_text_launcher;
pub mod calc_launcher;
pub mod category_launcher;
pub mod clipboard_launcher;
pub mod event_launcher;
pub mod process_launcher;
pub mod system_cmd_launcher;
mod utils;
pub mod weather_launcher;
pub mod web_launcher;

use crate::{
    g_subclasses::sherlock_row::SherlockRow,
    loader::util::AppData,
    ui::tiles::{util::AsyncLauncherTile, Tile},
};

use app_launcher::App;
use audio_launcher::MusicPlayerLauncher;
use bulk_text_launcher::BulkText;
use calc_launcher::Calculator;
use category_launcher::CategoryLauncher;
use clipboard_launcher::ClipboardLauncher;
use event_launcher::EventLauncher;
use process_launcher::ProcessLauncher;
use system_cmd_launcher::SystemCommand;
use weather_launcher::{WeatherData, WeatherLauncher};
use web_launcher::Web;

#[derive(Clone, Debug)]
pub enum LauncherType {
    CategoryLauncher(CategoryLauncher),
    App(App),
    Web(Web),
    Calc(Calculator),
    BulkText(BulkText),
    SystemCommand(SystemCommand),
    Clipboard((ClipboardLauncher, Calculator)),
    EventLauncher(EventLauncher),
    MusicPlayerLauncher(MusicPlayerLauncher),
    ProcessLauncher(ProcessLauncher),
    WeatherLauncher(WeatherLauncher),
    Empty,
}

#[derive(Clone, Debug)]
pub struct Launcher {
    pub name: Option<String>,
    pub alias: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub method: String,
    pub next_content: Option<String>,
    pub priority: u32,
    pub r#async: bool,
    pub home: bool,
    pub launcher_type: LauncherType,
    pub shortcut: bool,
    pub spawn_focus: bool,
    pub only_home: bool,
}

#[derive(Clone, Debug)]
pub struct ResultItem {
    pub priority: f32,
    pub row_item: SherlockRow,
    pub shortcut_holder: Option<Box>,
}

impl Launcher {
    // TODO: tile method recreates already stored data...
    pub fn get_patch(&self, keyword: &str) -> Vec<ResultItem> {
        match &self.launcher_type {
            LauncherType::App(app) => Tile::app_tile(self, keyword, &app.apps),
            LauncherType::Calc(calc) => Tile::calc_tile(self, &calc, keyword),
            LauncherType::CategoryLauncher(ctg) => Tile::app_tile(self, keyword, &ctg.categories),
            LauncherType::Clipboard((clp, calc)) => {
                Tile::clipboard_tile(self, &clp, &calc, keyword)
            }
            LauncherType::EventLauncher(evl) => Tile::event_tile(self, keyword, evl),
            LauncherType::ProcessLauncher(proc) => Tile::process_tile(self, keyword, &proc),
            LauncherType::SystemCommand(cmd) => Tile::app_tile(self, keyword, &cmd.commands),
            LauncherType::Web(web) => Tile::web_tile(self, keyword, &web),

            _ => Vec::new(),
        }
    }

    pub fn get_apps(&self) -> Option<&HashMap<String, AppData>> {
        match &self.launcher_type {
            LauncherType::App(app) => Some(&app.apps),
            _ => None,
        }
    }

    pub fn get_execs(&self) -> Option<HashSet<String>> {
        // NOTE: make a function to check for exec changes in the caching algorithm
        match &self.launcher_type {
            LauncherType::App(app) => {
                let execs: HashSet<String> =
                    app.apps.iter().map(|(_, v)| v.exec.to_string()).collect();
                Some(execs)
            }
            LauncherType::Web(web) => {
                let exec = format!("websearch-{}", web.engine);
                let execs: HashSet<String> = HashSet::from([(exec)]);
                Some(execs)
            }
            LauncherType::SystemCommand(cmd) => {
                let execs: HashSet<String> = cmd
                    .commands
                    .iter()
                    .map(|(_, v)| v.exec.to_string())
                    .collect();
                Some(execs)
            }
            LauncherType::CategoryLauncher(ctg) => {
                let execs: HashSet<String> = ctg
                    .categories
                    .iter()
                    .map(|(_, v)| v.exec.to_string())
                    .collect();
                Some(execs)
            }

            // None-Home Launchers
            LauncherType::Calc(_) => None,
            LauncherType::BulkText(_) => None,
            LauncherType::Clipboard(_) => None,
            LauncherType::EventLauncher(_) => None,
            _ => None,
        }
    }
    pub fn get_loader_widget(self, keyword: &str) -> Option<AsyncLauncherTile> {
        match self.launcher_type.clone() {
            LauncherType::BulkText(bulk_text) => {
                Tile::bulk_text_tile_loader(self, keyword, &bulk_text)
            }
            LauncherType::MusicPlayerLauncher(mpris) => Tile::mpris_tile(self, &mpris),
            LauncherType::WeatherLauncher(_) => Tile::weather_tile_loader(self),
            _ => None,
        }
    }
    pub async fn get_result(&self, keyword: &str) -> Option<(String, String, Option<String>)> {
        match &self.launcher_type {
            LauncherType::BulkText(bulk_text) => bulk_text.get_result(keyword).await,
            _ => None,
        }
    }
    pub async fn get_image(&self) -> Option<(gdk_pixbuf::Pixbuf, bool)> {
        match &self.launcher_type {
            LauncherType::MusicPlayerLauncher(mpis) => mpis.get_image().await,
            _ => None,
        }
    }
    pub async fn get_weather(&self) -> Option<(WeatherData, bool)> {
        match &self.launcher_type {
            LauncherType::WeatherLauncher(wtr) => wtr.get_result().await,
            _ => None,
        }
    }
}

pub fn construct_tiles(keyword: &str, launchers: &[Launcher], mode: &str) -> Vec<ResultItem> {
    let mut results = Vec::new();
    let sel_mode = mode.trim();
    for launcher in launchers.iter() {
        let alias = launcher.alias.as_deref().unwrap_or("all");

        if launcher.priority == 0 && alias != sel_mode {
            continue;
        }

        if alias == sel_mode || sel_mode == "all" {
            let result = launcher.get_patch(keyword);
            results.extend(result);
        }
    }
    results
}
