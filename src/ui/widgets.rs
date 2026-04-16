use gpui::{AnyElement, App, AsyncApp, SharedString};
use std::sync::Arc;

pub mod app;
pub mod calculator;
pub mod clipboard;
pub mod emoji;
pub mod event;
pub mod file;
pub mod message;
pub mod mpris;
pub mod script;
pub mod weather;

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher, audio_launcher::AudioLauncherFunctions, emoji_launcher::EmojiData,
        utils::MprisState, variant_type::LauncherType, weather_launcher::WeatherData,
    },
    loader::utils::{AppData, ExecVariable},
    ui::{
        launcher::context_menu::ContextMenuAction,
        widgets::{message::MessageChild, script::ScriptData},
    },
    utils::config::HomeType,
};

use calculator::CalcData;
use clipboard::ClipData;
use event::EventData;
use file::FileData;

/// Creates enum RenderableChild,
/// ## Example:
/// ```
/// renderable_enum! {
///     enum RenderableChild {
///         AppLike(AppData),
///         WeatherLike(WeatherData),
///     }
/// }
/// ```
macro_rules! renderable_enum {
    (
        enum $name:ident {
            $($variant:ident($inner:ty)),* $(,)?
        }
    ) => {
        #[derive(Clone)]
        pub enum $name {
            $(
                $variant {
                    launcher: Arc<Launcher>,
                    inner: $inner,
                }
            ),*
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$variant { .. } => write!(f, "{}", stringify!($variant)),
                    )*
                }
            }
        }

        impl<'a> RenderableChildDelegate<'a> for $name {
            fn handles_borders(&self) -> bool {
                match self {
                    $(Self::$variant { .. } => <$inner>::HANDLES_BORDERS),*
                }
            }

            fn render(&self, selection: Selection, theme: Arc<ThemeData>, cx: &mut App) -> AnyElement {
                match self {
                    $(Self::$variant {inner, launcher} => inner.render(launcher, selection, theme, cx)),*
                }
            }

            fn build_action_exec(&self, action: Arc<ContextMenuAction>) -> ExecMode {
                ExecMode::from_app_action(action, &self)
            }

            fn build_exec(&self) -> Option<ExecMode> {
                match self {
                    $(Self::$variant {launcher, inner} => {
                        inner.build_exec(launcher)
                    }),*
                }
            }

            fn search(&'a self) -> &'a str {
                match self {
                    $(Self::$variant {inner, launcher} => inner.search(launcher)),*
                }
            }

            fn vars(&self) -> Option<&[ExecVariable]> {
                match self {
                    Self::AppLike { inner, .. } => Some(&inner.vars), // Works for Vec or SmallVec
                    _ => None,
                }
            }

            fn actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
                match self {
                    $(Self::$variant {inner, launcher} => inner.actions(launcher, cx)),*
                }
            }

            fn has_actions(&self, cx: &mut App) -> bool {
                match self {
                    $(Self::$variant {inner, launcher} => {
                        if launcher.actions.as_ref().map_or(false, |actions| !actions.is_empty()) {
                            return true
                        }
                        if launcher.add_actions.as_ref().map_or(false, |actions| !actions.is_empty()) {
                            return true
                        }
                        inner.has_actions(cx)
                    }),*
                }
            }

            fn based_show(&self, keyword: &str) -> Option<bool> {
                match self {
                    $(Self::$variant {inner, ..} => inner.based_show(keyword)),*
                }
            }

            fn sidebar(&self, cx: &mut App) -> Option<AnyElement> {
                match self {
                    $(Self::$variant {inner, ..} => inner.sidebar(cx)),*
                }
            }

            fn update_sync(&self, query: SharedString, cx: &mut App) {
                match self {
                    $(Self::$variant {inner, launcher} => inner.update_sync(query, launcher, cx)),*
                }
            }
        }

        impl<'a> LauncherValues<'a> for $name {
            fn name(&'a self) -> Option<&'a str> {
                self.launcher().name.as_deref()
            }

            fn display_name(&self) -> Option<SharedString> {
                self.launcher().display_name.clone()
            }

            fn home(&self) -> HomeType {
                self.launcher().home
            }

            fn is_async(&self) -> bool {
                self.launcher().r#async
            }

            fn alias(&'a self) -> Option<&'a str> {
                self.launcher().alias.as_deref()
            }

            fn priority(&self) -> f32 {
                match self {
                    $(Self::$variant {inner, launcher} => inner.priority(launcher)),*
                }
            }

            fn spawn_focus(&self) -> bool {
                match self {
                    $(Self::$variant {launcher, ..} => launcher.spawn_focus),*
                }
            }

            fn launcher_type(&self) -> &LauncherType {
                &self.launcher().launcher_type
            }

            fn shortcut(&self) -> bool {
                match self {
                    $(Self::$variant {launcher, ..} => launcher.shortcut),*
                }
            }
        }

        impl <'a> $name {
            #[inline(always)]
            fn launcher(&'a self) -> &'a Launcher {
                match self {
                    $(Self::$variant {launcher, ..} => &launcher),*
                }
            }

            pub fn with_launcher<F, R>(&self, f: F) -> R
            where
                F: FnOnce(&Arc<Launcher>) -> R
            {
                match self {
                    $(Self::$variant { launcher, .. } => f(launcher)),*
                }
            }
        }

    };
}
impl RenderableChild {
    /// Updates a dynamic renderable child that requires re-evaluation.
    ///
    /// This is used for items whose state depends on internal logic (e.g., a timer)
    /// or external factors (e.g., a weather API or file system change).
    ///
    /// # Returns
    ///
    /// * `Some(Self)` - If the state was updated and a re-render is required.
    /// * `None` - If no changes were detected, allowing the UI to skip an update cycle.
    pub async fn update_async(mut self, _cx: &mut AsyncApp) -> Option<Self> {
        match &mut self {
            Self::ClipLike { inner, .. } => {
                inner.update_async();
            }
            Self::EventLike { inner, .. } => {
                let _ = inner.update_async().await;
            }
            Self::MusicLike { inner, .. } => {
                let launcher = AudioLauncherFunctions::new()?;
                inner.player = launcher.get_current_player();

                // id player is none, nothing is playing...
                if inner.player.is_none() {
                    inner.raw = None;
                    inner.image = None;
                    return Some(self);
                }

                let new_inner = launcher.get_metadata(inner.player.as_ref()?);

                // early return if nothing has changed
                if new_inner.as_ref().and_then(|i| i.metadata.title.as_ref())
                    == inner.raw.as_ref().and_then(|i| i.metadata.title.as_ref())
                {
                    return None;
                }

                if let Some(new_inner) = &new_inner {
                    inner.image = new_inner.get_image().await.map(|(image, _)| image);
                }
                inner.raw = new_inner;
            }
            Self::WeatherLike { inner, launcher } => {
                let LauncherType::Weather(wtr) = &launcher.launcher_type else {
                    unreachable!("WeatherLike variant must have LauncherType::Weather");
                };

                let (new_weather_data, changed) = WeatherData::fetch_async(wtr).await?;

                if changed {
                    *inner = new_weather_data;
                } else {
                    return None;
                }
            }
            _ => return None,
        }

        Some(self)
    }
}
renderable_enum! {
    enum RenderableChild {
        AppLike(AppData),
        CalcLike(CalcData),
        ClipLike(ClipData),
        EmojiLike(EmojiData),
        EventLike(EventData),
        FileLike(FileData),
        MessageLike(MessageChild),
        MusicLike(MprisState),
        ScriptLike(ScriptData),
        WeatherLike(WeatherData),
    }
}

