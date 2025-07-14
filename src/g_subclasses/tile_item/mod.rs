mod imp;

use std::{rc::Rc, usize};

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, WeakRef};
use glib::Object;
use gtk4::{glib, Widget};
use simd_json::prelude::Indexed;

use crate::launcher::LauncherType;
use crate::loader::util::ApplicationAction;
use crate::ui::tiles::app_tile::AppTileHandler;
use crate::ui::tiles::calc_tile::CalcTileHandler;
use crate::ui::tiles::pomodoro_tile::PomodoroTileHandler;
use crate::ui::tiles::weather_tile::WeatherTileHandler;
use crate::ui::tiles::web_tile::WebTileHandler;
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
        if let Some(parent) = parent {
            let weak = parent.downgrade();
            self.imp().parent.replace(weak);
        } else {
            self.imp().parent.take();
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

    pub fn get_patch(&self) -> Option<(Widget, UpdateHandler)> {
        let imp = self.imp();
        let launcher = imp.launcher.borrow();
        let index = imp.index.get();
        launcher.get_tile(index, launcher.clone(), self)
    }
    pub fn parent(&self) -> WeakRef<SherlockRow> {
        self.imp().parent.borrow().clone()
    }
    pub fn search(&self) -> Option<String> {
        self.get_by_key(|data| data.search_string.clone())
    }
    pub fn priority(&self) -> f32 {
        self.get_by_key(|data| data.priority)
            .unwrap_or(self.imp().launcher.borrow().priority as f32)
    }
    pub fn actions(&self) -> Vec<ApplicationAction> {
        let imp = self.imp();
        let launcher = imp.launcher.borrow();
        let mut actions = launcher.actions.clone().unwrap_or_default();
        actions.extend(imp.actions.borrow().clone());
        actions
    }
    pub fn based_show(&self, keyword: &str) -> bool {
        let imp = self.imp();
        match &*imp.update_handler.borrow() {
            UpdateHandler::AppTile(_) => false,
            UpdateHandler::Calculator(inner) => {
                let launcher = self.imp().launcher.borrow();
                if let LauncherType::Calc(clc) = &launcher.launcher_type {
                    inner.based_show(keyword, clc)
                } else {
                    false
                }
            }
            UpdateHandler::Pomodoro(_) => false,
            UpdateHandler::Weather(_) => false,
            UpdateHandler::WebTile(_) => false,
            UpdateHandler::Default => false,
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
                    return inner.update(keyword, launcher.clone());
                }
            }
            UpdateHandler::WebTile(inner) => {
                let launcher = imp.launcher.borrow();
                if let LauncherType::Web(web) = &launcher.launcher_type {
                    return inner.update(keyword, launcher.clone(), web);
                }
            }

            UpdateHandler::Pomodoro(_) | UpdateHandler::Default | UpdateHandler::Weather(_) => {}
        }
        Some(())
    }
    pub async fn update_async(&self, _keyword: &str) -> Option<()> {
        let imp = self.imp();
        match &*imp.update_handler.borrow() {
            UpdateHandler::Weather(wttr) => {
                let launcher = imp.launcher.borrow();
                let row = self.parent().upgrade()?;
                wttr.async_update(&row, launcher.clone()).await
            }
            _ => None,
        }
    }
    pub fn bind_signal(&self, row: &SherlockRow) {
        match &*self.imp().update_handler.borrow() {
            UpdateHandler::AppTile(inner) => inner.bind_signal(row),
            UpdateHandler::Calculator(inner) => inner.bind_signal(row),
            UpdateHandler::Pomodoro(inner) => {
                if let LauncherType::Pomodoro(pmd) = &self.imp().launcher.borrow().launcher_type {
                    inner.bind_signal(row, pmd)
                }
            }
            UpdateHandler::Weather(inner) => inner.bind_signal(row),
            UpdateHandler::WebTile(inner) => inner.bind_signal(row),
            UpdateHandler::Default => {}
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
    Calculator(CalcTileHandler),
    Pomodoro(PomodoroTileHandler),
    Weather(WeatherTileHandler),
    WebTile(WebTileHandler),
    Default,
}
impl Default for UpdateHandler {
    fn default() -> Self {
        Self::Default
    }
}
