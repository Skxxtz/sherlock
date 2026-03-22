use gpui::{AnyElement, SharedString};
use std::sync::Arc;

pub mod app_data;
pub mod calc_data;
pub mod clip_data;
pub mod mpris_data;
pub mod weather_data;

use crate::{
    launcher::{
        ExecMode, Launcher, LauncherType, audio_launcher::AudioLauncherFunctions,
        utils::MprisState, weather_launcher::WeatherData,
    },
    loader::utils::{AppData, ApplicationAction, ExecVariable},
    utils::config::HomeType,
};

use calc_data::CalcData;
use clip_data::ClipData;

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

        impl<'a> RenderableChildDelegate<'a> for $name {
            fn render(&self, is_selected: bool) -> AnyElement {
                match self {
                    $(Self::$variant {inner, launcher} => inner.render(launcher, is_selected)),*
                }
            }

            fn build_action_exec(&self, action: &ApplicationAction) -> ExecMode {
                match self {
                    $(Self::$variant {launcher, ..} => { ExecMode::from_app_action(action, launcher) }),*
                }
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

            fn actions(&self) -> Option<Arc<[Arc<ApplicationAction>]>> {
                match self {
                    Self::AppLike { inner, ..} => Some(inner.actions.clone()),
                    _ => None
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

            fn launcher_type(&'a self) -> &'a LauncherType {
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
    pub fn based_show(&self, query: &str) -> Option<bool> {
        match self {
            Self::ClipLike { inner, .. } => Some(inner.based_show()),
            Self::CalcLike { inner, .. } => Some(inner.based_show(query)),
            Self::MusicLike { inner, .. } => {
                // this skips early if the music launcher is empty
                if inner.raw.is_some() {
                    return None;
                } else {
                    Some(false)
                }
            }
            _ => None,
        }
    }
    pub async fn update_async(mut self) -> Option<Self> {
        match &mut self {
            Self::ClipLike { inner, .. } => {
                inner.update_async();
            }
            Self::MusicLike { inner, .. } => {
                let new_inner = AudioLauncherFunctions::new().and_then(|launcher| {
                    launcher
                        .get_current_player()
                        .and_then(|player| launcher.get_metadata(&player))
                });

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
        MusicLike(MprisState),
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
    fn render(&self, is_selected: bool) -> AnyElement;
    fn build_action_exec(&'a self, action: &'a ApplicationAction) -> ExecMode;
    fn build_exec(&self) -> Option<ExecMode>;
    fn search(&'a self) -> &'a str;
    fn vars(&self) -> Option<&[ExecVariable]>;
    fn actions(&self) -> Option<Arc<[Arc<ApplicationAction>]>>;
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
    fn render(&self, launcher: &Arc<Launcher>, is_selected: bool) -> AnyElement;
    fn build_exec(&self, launcher: &Arc<Launcher>) -> Option<ExecMode>;
    fn priority(&self, launcher: &Arc<Launcher>) -> f32;
    fn search(&'a self, launcher: &Arc<Launcher>) -> &'a str;
}

pub trait SherlockSearch {
    /// Both self and substring should already be lowercased to increase performance
    fn fuzzy_match<'a>(&'a self, substring: &'a str) -> bool;
}

impl<T: AsRef<str>> SherlockSearch for T {
    fn fuzzy_match(&self, pattern: &str) -> bool {
        let t_bytes = self.as_ref().as_bytes();
        let p_bytes = pattern.as_bytes();

        // Early return for empty bytes
        if p_bytes.is_empty() {
            return true;
        }
        if t_bytes.is_empty() {
            return false;
        }

        let mut current_target = t_bytes;

        // memchr find first search byte
        while let Some(pos) = memchr::memchr(p_bytes[0], current_target) {
            if sequential_check(p_bytes, &current_target[pos..], 5) {
                return true;
            }
            // Move past the current match to find the next possible start
            if pos + 1 >= current_target.len() {
                break;
            }
            current_target = &current_target[pos + 1..];
        }

        false
    }
}

fn sequential_check(pattern: &[u8], target: &[u8], window_size: usize) -> bool {
    // pattern[0] was already matched by memchr at target[0]
    let mut t_idx = 1;

    // We start from the second character (index 1)
    for &pattern_char in &pattern[1..] {
        // The window starts at t_idx and ends at t_idx + window_size
        let limit = std::cmp::min(t_idx + window_size, target.len());
        let mut found = false;

        while t_idx < limit {
            if target[t_idx] == pattern_char {
                t_idx += 1; // Start searching for the NEXT char from here
                found = true;
                break;
            }
            t_idx += 1;
        }

        // If the inner loop finishes without finding the char, the chain is broken
        if !found {
            return false;
        }
    }

    true
}
