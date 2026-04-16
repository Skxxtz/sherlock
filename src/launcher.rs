pub mod app_launcher;
pub mod audio_launcher;
pub mod bookmark_launcher;
pub mod bulk_text_launcher;
pub mod calc_launcher;
pub mod category_launcher;
pub mod clipboard_launcher;
pub mod emoji_launcher;
pub mod event_launcher;
pub mod file_launcher;
pub mod message_launcher;
pub mod system_cmd_launcher;
pub mod utils;
pub mod variant_type;
pub mod weather_launcher;
pub mod web_launcher;
// Integrate later: TODO
// pub mod pipe_launcher;
// pub mod pomodoro_launcher;
// pub mod process_launcher;
// pub mod theme_picker;

use crate::{
    launcher::variant_type::{InnerFunction, LauncherType},
    loader::{
        LoadContext, resolve_icon_path,
        utils::{AppData, RawLauncher},
    },
    sherlock_msg,
    ui::{
        launcher::{LauncherMode, context_menu::ContextMenuAction, views::NavigationViewType},
        widgets::{
            LauncherValues, RenderableChild, RenderableChildDelegate,
            emoji::{get_emoji, get_selected_skin_tones},
        },
    },
    utils::{
        config::HomeType,
        errors::{SherlockMessage, types::SherlockErrorType},
    },
};
use gpui::{App, AppContext, Keystroke, SharedString};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::Path, sync::Arc};

// Integrate later: TODO
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
        cx: &mut App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage>;
    fn binds(&self) -> Option<Arc<Vec<Bind>>> {
        None
    }
    fn execute_function<C: AppContext>(
        &self,
        func: InnerFunction,
        _child: &RenderableChild,
        _cx: &mut C,
    ) -> Result<bool, SherlockMessage> {
        Err(sherlock_msg!(
            Warning,
            SherlockErrorType::InvalidFunction,
            format!("{} does not provide function: {:?}", stringify!(self), func)
        ))
    }
}

