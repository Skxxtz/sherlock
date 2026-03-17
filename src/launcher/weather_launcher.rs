use gpui::{Hsla, LinearColorStop, hsla, linear_color_stop, rgb};
use serde::{Deserialize, Serialize};
use simd_json::base::{ValueAsArray, ValueAsScalar};
use simd_json::derived::ValueObjectAccess;
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use strum::Display;

use super::utils::to_title_case;
use crate::loader::resolve_icon_path;
use crate::utils::config::ConfigGuard;
use crate::utils::files::home_dir;

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
    pub show_datetime: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WeatherData {
    pub temperature: String,
    pub icon: Option<Arc<Path>>,
    pub format_str: String,
    pub location: String,
    pub css: WeatherClass,
    pub sunset: chrono::NaiveTime,
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
    pub async fn fetch_async(launcher: &WeatherLauncher) -> Option<(WeatherData, bool)> {
        let config = ConfigGuard::read().ok()?;
        // try read cache
        if let Some(data) = WeatherData::from_cache(launcher) {
            return Some((data, false));
        };

        let url = format!("https://de.wttr.in/{}?format=j2", launcher.location);

        let response = reqwest::get(url).await.ok()?.text().await.ok()?;
        let mut response_bytes = response.into_bytes();
        let json: simd_json::OwnedValue = simd_json::to_owned_value(&mut response_bytes).ok()?;
        let current_condition = json
            .get("data")?
            .get("current_condition")?
            .as_array()?
            .get(0)?;

        // Get sunset time
        let astronomy = json
            .get("data")?
            .get("weather")?
            .as_array()?
            .get(0)?
            .get("astronomy")?
            .as_array()?
            .get(0)?;
        let sunset_raw = astronomy.get("sunset")?.as_str()?;
        let sunset = chrono::NaiveTime::parse_from_str(sunset_raw, "%I:%M %p").ok()?;

        // Parse Temperature
        let temperature = match config.units.temperatures.as_str() {
            "f" | "F" => format!("{}°F", current_condition.get("temp_F")?.as_str()?),
            _ => format!("{}°C", current_condition.get("temp_C")?.as_str()?),
        };

        // Parse Icon
        let code = current_condition.get("weatherCode")?.as_str()?;
        let icon = if matches!(launcher.icon_theme, WeatherIconTheme::Sherlock) {
            resolve_icon_path(&format!(
                "weather-icons/sherlock-weather-{}",
                Self::match_weather_code(code)
            ))
        } else {
            resolve_icon_path(&format!("weather-{}", Self::match_weather_code(code)))
        };

        // Parse wind dir
        let wind_deg = current_condition
            .get("winddirDegree")?
            .as_str()?
            .parse::<f32>()
            .ok()?;
        let sector_size: f32 = 45.0;
        let index = ((wind_deg + sector_size / 2.0) / sector_size).floor() as usize % 8;
        let win_dirs = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];
        let wind_dir = win_dirs.get(index)?;

        // Parse wind speed
        let imperials: HashSet<&str> = HashSet::from([
            "inches", "inch", "in", "feet", "foot", "ft", "yards", "yard", "yd", "miles", "mile",
            "mi",
        ]);
        let wind = if imperials.contains(config.units.lengths.to_lowercase().as_str()) {
            let speed = current_condition.get("windspeedMiles")?.as_str()?;
            format!("{} {}mph", wind_dir, speed)
        } else {
            let speed = current_condition.get("windspeedKmph")?.as_str()?;
            format!("{} {}km/h", wind_dir, speed)
        };

        let loc = to_title_case(&launcher.location);
        let format_str = format!("{}  {}", loc, wind);
        let data = WeatherData {
            temperature,
            icon,
            format_str,
            location: launcher.location.clone(),
            css: Self::match_weather_code(code),
            sunset,
            init: true,
        };
        data.cache();

        Some((data, true))
    }
    fn match_weather_code(code: &str) -> WeatherClass {
        match code {
            "113" => WeatherClass::Clear,
            "116" => WeatherClass::FewClouds,
            "119" | "122" => WeatherClass::ManyClouds,
            "143" | "248" | "260" => WeatherClass::Mist,
            "176" | "263" | "299" | "305" | "353" | "356" => WeatherClass::Showers,
            "179" | "362" | "365" | "374" => WeatherClass::FreezingScatteredRainStorm,
            "182" | "185" | "281" | "284" | "311" | "314" | "317" | "350" | "377" => {
                WeatherClass::FreezingScatteredRain
            }
            "200" | "302" | "308" | "359" | "386" | "389" => WeatherClass::Storm,
            "227" | "320" => WeatherClass::SnowScatteredDay,
            "230" | "329" | "332" | "338" => WeatherClass::SnowStorm,
            "323" | "326" | "335" | "368" | "371" | "392" | "395" => {
                WeatherClass::SnowScatteredStorm
            }
            "266" | "293" | "296" => WeatherClass::ShowersScattered,
            _ => WeatherClass::None,
        }
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
    pub fn background(&self) -> (LinearColorStop, LinearColorStop) {
        match self {
            Self::Clear => (
                linear_color_stop(hsla(2.1101, 0.5894, 0.7039, 1.0), 0.0),
                linear_color_stop(hsla(2.113, 0.3067, 0.8529, 1.0), 0.5),
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
    pub fn color(&self) -> impl Into<Hsla> {
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
