use gpui::{AnyElement, SharedString};
use std::sync::Arc;

pub mod app_data;
pub mod calc_data;
pub mod clip_data;
pub mod emoji_data;
pub mod event_data;
pub mod file_data;
pub mod message;
pub mod mpris_data;
pub mod weather_data;

use crate::{
    app::theme::ThemeData,
    launcher::{
        ExecMode, Launcher, LauncherType, audio_launcher::AudioLauncherFunctions,
        children::message::MessageChild, emoji_launcher::EmojiData, utils::MprisState,
        weather_launcher::WeatherData,
    },
    loader::utils::{AppData, ExecVariable},
    ui::launcher::context_menu::ContextMenuAction,
    utils::config::HomeType,
};

use calc_data::CalcData;
use clip_data::ClipData;
use event_data::EventData;
use file_data::FileData;

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
                    $(Self::$variant { .. } => <$inner>::HANDLES_BODERS),*
                }
            }

            fn render(&self, selection: Selection, theme: Arc<ThemeData>) -> AnyElement {
                match self {
                    $(Self::$variant {inner, launcher} => inner.render(launcher, selection, theme)),*
                }
            }

            fn build_action_exec(&self, action: Arc<ContextMenuAction>) -> ExecMode {
                ExecMode::from_app_action(action, &self)
            }

            fn build_exec(&self) -> Option<ExecMode> {
                match self {
                    $(Self::$variant {launcher, inner} => inner.build_exec(launcher)),*
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

            fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
                match self {
                    $(Self::$variant {inner, ..} => inner.actions()),*
                }
            }

            fn has_actions(&self) -> bool {
                match self {
                    $(Self::$variant {inner, ..} => inner.has_actions()),*
                }
            }

            fn based_show(&self, keyword: &str) -> Option<bool> {
                match self {
                    $(Self::$variant {inner, ..} => inner.based_show(keyword)),*
                }
            }

            fn sidebar(&self, theme: Arc<ThemeData>) -> Option<AnyElement> {
                match self {
                    $(Self::$variant {inner, ..} => inner.sidebar(theme)),*
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
        }

        impl <'a> $name {
            #[inline(always)]
            fn launcher(&'a self) -> &'a Launcher {
                match self {
                    $(Self::$variant {launcher, ..} => &launcher),*
                }
            }
        }

    };
}
impl RenderableChild {
    pub async fn update_async(mut self) -> Option<Self> {
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
        FileLike(FileData),
        MusicLike(MprisState),
        WeatherLike(WeatherData),
        MessageLike(MessageChild),
        EventLike(EventData)
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
    fn handles_borders(&self) -> bool;
    fn render(&self, selection: Selection, theme: Arc<ThemeData>) -> AnyElement;
    fn build_action_exec(&'a self, action: Arc<ContextMenuAction>) -> ExecMode;
    fn build_exec(&self) -> Option<ExecMode>;
    fn search(&'a self) -> &'a str;
    fn vars(&self) -> Option<&[ExecVariable]>;
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>>;
    fn has_actions(&self) -> bool;
    fn based_show(&self, keyword: &str) -> Option<bool>;
    fn sidebar(&self, theme: Arc<ThemeData>) -> Option<AnyElement>;
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
}

pub trait RenderableChildImpl<'a> {
    /// If set to true, disables the inheritage of the border and background fill of the list item
    const HANDLES_BODERS: bool = false;
    fn render(
        &self,
        launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
    ) -> AnyElement;
    fn build_exec(&self, launcher: &Arc<Launcher>) -> Option<ExecMode>;
    fn priority(&self, launcher: &Arc<Launcher>) -> f32;
    fn search(&'a self, launcher: &Arc<Launcher>) -> &'a str;
    /// Will only get called once the context menu gets opened
    fn actions(&self) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        None
    }
    /// Whether the `additional actions` indicator should show in the status bar
    fn has_actions(&self) -> bool {
        false
    }
    fn based_show(&self, _keyword: &str) -> Option<bool> {
        None
    }
    fn sidebar(&self, _theme: Arc<ThemeData>) -> Option<AnyElement> {
        None
    }
}

pub trait SherlockSearch {
    /// Both self and substring should already be lowercased to increase performance
    fn fuzzy_match<'a>(&'a self, substring: &'a str) -> bool;
}

impl<T: AsRef<str>> SherlockSearch for T {
    fn fuzzy_match(&self, pattern: &str) -> bool {
        let t_bytes = self.as_ref().as_bytes();
        let p_bytes = pattern.as_bytes();

        if p_bytes.is_empty() {
            return true;
        }
        if t_bytes.len() < p_bytes.len() {
            return false;
        }

        let mut p_idx = 0;
        let p_len = p_bytes.len();

        for &byte in t_bytes {
            if byte.eq_ignore_ascii_case(&p_bytes[p_idx]) {
                p_idx += 1;
                if p_idx == p_len {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub data_idx: usize,
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
