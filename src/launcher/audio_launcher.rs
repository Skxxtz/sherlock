use bytes::Bytes;
use gpui::{Image, ImageFormat};
use serde_json::Value;
use simd_json::prelude::ArrayTrait;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use zbus::blocking::{Connection, Proxy};

use crate::launcher::children::RenderableChild;
use crate::launcher::utils::MprisState;
use crate::launcher::variant_type::InnerFunction;
use crate::utils::config::ConfigGuard;
use crate::utils::errors::SherlockMessage;
use crate::utils::errors::types::{DBusAction, DirAction, FileAction, SherlockErrorType};
use crate::{ensure_func, sherlock_msg};

use super::utils::MprisData;

use crate::launcher::{Bind, LauncherProvider, LauncherType};
use crate::loader::utils::RawLauncher;

#[derive(Debug, Clone, Default)]
pub struct MusicPlayerLauncher {
    binds: Option<Arc<Vec<Bind>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, strum::VariantNames, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum MusicPlayerFunctions {
    TogglePlayback,
    Previous,
    Next,
}

impl LauncherProvider for MusicPlayerLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        let binds = raw.binds.as_ref().map(|vec| {
            Arc::new(
                vec.iter()
                    .filter_map(|b| {
                        let func = MusicPlayerFunctions::from_str(&b.callback).ok()?;
                        b.get_bind(InnerFunction::MusicPlayer(func))
                    })
                    .collect(),
            )
        });
        LauncherType::MusicPlayer(MusicPlayerLauncher { binds })
    }
    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        _: &crate::loader::LoadContext,
        _opts: Arc<Value>,
    ) -> Result<Vec<super::children::RenderableChild>, SherlockMessage> {
        let inner = MprisState::default();
        Ok(vec![RenderableChild::MusicLike { launcher, inner }])
    }
    fn binds(&self) -> Option<Arc<Vec<Bind>>> {
        self.binds.clone()
    }
    fn execute_function(
        &self,
        func: InnerFunction,
        child: &RenderableChild,
    ) -> Result<bool, SherlockMessage> {
        let func = ensure_func!(func, InnerFunction::MusicPlayer);

        let RenderableChild::MusicLike { inner, .. } = child else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::Unreachable,
                format!("Tried to unpack music tile but received: {:?}", child)
            ));
        };

        let Some(player) = &inner.player else {
            return Ok(false);
        };

        match func {
            MusicPlayerFunctions::Next => MprisData::next(player)?,
            MusicPlayerFunctions::Previous => MprisData::previous(player)?,
            MusicPlayerFunctions::TogglePlayback => MprisData::playpause(player)?,
        }

        Ok(true)
    }
}