#[derive(Debug, Clone)]
pub struct Bind {
    pub exit: bool,
    bind: Keystroke,
    callback: InnerFunction,
}
impl Bind {
    pub fn matches(&self, stroke: &Keystroke) -> bool {
        &self.bind == stroke
    }
    pub fn get_exec(&self) -> ExecMode {
        ExecMode::Inner {
            func: self.callback,
            exit: self.exit,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BindSerde {
    bind: String,
    callback: String,
    exit: bool,
}

impl BindSerde {
    pub fn get_bind(&self, func: InnerFunction) -> Option<Bind> {
        Some(Bind {
            bind: Keystroke::parse(&self.bind).ok()?,
            callback: func,
            exit: self.exit,
        })
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
#[derive(Debug, Default)]
pub struct Launcher {
    pub name: Option<String>,
    pub display_name: Option<SharedString>,
    pub icon: Option<Arc<Path>>,
    pub alias: Option<String>,
    pub on_return: Option<String>, // nu
    pub exit: bool,
    pub priority: u32,
    pub r#async: bool,
    pub home: HomeType,
    pub launcher_type: LauncherType,
    pub shortcut: bool,
    pub spawn_focus: bool,
    pub actions: Option<Arc<[Arc<ContextMenuAction>]>>,
    pub add_actions: Option<Arc<[Arc<ContextMenuAction>]>>,
}
impl Launcher {
    pub fn from_raw(raw: RawLauncher, launcher_type: LauncherType, icon: Option<String>) -> Self {
        Self {
            name: raw.name,
            display_name: raw.display_name.map(|n| SharedString::from(n)),
            icon: icon.as_deref().and_then(resolve_icon_path),
            alias: raw.alias,
            on_return: raw.on_return,
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
impl Display for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.display_name.as_ref() {
            return f.write_str(name);
        }

        if let Some(name) = self.name.as_ref() {
            return f.write_str(name);
        }

        f.write_str(&format!("{:?}", self.launcher_type))
    }
}

pub enum ExecMode {
    Inner {
        func: InnerFunction,
        exit: bool,
    },
    App {
        exec: String,
        terminal: bool,
    },
    Command {
        exec: String,
    },
    Category {
        category: LauncherMode,
    },
    CreateView {
        mode: NavigationViewType,
        launcher: Arc<Launcher>,
    },
    DynamicContextMenuFunc {
        action: Arc<ContextMenuAction>,
    },
    SwitchView {
        idx: usize,
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
            LauncherType::Apps(_) => Self::App {
                exec: app_data.exec.clone().unwrap_or_default(),
                terminal: app_data.terminal,
            },
            LauncherType::Bookmarks(bkm) => Self::Web {
                engine: None,
                browser: Some(bkm.target_browser.clone()),
                exec: app_data.exec.clone(),
            },
            LauncherType::Categories(_) => Self::Category {
                category: LauncherMode::Alias {
                    short: app_data
                        .exec
                        .as_ref()
                        .map(SharedString::from)
                        .unwrap_or_default(),
                    name: app_data.name.clone().unwrap_or_default(),
                },
            },
            LauncherType::Commands(_) => Self::Command {
                exec: app_data.exec.clone().unwrap_or_default(),
            },
            LauncherType::Emoji(_) => Self::CreateView {
                mode: NavigationViewType::Emoji,
                launcher: Arc::clone(launcher),
            },
            LauncherType::Files(_) => Self::CreateView {
                mode: NavigationViewType::Files { dir: None },
                launcher: Arc::clone(&launcher),
            },
            LauncherType::Message(_) => Self::SwitchView { idx: 0 },
            LauncherType::Web(web) => Self::Web {
                engine: Some(web.engine.clone()),
                browser: web.browser.clone(),
                exec: app_data.exec.clone(),
            },
            _ => Self::None,
        }
    }
    pub fn from_child(data: &RenderableChild) -> Option<Self> {
        let launcher_snapshot = data.with_launcher(|l| l.clone());

        if let Some(on_return) = launcher_snapshot.on_return.as_ref() {
            match on_return.as_str() {
                "app_launcher" | "command" => {
                    if let Some(exec) = data.get_exec() {
                        return Some(Self::Command {
                            exec: exec.to_string(),
                        });
                    }
                }
                "create_bookmark" => {
                    if let RenderableChild::AppLike { launcher, inner } = data {
                        if matches!(launcher.launcher_type, LauncherType::Clipboard(_)) {
                            if let (Some(exec), Some(name)) = (&inner.exec, &inner.name) {
                                return Some(Self::CreateBookmark {
                                    url: exec.to_string(),
                                    name: name.to_string(),
                                });
                            }
                        }
                    }
                }

                k if k.starts_with("inner.") => {
                    let inner = InnerFunction::from_str(
                        data.launcher_type(),
                        k.trim_start_matches("inner."),
                    );
                    if inner != InnerFunction::Empty {
                        return Some(Self::Inner {
                            func: inner,
                            exit: launcher_snapshot.exit,
                        });
                    }
                }
                _ => {}
            };
        }

        data.build_exec()
    }
    pub fn from_app_action(action: Arc<ContextMenuAction>, data: &RenderableChild) -> Self {
        match action.as_ref() {
            ContextMenuAction::App(action) => match action.method.as_str() {
                "app_launcher" | "command" => Self::Command {
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

                "web_launcher" => Self::Web {
                    engine: None,
                    browser: None,
                    exec: action.exec.clone(),
                },

                k if k.starts_with("inner.") => {
                    let inner = InnerFunction::from_str(
                        data.launcher_type(),
                        k.trim_start_matches("inner."),
                    );
                    if inner == InnerFunction::Empty {
                        Self::None
                    } else {
                        Self::Inner {
                            func: inner,
                            exit: action.exit,
                        }
                    }
                }
                _ => Self::None,
            },
            ContextMenuAction::Fn(_) => Self::DynamicContextMenuFunc { action },
            ContextMenuAction::Emoji(emj) => {
                if let Some(entry) = emj.entry() {
                    let content = get_emoji(entry, &get_selected_skin_tones())
                        .as_str()
                        .to_string();
                    Self::Copy { content }
                } else {
                    Self::None
                }
            }
        }
    }
}
