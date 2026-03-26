use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    launcher::{
        LauncherProvider, app_launcher::AppLauncher, audio_launcher::MusicPlayerLauncher,
        bookmark_launcher::BookmarkLauncher, calc_launcher::CalculatorLauncher,
        category_launcher::CategoryLauncher, clipboard_launcher::ClipboardLauncher,
        emoji_launcher::EmojiPicker, system_cmd_launcher::CommandLauncher,
        weather_launcher::WeatherLauncher, web_launcher::WebLauncher,
    },
    loader::utils::RawLauncher,
};

macro_rules! create_variants {
    (
        enum $name:ident {
            $($variant:ident($inner:ty)),* $(,)?
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

        impl $name {
            pub fn get_render_obj(
                &self,
                launcher: std::sync::Arc<crate::launcher::Launcher>,
                ctx: &crate::loader::LoadContext,
                opts: std::sync::Arc<serde_json::Value>,
            ) -> Result<Vec<crate::launcher::children::RenderableChild>, crate::utils::errors::SherlockError> {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as LauncherProvider>::objects(inner, launcher, ctx, opts),
                    )*
                    Self::Empty => Ok(vec![]),
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
        MusicPlayer(MusicPlayerLauncher),
        Weather(WeatherLauncher),
        Web(WebLauncher),
        Emoji(EmojiPicker),
        // Integrate later: TODO
        // Event(EventLauncher),
        // Pipe(PipeLauncher),
        // Api(BulkTextLauncher),
        // File(FileLauncher),
        // Pomodoro(Pomodoro),
        // Process(ProcessLauncher),
        // Theme(ThemePicker),
    }
}
