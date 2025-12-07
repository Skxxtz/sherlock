mod imp;

use gio::glib::object::Cast;
use gio::glib::{object::ObjectExt, WeakRef};
use glib::Object;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{glib, Box as GtkBox, Widget};
use simd_json::prelude::Indexed;
use std::{rc::Rc, usize};

use crate::g_subclasses::sherlock_row::SherlockRowBind;
use crate::launcher::LauncherType;
use crate::loader::util::{ApplicationAction, ExecVariable};
use crate::prelude::TileHandler;
use crate::ui::tiles::api_tile::ApiTileHandler;
use crate::ui::tiles::app_tile::AppTileHandler;
use crate::ui::tiles::calc_tile::CalcTileHandler;
use crate::ui::tiles::clipboard_tile::ClipboardHandler;
use crate::ui::tiles::event_tile::EventTileHandler;
use crate::ui::tiles::mpris_tile::MusicTileHandler;
use crate::ui::tiles::pipe_tile::PipeTileHandler;
use crate::ui::tiles::pomodoro_tile::PomodoroTileHandler;
use crate::ui::tiles::process_tile::ProcTileHandler;
use crate::ui::tiles::weather_tile::WeatherTileHandler;
use crate::ui::tiles::web_tile::WebTileHandler;
use crate::ui::tiles::Tile;
use crate::{g_subclasses::sherlock_row::SherlockRow, launcher::Launcher, loader::util::AppData};

glib::wrapper! {
    pub struct TileItem(ObjectSubclass<imp::TileItem>);
}

impl TileItem {
    pub fn set_index<T: TryInto<u16>>(&self, index: T) {
        self.imp().index.replace(index.try_into().ok());
    }
    pub fn set_launcher(&self, launcher: Rc<Launcher>) {
        self.imp().launcher.replace(launcher);
    }
    pub fn set_parent(&self, parent: Option<&SherlockRow>) {
        let imp = self.imp();
        if let Some(parent) = parent {
            if imp.active.get() {
                parent.add_css_class("multi-active");
            }
            let weak = parent.downgrade();
            imp.parent.replace(weak);
        } else {
            imp.parent.take();
        }
    }
    pub fn set_actions(&self, actions: Vec<ApplicationAction>) {
        *self.imp().actions.borrow_mut() = actions;
    }
    pub fn add_actions(&self, actions: &Option<Vec<ApplicationAction>>) {
        if let Some(actions) = actions {
            self.imp().actions.borrow_mut().extend(actions.clone());
        }
    }

    pub fn get_by_key<F, T>(&self, key: F) -> Option<T>
    where
        F: FnOnce(&AppData) -> T,
    {
        let imp = self.imp();
        let launcher = imp.launcher.borrow();
        let index = imp.index.get()?;
        let inner = launcher.inner()?;
        let data = inner.get(index as usize)?;
        Some(key(&data))
    }

