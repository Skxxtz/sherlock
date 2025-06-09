use bytes::Bytes;
use gtk4::gdk_pixbuf::{Pixbuf, PixbufLoader};
use gtk4::prelude::*;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use zbus::blocking::{Connection, Proxy};

use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::{sherlock_error, CONFIG};

use super::utils::MprisData;

#[derive(Debug, Clone)]
pub struct MusicPlayerLauncher {
    pub player: String,
    pub mpris: MprisData,
}
impl MusicPlayerLauncher {
    /// Get current image
    /// Return:
    /// image: Pixbuf
    /// was_cached: bool
    pub async fn get_image(&self) -> Option<(Pixbuf, bool)> {
        let loc = match &self.mpris.metadata.art.split("/").last() {
            Some(s) => s.to_string(),
            _ => return None,
        };
        let mut was_cached = true;
        let bytes = match MusicPlayerLauncher::read_cached_cover(&loc) {
            Ok(b) => b,
            Err(_) => {
                if self.mpris.metadata.art.starts_with("file") {
                    MusicPlayerLauncher::read_image_file(&self.mpris.metadata.art).ok()?
                } else {
                    let response = reqwest::get(&self.mpris.metadata.art).await.ok()?;
                    let bytes = response.bytes().await.ok()?;
                    let _ = MusicPlayerLauncher::cache_cover(&bytes, &loc);
                    was_cached = false;
                    bytes
                }
            }
        };

        let loader = PixbufLoader::new();
        loader.write(&bytes).ok()?;
        loader.close().ok()?;
        loader.pixbuf().and_then(|i| Some((i, was_cached)))
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
    fn read_cached_cover(loc: &str) -> Result<Bytes, SherlockError> {
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
        Ok(buffer.into())
    }
    fn read_image_file(loc: &str) -> Result<Bytes, SherlockError> {
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
        Ok(buffer.into())
    }
    pub fn playpause(player: &str) -> Result<(), SherlockError> {
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
        proxy.call_method("PlayPause", &()).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::DBusMessageSendError(format!("PlayPause to {}", player)),
                e.to_string()
            )
        })?;
        Ok(())
    }
    pub fn update(&self) -> Option<(Self, bool)> {
        // needed because Sherlock is too fast 🥴
        std::thread::sleep(std::time::Duration::from_millis(50));
        let audio_launcher = AudioLauncherFunctions::new()?;
        let player = audio_launcher.get_current_player()?;
        let mpris = audio_launcher.get_metadata(&player)?;
        let changed = mpris.mpris.metadata.title != self.mpris.metadata.title;
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
        if let Some(m) = CONFIG.get().and_then(|c| c.default_apps.mpris.as_deref()) {
            let preffered = names.into_iter().find(|name| name.contains(m));
            if preffered.is_some() {
                return preffered;
            }
        }
        first
    }
    pub fn get_metadata(&self, player: &str) -> Option<MusicPlayerLauncher> {
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
        let mpris_data: MprisData = body.deserialize().ok()?;

        Some(MusicPlayerLauncher {
            player: player.to_string(),
            mpris: mpris_data,
        })
    }
}
