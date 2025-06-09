use std::{fmt::Display, sync::RwLock};

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::{
    glib::{object::ObjectExt, variant::ToVariant, WeakRef},
    ListStore,
};
use gtk4::{
    prelude::{EntryExt, GtkWindowExt, WidgetExt},
    Application, ApplicationWindow, Stack,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use simd_json::prelude::ArrayTrait;

use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    loader::{
        pipe_loader::{PipedData, PipedElements},
        util::JsonCache,
    },
    prelude::StackHelpers,
    sher_log,
    ui::{
        input_window::InputWindow,
        search::SearchUiObj,
        tiles::Tile,
        util::{display_raw, SearchHandler, SherlockAction, SherlockCounter},
    },
    utils::errors::SherlockError,
    CONFIG,
};

use super::call::ApiCall;

pub static RESPONSE_SOCKET: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

pub struct SherlockAPI {
    pub app: WeakRef<Application>,
    pub window: Option<WeakRef<ApplicationWindow>>,
    pub stack: Option<WeakRef<Stack>>,
    pub search_ui: Option<WeakRef<SearchUiObj>>,
    pub search_handler: Option<SearchHandler>,
    pub errors: Option<WeakRef<ListStore>>,
    pub queue: Vec<ApiCall>,
}
impl SherlockAPI {
    pub fn new(app: &Application) -> Self {
        Self {
            app: app.downgrade(),
            window: None,
            stack: None,
            search_ui: None,
            search_handler: None,
            errors: None,
            queue: vec![],
        }
    }

    /// Best use await_request() followed by flush() instead
    pub fn _request(&mut self, api_call: ApiCall) {
        self.flush();
        if self.match_action(&api_call).is_none() {
            self.queue.push(api_call);
        }
        if !self.queue.is_empty() {
            self.queue.iter().for_each(|wait| {
                sher_log!(format!("Action {} stays in queue", wait));
            });
        }
    }
    pub fn flush(&mut self) -> Option<()> {
        let mut queue = std::mem::take(&mut self.queue);
        self.queue = queue
            .drain(..)
            .filter(|api_call| self.match_action(api_call).is_none())
            .collect();
        Some(())
    }
    pub fn await_request(&mut self, request: ApiCall) -> Option<()> {
        self.queue.push(request);
        Some(())
    }

    pub fn match_action(&mut self, api_call: &ApiCall) -> Option<()> {
        match api_call {
            ApiCall::Obfuscate(vis) => self.obfuscate(*vis),
            ApiCall::Clear => self.clear_results(),
            ApiCall::SherlockError(err) => self.insert_msg(err),
            ApiCall::InputOnly => self.show_raw(),
            ApiCall::Show => self.open(),
            ApiCall::ClearAwaiting => self.flush(),
            ApiCall::Pipe(pipe) => self.load_pipe_elements(pipe),
            ApiCall::DisplayRaw(pipe) => self.display_raw(pipe),
            ApiCall::SwitchMode(mode) => self.switch_mode(mode),
            ApiCall::Socket(socket) => self.create_socket(socket.as_deref()),
        }
    }
    pub fn open(&self) -> Option<()> {
        let window = self.window.as_ref().and_then(|win| win.upgrade())?;
        let start_count = SherlockCounter::new()
            .and_then(|counter| counter.increment())
            .unwrap_or(0);

        let config = CONFIG.get()?;

        // parse sherlock actions
        let actions: Vec<SherlockAction> =
            JsonCache::read(&config.files.actions).unwrap_or_default();
        // activate sherlock actions
        actions
            .into_iter()
            .filter(|action| start_count % action.on == 0)
            .for_each(|action| {
                let attrs = get_attrs_map(vec![
                    ("method", Some(&action.action)),
                    ("exec", action.exec.as_deref()),
                ]);
                execute_from_attrs(&window, &attrs, None);
            });
        window.present();
        Some(())
    }
    pub fn obfuscate(&self, vis: bool) -> Option<()> {
        let ui = self.search_ui.as_ref().and_then(|ui| ui.upgrade())?;
        let imp = ui.imp();
        imp.search_bar.set_visibility(vis == false);
        Some(())
    }
    pub fn create_socket<T: AsRef<str>>(&self, socket: Option<T>) -> Option<()> {
        let addr = socket.map(|s| s.as_ref().to_string());
        let mut response = RESPONSE_SOCKET.write().unwrap();
        *response = addr;
        Some(())
    }
    pub fn clear_results(&self) -> Option<()> {
        let handler = self.search_handler.as_ref()?;
        if let Some(model) = handler.model.as_ref().and_then(|s| s.upgrade()) {
            model.remove_all();
        }
        Some(())
    }
    pub fn show_raw(&self) -> Option<()> {
        let ui = self.search_ui.as_ref().and_then(|ui| ui.upgrade())?;
        let imp = ui.imp();
        let handler = self.search_handler.as_ref()?;
        if let Some(model) = handler.model.as_ref().and_then(|s| s.upgrade()) {
            model.remove_all();
        }
        imp.mode_title.set_visible(false);
        imp.mode_title.unparent();
        imp.all.set_visible(false);
        imp.status_bar.set_visible(false);
        Some(())
    }
    pub fn display_pipe(&self, content: Vec<PipedElements>) -> Option<()> {
        let handler = self.search_handler.as_ref()?;
        let model = handler.model.as_ref().and_then(|s| s.upgrade())?;
        handler.clear();

        let data = Tile::pipe_data(&content, "print");
        data.into_iter().for_each(|elem| {
            model.append(&elem);
        });
        Some(())
    }
    pub fn insert_msg(&self, error: &SherlockError) -> Option<()> {
        let model = self.errors.as_ref().and_then(|tmp| tmp.upgrade())?;
        let (_, tiles) = Tile::error_tile(0, &vec![error], "⚠️", "WARNING");
        model.append(tiles.first()?);
        Some(())
    }