    pub fn get_patch(&self) -> Option<Widget> {
        let imp = self.imp();
        let launcher = imp.launcher.borrow();
        let index = imp.index.get();
        match &launcher.launcher_type {
            // App Tile Based
            LauncherType::App(_)
            | LauncherType::Bookmark(_)
            | LauncherType::Category(_)
            | LauncherType::Command(_)
            | LauncherType::Emoji(_)
            | LauncherType::File(_)
            | LauncherType::Theme(_) => {
                // Get app data value
                let inner = launcher.inner()?;
                let value = inner.get(index? as usize)?;
                let tile = Tile::app(value, launcher.clone(), self);
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Api(api) => {
                let tile = Tile::api(launcher.clone(), &api);
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Clipboard(clp) => {
                if let Some((tile, handler)) = Tile::clipboard(launcher.clone(), &clp) {
                    self.imp().update_handler.replace(handler);
                    Some(tile)
                } else {
                    None
                }
            }
            LauncherType::Calc(_) => {
                let tile = Tile::calculator();
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Event(evt) => {
                let tile = Tile::event(evt)?;
                Some(tile.upcast::<Widget>())
            }
            LauncherType::MusicPlayer(_) => {
                let tile = Tile::mpris_tile();
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Pipe(pipe) => {
                let tile = Tile::pipe(launcher.clone(), pipe)?;
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Pomodoro(_) => {
                let tile = Tile::pomodoro(launcher.clone());
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Process(_) => {
                // Get app data value
                let inner = launcher.inner()?;
                let value = inner.get(index? as usize)?;
                let tile = Tile::process(value, launcher.clone(), self);
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Weather(_) => {
                let tile = Tile::weather();
                Some(tile.upcast::<Widget>())
            }
            LauncherType::Web(web) => {
                let tile = Tile::web(launcher.clone(), &web);
                Some(tile.upcast::<Widget>())
            }

            _ => None,
        }
    }
    pub fn binds(&self) -> Option<Vec<SherlockRowBind>> {
        self.imp().launcher.borrow().binds.clone()
    }
    pub fn parent(&self) -> WeakRef<SherlockRow> {
        self.imp().parent.borrow().clone()
    }
    pub fn search(&self) -> Option<String> {
        let imp = self.imp();
        let launcher = imp.launcher.borrow();

        if let LauncherType::Pipe(pipe) = &launcher.launcher_type {
            launcher
                .name
                .as_ref()
                .or(pipe.description.as_ref())
                .map(|s| match (&launcher.name, &pipe.description) {
                    (Some(name), Some(desc)) => format!("{};{}", name, desc),
                    _ => s.to_string(),
                })
        } else {
            let index = imp.index.get()?;
            let inner = launcher.inner()?;
            let data = inner.get(index as usize)?;
            Some(data.search_string.clone())
        }
    }

    pub fn priority(&self) -> f32 {
        self.get_by_key(|data| data.priority)
            .unwrap_or(self.imp().launcher.borrow().priority as f32)
    }
    pub fn is_async(&self) -> bool {
        self.imp().launcher.borrow().r#async
    }
    pub fn spawn_focus(&self) -> bool {
        self.imp().launcher.borrow().spawn_focus
    }
    pub fn num_actions(&self) -> usize {
        let imp = self.imp();
        if let Some(index) = imp.index.get() {
            imp.launcher
                .borrow()
                .inner()
                .and_then(|inner| inner.get(index as usize))
                .map_or(0, |val| val.actions.len())
        } else {
            imp.launcher
                .borrow()
                .actions
                .as_ref()
                .map_or(0, |a| a.len())
        }
    }
    pub fn actions(&self) -> Vec<ApplicationAction> {
        let imp = self.imp();
        let actions = if let Some(index) = imp.index.get() {
            imp.launcher
                .borrow()
                .inner()
                .and_then(|inner| inner.get(index as usize))
                .map(|val| val.actions.clone())
        } else {
            imp.launcher.borrow().actions.clone()
        };
        actions.unwrap_or_default()
    }
    pub fn variables(&self) -> Vec<ExecVariable> {
        let imp = self.imp();
        if let Some(index) = imp.index.get() {
            imp.launcher
                .borrow()
                .inner()
                .and_then(|inner| inner.get(index as usize))
                .map(|val| val.vars.clone())
                .unwrap_or_default()
        } else {
            vec![]
        }
    }
    pub fn alias(&self) -> String {
        self.imp()
            .launcher
            .borrow()
            .alias
            .clone()
            .unwrap_or_default()
    }
    pub fn terminal(&self) -> bool {
        let imp = self.imp();
        if let Some(index) = imp.index.get() {
            imp.launcher
                .borrow()
                .inner()
                .and_then(|inner| inner.get(index as usize))
                .map_or(false, |v| v.terminal)
        } else {
            false
        }
    }
    pub fn toggle_active(&self) {
        let imp = self.imp();
        let a = imp.active.get();
        if let Some(parent) = self.parent().upgrade() {
            if !a {
                parent.add_css_class("multi-active");
            } else {
                parent.remove_css_class("multi-active");
            }
        }
        imp.active.set(!a)
    }
    pub fn active(&self) -> bool {
        self.imp().active.get()
    }
    pub fn based_show(&self, keyword: &str) -> bool {
        let imp = self.imp();
        match &*imp.update_handler.borrow() {
            UpdateHandler::Calculator(inner) => {
                let launcher = self.imp().launcher.borrow();
                if let LauncherType::Calc(clc) = &launcher.launcher_type {
                    inner.based_show(keyword, &clc.capabilities)
                } else {
                    false
                }
            }

            UpdateHandler::AppTile(_)
            | UpdateHandler::Clipboard(_)
            | UpdateHandler::Event(_)
            | UpdateHandler::MusicPlayer(_)
            | UpdateHandler::Pipe(_)
            | UpdateHandler::Pomodoro(_)
            | UpdateHandler::Process(_)
            | UpdateHandler::Weather(_)
            | UpdateHandler::Default => false,

            UpdateHandler::ApiTile(_) | UpdateHandler::WebTile(_) => true,
        }
    }
    pub fn replace_tile(&self, tile: &Widget) {
        if let Ok(mut handler) = self.imp().update_handler.try_borrow_mut() {
            handler.replace_tile(tile);
        } else {
            eprintln!("Warning: Could not borrow update_handler mutably in replace_tile");
        }
    }
    pub fn update(&self, keyword: &str) -> Option<()> {
        let imp = self.imp();
        match &*imp.update_handler.borrow() {
            UpdateHandler::AppTile(app) => {
                let launcher = imp.launcher.borrow();
                let index = imp.index.get().unwrap();
                if let Some(inner) = launcher.inner() {
                    if let Some(value) = inner.get(index as usize) {
                        return app.update(keyword, launcher.clone(), value);
                    }
                }
            }
            UpdateHandler::Calculator(inner) => {
                let launcher = imp.launcher.borrow();
                if let LauncherType::Calc(_) = &launcher.launcher_type {
                    return inner.update(keyword);
                }
            }
            UpdateHandler::Process(proc) => {
                let launcher = imp.launcher.borrow();
                let index = imp.index.get().unwrap();
                if let Some(inner) = launcher.inner() {
                    if let Some(value) = inner.get(index as usize) {
                        return proc.update(keyword, launcher.clone(), value);
                    }
                }
            }
            UpdateHandler::Weather(inner) => {
                if let Some(parent) = self.parent().upgrade() {
                    let launcher = imp.launcher.borrow();
                    return inner.update(&parent, launcher.clone());
                }
            }
            UpdateHandler::WebTile(inner) => {
                let launcher = imp.launcher.borrow();
                if let LauncherType::Web(web) = &launcher.launcher_type {
                    return inner.update(keyword, launcher.clone(), web);
                }
            }

            _ => {}
        }
        Some(())
    }
    pub async fn update_async(&self, keyword: &str) -> Option<()> {
        let imp = self.imp();
        match &*imp.update_handler.borrow() {
            UpdateHandler::ApiTile(inner) => {
                let launcher = imp.launcher.borrow();
                let row = self.parent().upgrade()?;
                inner.update_async(keyword, launcher.clone(), &row).await
            }
            UpdateHandler::MusicPlayer(inner) => {
                let row = self.parent().upgrade()?;
                inner.update_async(&row).await
            }
            UpdateHandler::Weather(inner) => {
                let launcher = imp.launcher.borrow();
                let row = self.parent().upgrade()?;
                inner.async_update(&row, launcher.clone()).await
            }

            _ => None,
        }
    }
    pub fn change_attrs(&self, key: String, val: String) {
        match &*self.imp().update_handler.borrow() {
            UpdateHandler::ApiTile(inner) => inner.change_attrs(key, val),
            UpdateHandler::AppTile(inner) => inner.change_attrs(key, val),
            UpdateHandler::Calculator(inner) => inner.change_attrs(key, val),
            UpdateHandler::Clipboard(inner) => inner.change_attrs(key, val),
            UpdateHandler::Event(inner) => inner.change_attrs(key, val),
            UpdateHandler::MusicPlayer(inner) => inner.change_attrs(key, val),
            UpdateHandler::Pipe(inner) => inner.change_attrs(key, val),
            UpdateHandler::Process(inner) => inner.change_attrs(key, val),
            UpdateHandler::Weather(inner) => inner.change_attrs(key, val),
            UpdateHandler::WebTile(inner) => inner.change_attrs(key, val),
            UpdateHandler::Pomodoro(_) | UpdateHandler::Default => {}
        }
    }

    pub fn bind_signal(&self, row: &SherlockRow) {
        let launcher = self.imp().launcher.borrow().clone();
        match &*self.imp().update_handler.borrow() {
            UpdateHandler::ApiTile(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::AppTile(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::Calculator(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::Clipboard(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::Event(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::MusicPlayer(inner) => {
                if let LauncherType::MusicPlayer(mpris) =
                    &self.imp().launcher.borrow().launcher_type
                {
                    inner.bind_signal(row, mpris, launcher);
                }
            }
            UpdateHandler::Pipe(inner) => inner.bind_signal(row),
            UpdateHandler::Pomodoro(inner) => {
                if let LauncherType::Pomodoro(pmd) = &self.imp().launcher.borrow().launcher_type {
                    inner.bind_signal(row, pmd);
                }
            }
            UpdateHandler::Process(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::Weather(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::WebTile(inner) => inner.bind_signal(row, launcher),
            UpdateHandler::Default => {}
        }
    }
    pub fn shortcut(&self) -> Option<GtkBox> {
        if !self.imp().launcher.borrow().shortcut {
            return None;
        }

        match &*self.imp().update_handler.borrow() {
            UpdateHandler::AppTile(inner) => inner.shortcut(),
            UpdateHandler::Clipboard(inner) => inner.shortcut(),
            UpdateHandler::Event(inner) => inner.shortcut(),
            UpdateHandler::MusicPlayer(inner) => inner.shortcut(),
            UpdateHandler::Pipe(inner) => inner.shortcut(),
            UpdateHandler::Pomodoro(inner) => inner.shortcut(),
            UpdateHandler::Process(inner) => inner.shortcut(),
            UpdateHandler::WebTile(inner) => inner.shortcut(),

            UpdateHandler::ApiTile(_)
            | UpdateHandler::Calculator(_)
            | UpdateHandler::Weather(_)
            | UpdateHandler::Default => None,
        }
    }

    // Constructors
    pub fn from(launcher: Rc<Launcher>) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();

        imp.launcher.replace(launcher);
        obj
    }

    pub fn new() -> Self {
        Object::builder().build()
    }
}

#[derive(Debug)]
pub enum UpdateHandler {
    AppTile(AppTileHandler),
    ApiTile(ApiTileHandler),
    Calculator(CalcTileHandler),
    Clipboard(ClipboardHandler),
    Event(EventTileHandler),
    MusicPlayer(MusicTileHandler),
    Pipe(PipeTileHandler),
    Pomodoro(PomodoroTileHandler),
    Process(ProcTileHandler),
    Weather(WeatherTileHandler),
    WebTile(WebTileHandler),
    Default,
}
impl Default for UpdateHandler {
    fn default() -> Self {
        Self::Default
    }
}
impl UpdateHandler {
    pub fn replace_tile(&mut self, tile: &Widget) {
        match self {
            Self::AppTile(inner) => inner.replace_tile(tile),
            Self::ApiTile(inner) => inner.replace_tile(tile),
            Self::Calculator(inner) => inner.replace_tile(tile),
            Self::Clipboard(inner) => inner.replace_tile(tile),
            Self::Event(inner) => inner.replace_tile(tile),
            Self::MusicPlayer(inner) => inner.replace_tile(tile),
            Self::Pipe(inner) => inner.replace_tile(tile),
            Self::Pomodoro(inner) => inner.replace_tile(tile),
            Self::Process(inner) => inner.replace_tile(tile),
            Self::Weather(inner) => inner.replace_tile(tile),
            Self::WebTile(inner) => inner.replace_tile(tile),
            Self::Default => {}
        }
    }
}
