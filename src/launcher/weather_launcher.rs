use chrono::NaiveTime;
use gpui::{Hsla, LinearColorStop, linear_color_stop, rgb};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use strum::Display;

use crate::launcher::weather_launcher::utils::transform_weather;
use crate::launcher::weather_launcher::wttr_serde::WttrResponse;
use crate::loader::resolve_icon_path;
use crate::sherlock_msg;
use crate::ui::widgets::RenderableChild;
use crate::utils::errors::SherlockMessage;
use crate::utils::errors::types::{NetworkAction, SherlockErrorType};
use crate::utils::files::home_dir;
use crate::{
    launcher::{LauncherProvider, LauncherType},
    loader::utils::RawLauncher,
};

mod utils;
mod wttr_serde;

#[derive(Clone, Debug, Deserialize)]
pub enum WeatherIconTheme {
    Sherlock,
    None,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeatherLauncher {
    pub location: String,
    pub update_interval: u64,
    pub icon_theme: WeatherIconTheme,
    // pub show_datetime: bool,
}

impl LauncherProvider for WeatherLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        match serde_json::from_value::<WeatherLauncher>(raw.args.as_ref().clone()) {
            Ok(launcher) => LauncherType::Weather(launcher),
            Err(_) => LauncherType::Empty,
        }
    }
    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        _ctx: &crate::loader::LoadContext,
        _opts: Arc<serde_json::Value>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        match WeatherData::from_cache(self) {
            Some(inner) => Ok(vec![RenderableChild::WeatherLike { launcher, inner }]),
            None => {
                // Return None or a "Loading" placeholder for now
                Ok(vec![RenderableChild::WeatherLike {
                    launcher: Arc::clone(&launcher),
                    inner: WeatherData::uninitialized(),
                }])
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WeatherData {
    pub temperature: String,
    pub icon: Option<Arc<Path>>,
    pub format_str: String,
    pub location: String,
    pub css: WeatherClass,
    pub sunset: chrono::NaiveTime,
    pub sunrise: chrono::NaiveTime,
    pub init: bool,
}
impl WeatherData {
    pub fn uninitialized() -> Self {
        Self {
            temperature: String::new(),
            icon: None,
            format_str: String::new(),
            location: String::new(),
            css: WeatherClass::None,
            sunset: chrono::NaiveTime::default(),
            sunrise: chrono::NaiveTime::default(),
            init: false,
        }
    }
    pub fn from_cache(launcher: &WeatherLauncher) -> Option<Self> {
        let mut path = home_dir().ok()?;
        path.push(format!(
            ".cache/sherlock/weather/{}.json",
            launcher.location
        ));
        fn modtime(path: &PathBuf) -> Option<SystemTime> {
            fs::metadata(path).ok().and_then(|m| m.modified().ok())
        }
        let mtime = modtime(&path)?;
        let time_since = SystemTime::now().duration_since(mtime).ok()?;
        if time_since < Duration::from_secs(60 * launcher.update_interval) {
            let mut cached_data: Self = File::open(&path)
                .ok()
                .and_then(|f| simd_json::from_reader(f).ok())?;

            cached_data.icon = if matches!(launcher.icon_theme, WeatherIconTheme::Sherlock) {
                resolve_icon_path(&format!(
                    "weather-icons/sherlock-weather-{}",
                    cached_data.css
                ))
            } else {
                resolve_icon_path(&format!("weather-{}", cached_data.css))
            };

            return Some(cached_data);
        } else {
            return None;
        }
    }

    fn cache(&self) -> Option<()> {
        let mut path = home_dir().ok()?;
        path.push(format!(".cache/sherlock/weather/{}.json", self.location));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok()?;
        }
        let tmp_path = path.with_extension(".tmp");
        if let Ok(f) = File::create(&tmp_path) {
            if let Ok(_) = simd_json::to_writer(f, &self) {
                let _ = fs::rename(&tmp_path, &path);
            } else {
                let _ = fs::remove_file(&tmp_path);
            }
        }
        None
    }

    pub async fn fetch_async(
        launcher: &WeatherLauncher,
    ) -> Result<(WeatherData, bool), SherlockMessage> {
        // Check for cache hit
        if let Some(data) = WeatherData::from_cache(launcher) {
            return Ok((data, false));
        }

        // Get from wttr.in
        let url = format!("https://de.wttr.in/{}?format=j2", launcher.location);
        let bytes = reqwest::get(url)
            .await
            .map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::NetworkError(NetworkAction::Get, "wttr.in".into()),
                    e
                )
            })?
            .bytes()
            .await
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;

        // 3. Deserialize (Using Serde + simd-json)
        let raw: WttrResponse = simd_json::from_slice(&mut bytes.to_vec())
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;

        // 4. Transform
        let data = transform_weather(raw, launcher)?;
        data.cache();

        Ok((data, true))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum WeatherClass {
    #[serde(rename = "weather-clear")]
    Clear,
    #[serde(rename = "weather-few-clouds")]
    FewClouds,
    #[serde(rename = "weather-many-clouds")]
    ManyClouds,
    #[serde(rename = "weather-mist")]
    Mist,
    #[serde(rename = "weather-showers")]
    Showers,
    #[serde(rename = "weather-freezing-scattered-rain-storm")]
    FreezingScatteredRainStorm,
    #[serde(rename = "weather-freezing-scattered-rain")]
    FreezingScatteredRain,
    #[serde(rename = "weather-storm")]
    Storm,
    #[serde(rename = "weather-snow-scattered-day")]
    SnowScatteredDay,
    #[serde(rename = "weather-snow-storm")]
    SnowStorm,
    #[serde(rename = "weather-snow-scattered-storm")]
    SnowScatteredStorm,
    #[serde(rename = "weather-showers-scattered")]
    ShowersScattered,

    #[serde(rename = "weather-none-available")]
    #[strum(serialize = "none-available")]
    #[default]
    None,
}
impl WeatherClass {
    pub fn background(
        &self,
        now: NaiveTime,
        sunset: NaiveTime,
        sunrise: NaiveTime,
    ) -> (LinearColorStop, LinearColorStop) {
        let is_night = now < sunrise || now > sunset;

        if is_night {
            return (
                linear_color_stop(rgb(0x1E2333), 0.0),
                linear_color_stop(rgb(0x2C3140), 1.0),
            );
        }

        match self {
            Self::Clear => (
                linear_color_stop(rgb(0x87B2E0), 0.0),
                linear_color_stop(rgb(0xCED9E5), 1.0),
            ),
            Self::FewClouds => (
                linear_color_stop(rgb(0xA1A1A1), 0.0),
                linear_color_stop(rgb(0x87B2E0), 1.0),
            ),
            Self::ManyClouds => (
                linear_color_stop(rgb(0xB2B2B2), 0.0),
                linear_color_stop(rgb(0xC8C8C8), 1.0),
            ),
            Self::Mist => (
                linear_color_stop(rgb(0x878787), 0.0),
                linear_color_stop(rgb(0xD1D1C7), 1.0),
            ),
            Self::Showers | Self::ShowersScattered => (
                linear_color_stop(rgb(0x73848C), 0.0),
                linear_color_stop(rgb(0x374B54), 1.0),
            ),
            Self::FreezingScatteredRainStorm
            | Self::Storm
            | Self::SnowStorm
            | Self::SnowScatteredStorm => (
                linear_color_stop(rgb(0x1A1C1F), 0.0),
                linear_color_stop(rgb(0x242B35), 1.0),
            ),
            Self::FreezingScatteredRain | Self::SnowScatteredDay => (
                linear_color_stop(rgb(0x73848C), 0.0),
                linear_color_stop(rgb(0x242B35), 1.0),
            ),
            Self::None => (
                linear_color_stop(rgb(0x2e2e2e), 0.0),
                linear_color_stop(rgb(0x1a1a1a), 1.0),
            ),
        }
    }
    pub fn color(&self, now: NaiveTime, sunset: NaiveTime, sunrise: NaiveTime) -> impl Into<Hsla> {
        let is_night = now < sunrise || now > sunset;
        if is_night {
            return rgb(0xffffff);
        }
        match self {
            Self::Clear => rgb(0xffffff),
            Self::FewClouds => rgb(0xffffff),
            Self::ManyClouds => rgb(0xffffff),
            Self::Mist => rgb(0xffffff),
            Self::Showers | Self::ShowersScattered => rgb(0xffffff),
            Self::FreezingScatteredRainStorm
            | Self::Storm
            | Self::SnowStorm
            | Self::SnowScatteredStorm => rgb(0xffffff),
            Self::FreezingScatteredRain | Self::SnowScatteredDay => rgb(0xffffff),
            Self::None => rgb(0xffffff),
        }
    }
}
