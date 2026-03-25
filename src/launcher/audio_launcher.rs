use bytes::Bytes;
use gpui::{Image, ImageFormat};
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use zbus::blocking::{Connection, Proxy};

use crate::launcher::children::RenderableChild;
use crate::launcher::utils::MprisState;
use crate::sherlock_error;
use crate::utils::config::ConfigGuard;
use crate::utils::errors::{SherlockError, SherlockErrorType};

use super::utils::MprisData;

use crate::launcher::{LauncherProvider, LauncherType};
use crate::loader::utils::RawLauncher;

#[derive(Debug, Clone, Default)]
pub struct MusicPlayerLauncher {}

impl LauncherProvider for MusicPlayerLauncher {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::MusicPlayer(MusicPlayerLauncher {})
    }
    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        _: &crate::loader::LoadContext,
        _opts: Arc<Value>,
    ) -> Result<Vec<super::children::RenderableChild>, SherlockError> {
        let inner = MprisState {
            raw: None,
            image: None,
        };
        Ok(vec![RenderableChild::MusicLike { launcher, inner }])
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
    fn cache_cover(image: &Bytes, loc: &str) -> Result<(), SherlockError> {
        // Create dir and parents
        let home = env::var("HOME").map_err(|e| {
            sherlock_error!(
                SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
                e.to_string()
            )
        })?;

        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| sherlock_error!(
                SherlockErrorType::DirCreateError(
                    "~/.cache/sherlock/mpris-cache/".to_string(),
                ),
                e.to_string()
            ))?;
        };

        let mut file = if path.exists() {
            File::open(&path)
        } else {
            File::create(&path)
        }
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileExistError(path.clone()),
                e.to_string()
            )
        })?;

        file.write_all(&image).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileExistError(path.clone()),
                e.to_string()
            )
        })?;
        // if file not exist, create and write it
        Ok(())
    }
    fn read_cached_cover(loc: &str) -> Result<Vec<u8>, SherlockError> {
        let home = env::var("HOME").map_err(|e| {
            sherlock_error!(
                SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
                e.to_string()
            )
        })?;
        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);

        let mut file = File::open(&path).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileExistError(path.clone()),
                e.to_string()
            )
        })?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileReadError(path.clone()),
                e.to_string()
            )
        })?;
        Ok(buffer)
    }
    fn read_image_file(loc: &str) -> Result<Vec<u8>, SherlockError> {
        let path = PathBuf::from(loc.trim_start_matches("file://"));

        let mut file = File::open(&path).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileExistError(path.clone()),
                e.to_string()
            )
        })?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileReadError(path.clone()),
                e.to_string()
            )
        })?;
        Ok(buffer)
    }
    pub fn playpause(player: &str) -> Result<(), SherlockError> {
        Self::player_method(player, "PlayPause")
    }
    pub fn next(player: &str) -> Result<(), SherlockError> {
        Self::player_method(player, "Next")
    }
    pub fn previous(player: &str) -> Result<(), SherlockError> {
        Self::player_method(player, "Previous")
    }
    fn player_method(player: &str, method: &str) -> Result<(), SherlockError> {
        let conn = Connection::session()
            .map_err(|e| sherlock_error!(SherlockErrorType::DBusConnectionError, e.to_string()))?;
        let proxy = Proxy::new(
            &conn,
            player,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::DBusMessageConstructError(format!("PlayPause for {}", player)),
                e.to_string()
            )
        })?;
        proxy.call_method(method, &()).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::DBusMessageSendError(format!("PlayPause to {}", player)),
                e.to_string()
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
