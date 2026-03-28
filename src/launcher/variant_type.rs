use std::sync::Arc;

use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    launcher::{
        Bind, LauncherProvider,
        app_launcher::AppLauncher,
        audio_launcher::{MusicPlayerFunctions, MusicPlayerLauncher},
        bookmark_launcher::BookmarkLauncher,
        calc_launcher::CalculatorLauncher,
        category_launcher::CategoryLauncher,
        children::RenderableChild,
        clipboard_launcher::ClipboardLauncher,
        emoji_launcher::EmojiPicker,
        event_launcher::{EventLauncher, EventLauncherFunctions},
        message_launcher::MessageLauncher,
        system_cmd_launcher::CommandLauncher,
        weather_launcher::WeatherLauncher,
        web_launcher::WebLauncher,
    },
    loader::utils::RawLauncher,
    utils::errors::SherlockMessage,
};

macro_rules! create_variants {
    (
        enum $name:ident {
            $( $variant:ident( $inner:ty $(, $extra:ty)* ) ),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug, Default)]
        pub enum $name {
            $($variant($inner),)*
            #[default]
            Empty,
        }

        #[derive(Deserialize, Debug, Serialize, Clone, Copy, Default, Display, PartialEq)]
        #[serde(rename_all = "snake_case")]
        #[strum(serialize_all = "snake_case")]
        pub enum LauncherVariant {
            $($variant,)*
            #[default]
            Empty,
        }

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum InnerFunction {
            $(
                $( $variant($extra), )?
            )*
            #[allow(dead_code)]
            Empty
        }

        impl InnerFunction {
            pub fn from_str(variant: &$name, func_name: &str) -> Self {
                match variant {
                    $(
                        $name::$variant(_) => {
                            $(
                                use std::str::FromStr;
                                if let Ok(f) = <$extra>::from_str(func_name) {
                                    return Self::$variant(f);
                                }
                            )?
                            Self::Empty
                        }
                    )*
                    $name::Empty => Self::Empty,
                }
            }
        }

        impl $name {
            pub fn get_render_obj(
                &self,
                launcher: std::sync::Arc<crate::launcher::Launcher>,
                ctx: &crate::loader::LoadContext,
                opts: std::sync::Arc<serde_json::Value>,
            ) -> Result<Vec<RenderableChild>, SherlockMessage> {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as LauncherProvider>::objects(inner, launcher, ctx, opts),
                    )*
                    Self::Empty => Ok(vec![]),
                }
            }
            pub fn binds(&self) -> Option<Arc<Vec<Bind>>> {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as LauncherProvider>::binds(inner),
                    )*
                    Self::Empty => None
                }
            }
            pub fn execute_function(&self, func: InnerFunction, child: &RenderableChild) -> Result<bool, SherlockMessage> {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as LauncherProvider>::execute_function(inner, func, child),
                    )*
                    Self::Empty => unimplemented!(),
                }
            }
        }

        impl LauncherVariant {
            pub fn into_launcher_type(self, raw: &RawLauncher) -> $name {
                match self {
                    $(
                        Self::$variant => <$inner as LauncherProvider>::parse(raw),
                    )*
                    Self::Empty => $name::Empty
                }
            }
        }
    };
}

create_variants! {
    enum LauncherType {
        Apps(AppLauncher),
        Bookmarks(BookmarkLauncher),
        Calculator(CalculatorLauncher),
        Categories(CategoryLauncher),
        Clipboard(ClipboardLauncher),
        Commands(CommandLauncher),
        MusicPlayer(MusicPlayerLauncher, MusicPlayerFunctions),
        Weather(WeatherLauncher),
        Web(WebLauncher),
        Emoji(EmojiPicker),
        Event(EventLauncher, EventLauncherFunctions),
        Message(MessageLauncher)
        // Integrate later: TODO
        // Pipe(PipeLauncher),
        // Api(BulkTextLauncher),
        // File(FileLauncher),
        // Pomodoro(Pomodoro),
        // Process(ProcessLauncher),
        // Theme(ThemePicker),
    }
}

#[macro_export]
macro_rules! ensure_func {
    ($val:expr, $variant:path) => {
        if let $variant(inner) = $val {
            inner
        } else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::InvalidFunction,
                format!("Invalid function {:?} for this launcher", $val)
            ));
        }
    };
}