impl RenderableChild {
    pub fn get_exec(&self) -> Option<String> {
        match self {
            Self::AppLike { inner, launcher } => inner.get_exec(launcher),
            _ => None,
        }
    }
}

pub trait RenderableChildDelegate<'a> {
    /// Whether the child internally applies style for borders
    fn handles_borders(&self) -> bool;

    /// The logic to render the widget
    fn render(&self, selection: Selection, theme: Arc<ThemeData>, cx: &mut App) -> AnyElement;

    /// Generates an execution path based on the child and the context menu action
    fn build_action_exec(&'a self, action: Arc<ContextMenuAction>) -> ExecMode;

    /// Generates an execution path when pressing return on this widget
    fn build_exec(&self) -> Option<ExecMode>;

    /// The string that contains or otherwise matces the user-provided search query
    fn search(&'a self) -> &'a str;

    /// The variable fields that should be shown next to the search input
    fn vars(&self) -> Option<&[ExecVariable]>;

    /// The context menu actions for this widget. (Gets called on the selected item only if:
    /// self.has_actions == true and the context menu gets opened)
    fn actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>>;

    /// Whether this widget owns any context menu actions. (This gets called only on the selected
    /// item)
    fn has_actions(&self, cx: &mut App) -> bool;

    /// Boolean logic for conditional display (e.g., calculator)
    fn based_show(&self, keyword: &str) -> Option<bool>;

    /// Sidebar rendering
    fn sidebar(&self, cx: &mut App) -> Option<AnyElement>;

    /// Sync update on every keypress
    fn update_sync(&self, query: SharedString, cx: &mut App);
}

#[allow(dead_code)]
pub trait LauncherValues<'a> {
    fn name(&'a self) -> Option<&'a str>;
    fn display_name(&self) -> Option<SharedString>;
    fn alias(&'a self) -> Option<&'a str>;
    fn priority(&self) -> f32;
    fn is_async(&self) -> bool;
    fn home(&self) -> HomeType;
    fn spawn_focus(&self) -> bool;
    fn launcher_type(&'a self) -> &'a LauncherType;
    fn shortcut(&self) -> bool;
}

pub trait RenderableChildImpl<'a> {
    /// If set to true, disables the inheritage of the border and background fill of the list item
    const HANDLES_BORDERS: bool = false;
    fn render(
        &self,
        launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> AnyElement;
    fn build_exec(&self, launcher: &Arc<Launcher>) -> Option<ExecMode>;
    fn priority(&self, launcher: &Arc<Launcher>) -> f32;
    fn search(&'a self, launcher: &Arc<Launcher>) -> &'a str;
    /// Will only get called once the context menu gets opened
    fn actions(
        &self,
        launcher: &Arc<Launcher>,
        _cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        if let Some(actions) = launcher.actions.as_ref().cloned() {
            return Some(actions.into());
        }
        None
    }
    /// Whether the `additional actions` indicator should show in the status bar
    fn has_actions(&self, _cx: &mut App) -> bool {
        false
    }
    fn based_show(&self, _keyword: &str) -> Option<bool> {
        None
    }
    fn sidebar(&self, _cx: &mut App) -> Option<AnyElement> {
        None
    }
    fn update_sync(&self, _query: SharedString, _launcher: &Arc<Launcher>, _cx: &mut App) {}
}

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    /// The unique index of the item
    pub data_idx: usize,

    /// Whether the current item is selected by the user
    pub is_selected: bool,
}

impl Selection {
    #[inline(always)]
    pub fn new(data_idx: usize, is_selected: bool) -> Self {
        Self {
            data_idx,
            is_selected,
        }
    }
}
