use serde::{Deserialize, Serialize};
use simd_json::base::{ValueAsArray, ValueAsScalar};
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::utils::to_title_case;
use crate::utils::files::home_dir;
use crate::CONFIG;

#[derive(Clone, Debug)]
pub struct WeatherLauncher {
    pub location: String,
    pub update_interval: u64,
}
impl WeatherLauncher {
    pub async fn get_result(&self) -> Option<(WeatherData, bool)> {
        let config = CONFIG.get()?;
        // try read cache
        if let Some(data) = WeatherData::from(&self) {
            return Some((data, false));
        };

        let url = format!("https://de.wttr.in/{}?format=j2", self.location);

        let response = reqwest::get(url).await.ok()?.text().await.ok()?;
        let mut response_bytes = response.into_bytes();
        let json: simd_json::OwnedValue = simd_json::to_owned_value(&mut response_bytes).ok()?;
        let current_condition = json["current_condition"].as_array()?.get(0)?;

        // Parse Temperature
        let temperature = match config.units.temperatures.as_str() {
            "t" | "T" => format!("{}°F", current_condition["temp_F"].as_str()?),
            _ => format!("{}°C", current_condition["temp_C"].as_str()?),
        };

        // Parse Icon
        let code = current_condition["weatherCode"].as_str()?;
        let icon = WeatherLauncher::match_weather_code(code);

        // Parse wind dir
        let wind_deg = current_condition["winddirDegree"]
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
            let speed = current_condition["windspeedMiles"].as_str()?;
            format!("{} {}mph", wind_dir, speed)
        } else {
            let speed = current_condition["windspeedKmph"].as_str()?;
            format!("{} {}km/h", wind_dir, speed)
        };

        let loc = to_title_case(&self.location);
        let format_str = format!("{}  {}", loc, wind);
        let data = WeatherData {
            temperature,
            icon,
            format_str,
            location: self.location.clone(),
        };
        data.cache();

        Some((data, true))
    }
    fn match_weather_code(code: &str) -> String {
        let icon = match code {
            "113" => "weather-clear",
            "116" => "weather-few-clouds",
            "119" | "122" => "weather-many-clouds",
            "143" | "248" | "260" => "weather-mist",
            "176" | "263" | "299" | "305" | "353" | "356" => "weather-showers",
            "179" | "362" | "365" | "374" => "weather-freezing-scattered-rain-storm",
            "182" | "185" | "281" | "284" | "311" | "314" | "317" | "350" | "377" => {
                "weather-freezing-scattered-rain"
            }
            "200" | "302" | "308" | "359" | "386" | "389" => "weather-storm",
            "227" | "320" => "weather-snow-scattered-day",
            "230" | "329" | "332" | "338" => "weather-snow-storm",
            "323" | "326" | "335" | "368" | "371" | "392" | "395" => "weather-snow-scattered-storm",
            "266" | "293" | "296" => "weather-showers-scattered",
            _ => "weather-none-available",
        };
        String::from(icon)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WeatherData {
    pub temperature: String,
    pub icon: String,
    pub format_str: String,
    pub location: String,
}
impl WeatherData {
    fn from(launcher: &WeatherLauncher) -> Option<Self> {
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
            let cached_data: Option<Self> = File::open(&path)
                .ok()
                .and_then(|f| simd_json::from_reader(f).ok());
            return cached_data;
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
}
