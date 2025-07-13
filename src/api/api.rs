use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::{
    glib::{object::ObjectExt, variant::ToVariant, MainContext, WeakRef},
    ListStore,
};
use gtk4::{
    prelude::{EntryExt, GtkWindowExt, WidgetExt},
    Application, ApplicationWindow, Stack,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use simd_json::prelude::ArrayTrait;
use std::{fmt::Display, sync::RwLock, time::Instant};

use crate::{
    actions::{commandlaunch::command_launch, execute_from_attrs, get_attrs_map},
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
    utils::{config::ConfigGuard, errors::SherlockError},
};

use super::call::ApiCall;

pub static RESPONSE_SOCKET: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

pub struct SherlockAPI {
    pub app: WeakRef<Application>,
    pub window: Option<WeakRef<ApplicationWindow>>,
    pub open_window: Option<WeakRef<ApplicationWindow>>,
    pub stack: Option<WeakRef<Stack>>,
    pub search_ui: Option<WeakRef<SearchUiObj>>,
    pub search_handler: Option<SearchHandler>,
    pub errors: Option<WeakRef<ListStore>>,
    pub queue: Vec<ApiCall>,
    pub shutdown_queue: Vec<ApiCall>,
}
impl SherlockAPI {
    pub fn new(app: &Application) -> Self {
        Self {
            app: app.downgrade(),
            window: None,
            open_window: None,
            stack: None,
            search_ui: None,
            search_handler: None,
            errors: None,
            queue: vec![],
            shutdown_queue: vec![],
        }
    }

    /// Best use await_request() followed by flush() instead
    pub fn request(&mut self, api_call: ApiCall) {
        self.flush();
        if self.match_action(&api_call).is_none() {
            let _ = sher_log!(format!(
                "Action {} could not be executed and is moved to queue",
                api_call
            ));
            self.queue.push(api_call);
        }
        if !self.queue.is_empty() {
            self.queue.iter().for_each(|wait| {
                let _ = sher_log!(format!("Action {} stays in queue", wait));
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
            ApiCall::SherlockError(err) => self.insert_msg(err, true),
            ApiCall::SherlockWarning(err) => self.insert_msg(err, false),
            ApiCall::InputOnly => self.show_raw(),
            ApiCall::Show(submenu) => self.open(submenu),
            ApiCall::Close => self.close(),
            ApiCall::ClearAwaiting => self.flush(),
            ApiCall::Pipe(pipe) => self.load_pipe_elements(pipe),
            ApiCall::DisplayRaw(pipe) => self.display_raw(pipe),
            ApiCall::SwitchMode(mode) => self.switch_mode(mode),
            ApiCall::Socket(socket) => self.create_socket(socket.as_deref()),
            ApiCall::Method(meth) => self.call_method(meth),
        }
    }
    pub fn close(&mut self) -> Option<()> {
        let calls: Vec<ApiCall> = self.shutdown_queue.drain(..).collect();
        for call in calls {
            if let ApiCall::Method(x) = call {
                self.call_method(&x);
            }
        }
        let window = self.window.as_ref().and_then(|win| win.upgrade())?;
        let _ = window.activate_action("win.close", None);
        Some(())
    }
    pub fn open(&mut self, submenu: &str) -> Option<()> {
        let window = self.window.as_ref().and_then(|win| win.upgrade())?;
        let open_window = self.open_window.as_ref().and_then(|win| win.upgrade())?;
        let start_count = SherlockCounter::new()
            .and_then(|counter| counter.increment())
            .unwrap_or(0);

        // Switch mode to specified and assign config runtime parameter
        if let Some(ui) = self.search_ui.as_ref().and_then(|s| s.upgrade()) {
            let bar = &ui.imp().search_bar;
            if let Ok(_) = bar.activate_action("win.switch-mode", Some(&submenu.to_variant())) {
                let _ = ConfigGuard::write_key(|c| c.behavior.sub_menu = Some(submenu.to_string()));
            }
            let _ = bar.activate_action("win.update-items", Some(&false.to_variant()));
        }
        // parse sherlock actions
        let config = ConfigGuard::read().ok()?;
        let mut actions: Vec<SherlockAction> =
            JsonCache::read(&config.files.actions).unwrap_or_default();

        // activate sherlock actions
        let pos = actions
            .iter()
            .position(|action| action.exec.as_deref() == Some("restart"));
        if let Some(pos) = pos {
            let removed = actions.remove(pos);
            if removed.on > 2 && start_count % removed.on == 0 {
                let call = ApiCall::Method("restart".to_string());
                self.shutdown_queue.push(call);
            }
        }

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

        open_window.present();
        Some(())
    }
    pub fn call_method(&self, method: &str) -> Option<()> {
        match method {
            "restart" => {
                if let Ok(config) = ConfigGuard::read() {
                    if config.behavior.daemonize {
                        if let Err(err) = command_launch("sherlock --take-over --daemonize", "") {
                            let _result = err.insert(true);
                        }
                    }
                }
            }
            _ => {}
        }
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
    pub fn insert_msg(&self, error: &SherlockError, is_error: bool) -> Option<()> {
        let (icon, msg) = if is_error {
            ("", "")
        } else {
            ("⚠️", "WARNING")
        };
        let model = self.errors.as_ref().and_then(|tmp| tmp.upgrade())?;
        let (_, tiles) = Tile::error_tile(0, &vec![error], icon, msg);
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
        let config = ConfigGuard::read().ok()?;
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
        let t0 = Instant::now();
        let handler = self.search_handler.as_ref()?.clone();

        MainContext::default().spawn_local(async move {
            let start = Instant::now();

            let _ = handler.populate().await;

            let duration = start.elapsed();
            if let Ok(timing_enabled) = std::env::var("TIMING") {
                if timing_enabled == "true" {
                    println!("Popuate {:?}", duration);
                }
            }
        });

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

// POSSIBLE SOLUTION FOR API CALL DISPATCHER
// use std::{sync::{Mutex, Arc}, collections::HashMap};
// use serde_json::Value;
// type CommandHandler = Box<dyn Fn(Value) + Send + Sync>;
// struct ApiFunctionispatcher {
//     handlers: HashMap<String, CommandHandler>,
// }
// impl ApiFunctionispatcher {
//     fn new() -> Self {
//         Self { handlers: HashMap::new() }
//     }
//     fn register<F>(&mut self, name: &str, handler: F)
//         where F: Fn(Value) + Send + Sync + 'static,
//     {
//         self.handlers.insert(name.to_string(), Box::new(handler));
//     }
//     fn execute(&self, name: &str, args: &str) {
//         match serde_json::from_str::<Value>(args){
//             Ok(val) => {
//                 if let Some(func) = self.handlers.get(name){
//                     func(val)
//                 }
//             },
//             _ => {}
//         }

//     }
// }
// pub static DISPATCHER: Lazy<Arc<Mutex<ApiFunctionispatcher>>> = Lazy::new(|| {
//     Arc::new(Mutex::new(ApiFunctionispatcher::new()))
// });
