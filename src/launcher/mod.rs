use std::collections::HashSet;

use gtk4::{Box, Label};

pub mod app_launcher;
pub mod audio_launcher;
pub mod bulk_text_launcher;
pub mod clipboard_launcher;
pub mod event_launcher;
pub mod system_cmd_launcher;
mod utils;
pub mod web_launcher;

use crate::{
    g_subclasses::sherlock_row::SherlockRow,
    ui::tiles::{util::AsyncOptions, Tile},
    CONFIG,
};

use app_launcher::App;
use audio_launcher::MusicPlayerLauncher;
use bulk_text_launcher::BulkText;
use clipboard_launcher::Clp;
use event_launcher::EventLauncher;
use system_cmd_launcher::SystemCommand;
use web_launcher::Web;

#[derive(Clone, Debug)]
pub enum LauncherType {
    App(App),
    Web(Web),
    Calc(()),
    BulkText(BulkText),
    SystemCommand(SystemCommand),
    Clipboard(Clp),
    EventLauncher(EventLauncher),
    MusicPlayerLauncher(MusicPlayerLauncher),
    Empty,
}

#[derive(Clone, Debug)]
pub struct Launcher {
    pub name: String,
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
        if let Some(app_config) = CONFIG.get() {
            match &self.launcher_type {
                LauncherType::App(app) => {
                    Tile::app_tile(self, keyword, app.apps.clone(), app_config)
                }
                LauncherType::Web(web) => Tile::web_tile(self, keyword, &web),
                LauncherType::Calc(_) => Tile::calc_tile(self, keyword, None),
                LauncherType::BulkText(bulk_text) => {
                    Tile::bulk_text_tile(&self, keyword, &bulk_text)
                }
                LauncherType::SystemCommand(cmd) => {
                    Tile::app_tile(self, keyword, cmd.commands.clone(), app_config)
                }
                LauncherType::Clipboard(clp) => {
                    Tile::clipboard_tile(self, &clp.clipboard_content, keyword)
                }
                LauncherType::EventLauncher(evl) => Tile::event_tile(self, keyword, evl),

                _ => Vec::new(),
            }
        } else {
            Vec::new()
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

            // None-Home Launchers
            LauncherType::Calc(_) => None,
            LauncherType::BulkText(_) => None,
            LauncherType::Clipboard(_) => None,
            LauncherType::EventLauncher(_) => None,
            _ => None,
        }
    }
    pub fn get_loader_widget(
        &self,
        keyword: &str,
    ) -> Option<(
        ResultItem,
        Option<Label>,
        Option<Label>,
        Option<AsyncOptions>,
        Box,
    )> {
        match &self.launcher_type {
            LauncherType::BulkText(bulk_text) => {
                Tile::bulk_text_tile_loader(&self, keyword, &bulk_text)
            }
            LauncherType::MusicPlayerLauncher(mpris) => Tile::mpris_tile(&self, &mpris),
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
    // results.sort_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap());
    // results.into_iter().map(|r| r.row_item).collect()
    results
}