    fn load_pipe_elements<T: AsRef<[u8]>>(&mut self, msg: T) -> Option<()> {
        let elements = if let Some(elements) = PipedData::elements(&msg) {
            Some(elements)
        } else if let Some(elements) = PipedData::deserialize_pipe(&msg) {
            Some(elements)
        } else {
            None
        };
        if let Some(elements) = elements {
            self.display_pipe(elements);
            self.switch_page("search-page");
        }
        Some(())
    }
    fn display_raw<T: AsRef<str>>(&mut self, msg: T) -> Option<()> {
        let config = CONFIG.get()?;
        let stack = self.stack.as_ref().and_then(|tmp| tmp.upgrade())?;
        let message = msg.as_ref();

        let page = display_raw(message, config.runtime.center);
        stack.add_named(&page, Some("display-raw"));
        Some(())
    }
    fn switch_page<T: AsRef<str>>(&self, page: T) -> Option<()> {
        let stack = self.stack.as_ref().and_then(|tmp| tmp.upgrade())?;

        let page = page.as_ref();
        let current = stack.visible_child_name()?.to_string();
        let from_to = format!("{}->{}", current, page);

        let _ = stack.activate_action("win.switch-page", Some(&from_to.to_variant()));

        let retain = vec![
            String::from("search-page"),
            String::from("error-page"),
            page.to_string(),
        ];
        let all = stack.get_page_names();
        all.into_iter()
            .filter(|name| !retain.contains(&name))
            .filter_map(|name| stack.child_by_name(&name))
            .for_each(|child| stack.remove(&child));

        Some(())
    }

    fn search_view(&self) -> Option<()> {
        let handler = self.search_handler.as_ref()?;
        handler.populate();
        Some(())
    }

    fn spawn_input(&self, obfuscate: bool) -> Option<()> {
        let app = self.app.upgrade()?;
        let win = InputWindow::new(obfuscate);
        win.set_application(Some(&app));
        win.present();
        Some(())
    }
    pub fn switch_mode(&mut self, mode: &SherlockModes) -> Option<()> {
        match mode {
            SherlockModes::Search => {
                self.search_view()?;
                self.switch_page("search-page");
            }
            SherlockModes::Pipe(pipe) => {
                self.load_pipe_elements(pipe)?;
            }
            SherlockModes::DisplayRaw(pipe) => {
                self.display_raw(pipe)?;
                self.switch_page("display-raw");
            }
            SherlockModes::Error => {
                self.switch_page("error-page");
            }
            SherlockModes::Input(obfuscate) => {
                self.spawn_input(*obfuscate);
            }
        }
        Some(())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SherlockModes {
    Search,
    Error,
    DisplayRaw(String),
    Pipe(String),
    Input(bool),
}
impl Display for SherlockModes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "SearchView"),
            Self::Error => write!(f, "ErrorView"),
            Self::Pipe(_) => write!(f, "PipeView"),
            Self::DisplayRaw(_) => write!(f, "RawView"),
            Self::Input(obf) => write!(f, "Input:Obfuscated?{}", obf),
        }
    }
}
