use std::{path::Path, sync::Arc};

use crate::{
    launcher::{
        utils::to_title_case,
        weather_launcher::{
            WeatherClass, WeatherData, WeatherIconTheme, WeatherLauncher, WttrResponse,
            wttr_serde::CurrentCondition,
        },
    },
    loader::resolve_icon_path,
    sherlock_msg,
    utils::{
        config::ConfigGuard,
        errors::{SherlockMessage, types::SherlockErrorType},
    },
};

fn format_temp(current: &CurrentCondition, unit_pref: &str) -> String {
    match unit_pref.to_lowercase().as_str() {
        "f" => format!("{}°F", current.temp_f),
        _ => format!("{}°C", current.temp_c),
    }
}

fn format_wind(current: &CurrentCondition, length_unit: &str) -> String {
    const IMPERIALS: &[&str] = &[
        "inches", "inch", "in", "feet", "foot", "ft", "miles", "mile", "mi",
    ];

    let deg = current.wind_deg.as_str().parse::<f32>().unwrap_or(0.0);
    let direction = get_wind_dir(deg);

    if IMPERIALS.contains(&length_unit.to_lowercase().as_str()) {
        format!("{} {}mph", direction, current.wind_miles)
    } else {
        format!("{} {}km/h", direction, current.wind_kmph)
    }
}

fn resolve_icon(theme: &WeatherIconTheme, code: &str) -> Option<Arc<Path>> {
    let code_mapped = match_weather_code(code);

    let path = if matches!(theme, WeatherIconTheme::Sherlock) {
        format!("weather-icons/sherlock-weather-{}", code_mapped)
    } else {
        format!("weather-{}", code_mapped)
    };

    resolve_icon_path(&path)
}

fn parse_time(time_str: &str) -> Option<chrono::NaiveTime> {
    chrono::NaiveTime::parse_from_str(time_str, "%I:%M %p").ok()
}

pub(super) fn transform_weather(
    raw: WttrResponse,
    launcher: &WeatherLauncher,
) -> Result<WeatherData, SherlockMessage> {
    let current = &raw.current_condition[0];
    let astro = &raw.weather[0].astronomy[0];
    let config = ConfigGuard::read()?;

    let temperature = format_temp(&current, &config.units.temperatures);
    let icon = resolve_icon(&launcher.icon_theme, &current.weather_code);
    let wind = format_wind(&current, &config.units.lengths);

    let sunset = parse_time(&astro.sunset).ok_or_else(|| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DeserializationError,
            "Failed to parse sunset time from string"
        )
    })?;
    let sunrise = parse_time(&astro.sunrise).ok_or_else(|| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DeserializationError,
            "Failed to parse sunrise time from string"
        )
    })?;

    Ok(WeatherData {
        temperature,
        icon,
        format_str: format!("{}  {}", to_title_case(&launcher.location), wind),
        location: launcher.location.clone(),
        css: match_weather_code(&current.weather_code),
        sunset,
        sunrise,
        init: true,
    })
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
        "323" | "326" | "335" | "368" | "371" | "392" | "395" => WeatherClass::SnowScatteredStorm,
        "266" | "293" | "296" => WeatherClass::ShowersScattered,
        _ => WeatherClass::None,
    }
}

fn get_wind_dir(deg: f32) -> &'static str {
    // 360 / 8 segments = 45 degrees per segment
    // We add 22.5 to center the segments (e.g., N is 337.5 to 22.5)
    let win_dirs = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];
    let index = ((deg + 22.5) / 45.0).floor() as usize % 8;
    win_dirs[index]
}