impl MprisData {
    /// Get current image
    /// Return:
    /// image: Pixbuf
    /// was_cached: bool
    pub async fn get_image(&self) -> Option<(Arc<Image>, bool)> {
        let art_url = self.metadata.art.as_ref()?;
        let loc = art_url.split("/").last()?.to_string();
        let mut was_cached = true;
        let bytes = match Self::read_cached_cover(&loc) {
            Ok(b) => b,
            Err(_) => {
                if art_url.starts_with("file") {
                    Self::read_image_file(art_url).ok()?
                } else {
                    let response = reqwest::get(art_url).await.ok()?;
                    let bytes = response.bytes().await.ok()?;
                    let _ = Self::cache_cover(&bytes, &loc);
                    was_cached = false;
                    bytes.into()
                }
            }
        };

        // mimetype parsing
        let mime = identify_image_type(&bytes);
        let format = ImageFormat::from_mime_type(mime)?;

        let image_arc = Arc::new(Image::from_bytes(format, bytes));
        Some((image_arc, was_cached))
    }
    fn cache_cover(image: &Bytes, loc: &str) -> Result<(), SherlockMessage> {
        // Create dir and parents
        let home = env::var("HOME").map_err(|e| {
            sherlock_msg!(Warning, SherlockErrorType::EnvError("HOME".to_string()), e)
        })?;

        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::DirError(DirAction::Create, parent.to_path_buf(),),
                    e.to_string()
                )
            })?;
        };

        let mut file = if path.exists() {
            File::open(&path)
        } else {
            File::create(&path)
        }
        .map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;

        file.write_all(&image).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;
        // if file not exist, create and write it
        Ok(())
    }
    fn read_cached_cover(loc: &str) -> Result<Vec<u8>, SherlockMessage> {
        let home = env::var("HOME")
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::EnvError("$HOME".into()), e))?;
        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);

        let mut file = File::open(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.clone()),
                e
            )
        })?;
        Ok(buffer)
    }
    fn read_image_file(loc: &str) -> Result<Vec<u8>, SherlockMessage> {
        let path = PathBuf::from(loc.trim_start_matches("file://"));

        let mut file = File::open(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.clone()),
                e
            )
        })?;
        Ok(buffer)
    }
    pub fn playpause(player: &str) -> Result<(), SherlockMessage> {
        Self::player_method(player, "PlayPause")
    }
    pub fn next(player: &str) -> Result<(), SherlockMessage> {
        Self::player_method(player, "Next")
    }
    pub fn previous(player: &str) -> Result<(), SherlockMessage> {
        Self::player_method(player, "Previous")
    }
    fn player_method(player: &str, method: &str) -> Result<(), SherlockMessage> {
        let conn = Connection::session().map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::DBusError(DBusAction::Connect, "Session Bus".into()),
                e
            )
        })?;
        let proxy = Proxy::new(
            &conn,
            player,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DBusError(DBusAction::Construct, player.to_string()),
                e
            )
        })?;
        proxy.call_method(method, &()).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DBusError(DBusAction::Call, method.to_string()),
                e
            )
        })?;
        Ok(())
    }
    pub fn update(&self) -> Option<(Self, bool)> {
        let audio_launcher = AudioLauncherFunctions::new()?;
        let player = audio_launcher.get_current_player()?;
        let mpris = audio_launcher.get_metadata(&player)?;
        let changed = self.metadata.title != self.metadata.title;
        Some((mpris, changed))
    }
}

pub struct AudioLauncherFunctions {
    conn: Connection,
}

impl AudioLauncherFunctions {
    pub fn new() -> Option<Self> {
        let conn = Connection::session().ok()?;
        Some(AudioLauncherFunctions { conn })
    }
    pub fn get_current_player(&self) -> Option<String> {
        let proxy = Proxy::new(
            &self.conn,
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
        )
        .ok()?;
        let mut names: Vec<String> = proxy.call("ListNames", &()).ok()?;
        names.retain(|n| n.starts_with("org.mpris.MediaPlayer2."));
        let first = names.first().cloned();
        if let Ok(config) = ConfigGuard::read() {
            if let Some(m) = config.default_apps.mpris.as_ref() {
                let preferred = names.into_iter().find(|name| name.contains(m));
                if preferred.is_some() {
                    return preferred;
                }
            }
        }
        first
    }
    pub fn get_metadata(&self, player: &str) -> Option<MprisData> {
        let proxy = Proxy::new(
            &self.conn,
            player,
            "/org/mpris/MediaPlayer2", // Object path for the player
            "org.freedesktop.DBus.Properties",
        )
        .ok()?;
        let message = proxy
            .call_method("GetAll", &("org.mpris.MediaPlayer2.Player"))
            .ok()?;
        let body = message.body();
        body.deserialize().ok()
    }
}

/// This function reads the "magic bytes" of images files to identify its mimetype
fn identify_image_type(bytes: &[u8]) -> &'static str {
    if bytes.len() < 4 {
        return "image/png";
    }

    match &bytes[0..4] {
        [0x89, 0x50, 0x4E, 0x47] => "image/png",
        [0xFF, 0xD8, 0xFF, _] => "image/jpeg",
        [0x47, 0x49, 0x46, 0x38] => "image/gif",
        [0x42, 0x4D, _, _] => "image/bmp",
        [0x52, 0x49, 0x46, 0x46] if &bytes[8..12] == b"WEBP" => "image/webp",
        _ => "image/png",
    }
}
