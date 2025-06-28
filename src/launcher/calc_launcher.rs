use crate::{
    sherlock_error,
    utils::{
        config::get_config,
        errors::{SherlockError, SherlockErrorType},
        files::home_dir,
    },
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use simd_json::{
    base::{ValueAsArray, ValueAsScalar},
    derived::ValueObjectAccess,
    OwnedValue,
};
use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir_all, File},
    path::Path,
    sync::OnceLock,
    time::{Duration, SystemTime},
};

#[derive(Clone, Debug)]
pub struct CalculatorLauncher {
    pub capabilities: HashSet<String>,
}

pub struct Calculator;
impl Calculator {
    pub fn measurement(keyword: &str, unit_str: &str) -> Option<(String, String)> {
        let full_pattern = r"(?i)(\d+(?:\.\d+)?)\s*([a-zA-Z]+)\s*(in|to)\s*([a-zA-Z]+)";
        let full_re = Regex::new(full_pattern).unwrap();
        if let Some(caps) = full_re.captures(keyword) {
            let value: f32 = caps[1].parse().ok()?;
            let from = caps[2].to_lowercase();
            let to = caps[4].to_lowercase();

            let (factor_from, _) = Measurements::match_unit(&from, unit_str)?;
            let (factor_to, name) = Measurements::match_unit(&to, unit_str)?;

            let base = Calculator::to_basis(factor_from, value);
            let res = Calculator::to_unit(factor_to, base);
            let postfix = if res == 1.0 || unit_str == "currencies" {
                ""
            } else {
                "s"
            };
            return Some((res.to_string(), format!("= {:.2} {}{}", res, name, postfix)));
        }
        // Support for partial ones
        let part_pattern = r"(?i)(\d+(?:\.\d+)?)\s*([a-zA-Z]+)";
        let part_re = Regex::new(part_pattern).unwrap();
        if let Some(caps) = part_re.captures(keyword) {
            let config = get_config().ok()?;
            let value: f32 = caps[1].parse().ok()?;
            let from = caps[2].to_lowercase();
            let to = match unit_str {
                "weights" => config.units.weights.to_lowercase(),
                "volumes" => config.units.volumes.to_lowercase(),
                "currencies" => config.units.currency.to_lowercase(),
                _ => config.units.lengths.to_lowercase(),
            };

            let (factor_from, _) = Measurements::match_unit(&from, unit_str)?;
            let (factor_to, name) = Measurements::match_unit(&to, unit_str)?;

            let base = Calculator::to_basis(factor_from, value);
            let res = Calculator::to_unit(factor_to, base);
            let postfix = if res == 1.0 || unit_str == "currencies" {
                ""
            } else {
                "s"
            };
            return Some((res.to_string(), format!("= {:.2} {}{}", res, name, postfix)));
        }
        None
    }
    pub fn temperature(keyword: &str) -> Option<(String, String)> {
        let ctof = |c: f32| (c * 9.0 / 5.0) + 32.0;
        let ftoc = |f: f32| (f - 32.0) * 5.0 / 9.0;
        let parse_unit = |unit: &str| {
            if unit == "c" || unit.len() > 1 && "celsius".contains(&unit) {
                "C"
            } else if unit == "f" || unit.len() > 1 && "fahrenheit".contains(&unit) {
                "F"
            } else {
                "C"
            }
        };
        let full_pattern = r"(?i)^(?P<value>\d+(?:\.\d+)?)\s*(?:degrees?|°)?\s*(?P<from>(c|f)(elsius|ahrenheit)?)?\s*(?:to|as|in)?\s*(?:degrees?|°)?\s*(?P<to>(c|f)(elsius|ahrenheit)?)?$";

        let full_re = Regex::new(full_pattern).unwrap();
        match full_re.captures(keyword) {
            Some(caps) => {
                let value = caps.name("value")?.as_str().parse::<f32>().ok()?;
                let from = parse_unit(&caps.name("from")?.as_str().to_lowercase());
                let to = caps
                    .name("to")
                    .map_or(if from == "C" { "F" } else { "C" }, |v| {
                        parse_unit(v.as_str())
                    });
                match to {
                    "C" => {
                        let res = ftoc(value);
                        Some((res.to_string(), format!("= {} °C", res)))
                    }
                    _ => {
                        let res = ctof(value);
                        Some((res.to_string(), format!("= {} °F", res)))
                    }
                }
            }
            _ => None,
        }
    }
    fn to_basis(factor: f32, value: f32) -> f32 {
        value * factor
    }
    fn to_unit(factor: f32, value: f32) -> f32 {
        value / factor
    }
}

enum Measurements {}
impl Measurements {
    fn match_unit(unit: &str, unit_str: &str) -> Option<(f32, String)> {
        match unit_str {
            "weights" => Weight::match_unit(unit),
            "volumes" => Volume::match_unit(unit),
            "currencies" => {
                if Currency::unit_exists(unit) {
                    CURRENCIES.get()?.as_ref().and_then(|c| c.match_unit(unit))
                } else {
                    None
                }
            }
            _ => Length::match_unit(unit),
        }
    }
}

pub struct Length;
impl Length {
    pub const KILOMETER: f32 = 1000.0;
    pub const CENTIMETER: f32 = 0.01;
    pub const MILLIMETER: f32 = 0.001;
    pub const MICROMETER: f32 = 0.000_001;

    pub const INCH: f32 = 0.0254;
    pub const FEET: f32 = 0.3048;
    pub const YARD: f32 = 0.9144;
    pub const MILE: f32 = 1609.34;

    fn match_unit(unit: &str) -> Option<(f32, String)> {
        match unit.to_lowercase().as_str() {
            // Metric units
            "kilometers" | "kilometer" | "kilos" | "km" => {
                Some((Length::KILOMETER, String::from("Kilometer")))
            }
            "meters" | "meter" | "m" => Some((1.0, String::from("Meter"))),
            "centimeters" | "centimeter" | "cm" | "cents" => {
                Some((Length::CENTIMETER, String::from("Centimeter")))
            }
            "millimeters" | "millimeter" | "mm" => {
                Some((Length::MILLIMETER, String::from("Millimeter")))
            }
            "micrometers" | "micrometer" | "um" | "µm" => {
                Some((Length::MICROMETER, String::from("Micrometer")))
            }

            // Imperial units
            "inches" | "inch" | "in" => Some((Length::INCH, String::from("Inch"))),
            "feet" | "foot" | "ft" => Some((Length::FEET, String::from("Feet"))),
            "yards" | "yard" | "yd" => Some((Length::YARD, String::from("Yard"))),
            "miles" | "mile" | "mi" => Some((Length::MILE, String::from("Mile"))),

            _ => None,
        }
    }
}

pub struct Weight;
impl Weight {
    // Weight units
    pub const KILOGRAM: f32 = 1.0;
    pub const GRAM: f32 = 0.001;
    pub const MILLIGRAM: f32 = 0.000_001;
    pub const POUND: f32 = 0.453592;
    pub const OUNCE: f32 = 0.0283495;

    pub const TABLESPOON: f32 = 0.015;
    pub const TEASPOON: f32 = 0.005;
    pub const PINCH: f32 = 0.00036;
    pub const DASH: f32 = 0.0006;

    fn match_unit(unit: &str) -> Option<(f32, String)> {
        match unit.to_lowercase().as_str() {
            "kilograms" | "kilogram" | "kg" => Some((Weight::KILOGRAM, String::from("Kilogram"))),
            "grams" | "gram" | "g" => Some((Weight::GRAM, String::from("Gram"))),
            "milligrams" | "milligram" | "mg" => {
                Some((Weight::MILLIGRAM, String::from("Milligram")))
            }
            "pounds" | "pound" | "lbs" => Some((Weight::POUND, String::from("Pound"))),
            "ounces" | "ounce" | "oz" => Some((Weight::OUNCE, String::from("Ounce"))),

            "tablespoons" | "tablespoon" | "tbsp" => {
                Some((Weight::TABLESPOON, String::from("Tablespoon")))
            }
            "teaspoons" | "teaspoon" | "tsp" => Some((Weight::TEASPOON, String::from("Teaspoon"))),
            "pinch" | "pinches" => Some((Weight::PINCH, String::from("Pinch"))),
            "dash" | "dashes" => Some((Weight::DASH, String::from("Dash"))),

            _ => None,
        }
    }
}
pub struct Volume;
impl Volume {
    pub const LITER: f32 = 1.0;
    pub const MILLILITER: f32 = 0.001;
    pub const CUBIC_METER: f32 = 1000.0;
    pub const GALLON: f32 = 3.78541;
    pub const QUART: f32 = 0.946353;
    pub const PINT: f32 = 0.473176;
    pub const CUP: f32 = 0.24;
    pub const FLUID_OUNCE: f32 = 0.0295735;

    pub fn match_unit(unit: &str) -> Option<(f32, String)> {
        match unit.to_lowercase().as_str() {
            "liters" | "liter" | "l" => Some((Volume::LITER, String::from("Liter"))),
            "milliliters" | "milliliter" | "ml" => {
                Some((Volume::MILLILITER, String::from("Milliliter")))
            }
            "cubicmeters" | "cubicmeter" | "m3" => {
                Some((Volume::CUBIC_METER, String::from("Cubic Meter")))
            }
            "gallons" | "gallon" | "gal" => Some((Volume::GALLON, String::from("Gallon"))),
            "quarts" | "quart" | "qt" => Some((Volume::QUART, String::from("Quart"))),
            "pints" | "pint" | "pt" => Some((Volume::PINT, String::from("Pint"))),
            "cups" | "cup" => Some((Volume::CUP, String::from("Cup"))),
            "fluidounces" | "fluidounce" | "fl oz" | "oz" => {
                Some((Volume::FLUID_OUNCE, String::from("Fluid Ounce")))
            }

            _ => None,
        }
    }
}

pub static CURRENCIES: OnceLock<Option<Currency>> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize)]
pub struct Currency {
    usd: f32, // US Dollar
    eur: f32, // Euro
    jpy: f32, // Japanese Yen
    gbp: f32, // British Pound Sterling
    aud: f32, // Australian Dollar
    cad: f32, // Canadian Dollar
    chf: f32, // Swiss Franc
    cny: f32, // Chinese Yuan
    nzd: f32, // New Zealand Dollar
    sek: f32, // Swedish Krona
    nok: f32, // Norwegian Krone
    mxn: f32, // Mexican Peso
    sgd: f32, // Singapore Dollar
    hkd: f32, // Hong Kong Dollar
    krw: f32, // South Korean Won
}
impl Currency {
    pub fn from_map(mut map: HashMap<String, f32>) -> Option<Self> {
        Some(Self {
            usd: 1.0,
            eur: map.remove("eur")?,
            jpy: map.remove("jpy")?,
            gbp: map.remove("gbp")?,
            aud: map.remove("aud")?,
            cad: map.remove("cad")?,
            chf: map.remove("chf")?,
            cny: map.remove("cny")?,
            nzd: map.remove("nzd")?,
            sek: map.remove("sek")?,
            nok: map.remove("nok")?,
            mxn: map.remove("mxn")?,
            sgd: map.remove("sgd")?,
            hkd: map.remove("hkd")?,
            krw: map.remove("krw")?,
        })
    }
    pub fn match_unit(&self, unit: &str) -> Option<(f32, String)> {
        match unit.to_lowercase().trim() {
            "usd" | "dollar" | "us dollar" | "bucks" => Some((self.usd, "$".to_string())),
            "eur" | "euro" | "euros" | "european euro" => Some((self.eur, "€".to_string())),
            "jpy" | "yen" | "japanese yen" => Some((self.jpy, "¥".to_string())),
            "gbp" | "pound" | "british pound" | "pound sterling" => {
                Some((self.gbp, "£".to_string()))
            }
            "aud" | "australian dollar" | "aussie dollar" | "aussie" => {
                Some((self.aud, "A$".to_string()))
            }
            "cad" | "canadian dollar" | "loonie" => Some((self.cad, "C$".to_string())),
            "chf" | "swiss franc" | "franc" => Some((self.chf, "CHF".to_string())),
            "cny" | "chinese yuan" | "renminbi" | "yuan" => Some((self.cny, "¥".to_string())),
            "nzd" | "new zealand dollar" | "kiwi" => Some((self.nzd, "NZ$".to_string())),
            "sek" | "swedish krona" | "krona" => Some((self.sek, "kr".to_string())),
            "nok" | "norwegian krone" | "krone" => Some((self.nok, "kr".to_string())),
            "mxn" | "mexican peso" | "peso" => Some((self.mxn, "Mex$".to_string())),
            "sgd" | "singapore dollar" => Some((self.sgd, "S$".to_string())),
            "hkd" | "hong kong dollar" => Some((self.hkd, "HK$".to_string())),
            "krw" | "south korean won" | "won" => Some((self.krw, "₩".to_string())),
            _ => None,
        }
    }
    pub fn unit_exists(unit: &str) -> bool {
        let allowed = vec![
            "usd",
            "dollar",
            "us dollar",
            "bucks",
            "eur",
            "euro",
            "euros",
            "european euro",
            "jpy",
            "yen",
            "japanese yen",
            "gbp",
            "pound",
            "british pound",
            "pound sterling",
            "aud",
            "australian dollar",
            "aussie dollar",
            "aussie",
            "cad",
            "canadian dollar",
            "loonie",
            "chf",
            "swiss franc",
            "franc",
            "cny",
            "chinese yuan",
            "renminbi",
            "yuan",
            "nzd",
            "new zealand dollar",
            "kiwi",
            "sek",
            "swedish krona",
            "krona",
            "nok",
            "norwegian krone",
            "krone",
            "mxn",
            "mexican peso",
            "peso",
            "sgd",
            "singapore dollar",
            "hkd",
            "hong kong dollar",
            "krw",
            "south korean won",
            "won",
        ];
        allowed.contains(&unit.to_lowercase().as_str())
    }

    fn load_cached<P: AsRef<Path>>(loc: P, update_interval: u64) -> Option<Currency> {
        let absolute = loc.as_ref();
        if absolute.is_file() {
            let mtime = absolute.metadata().ok()?.modified().ok()?;
            let time_since = SystemTime::now().duration_since(mtime).ok()?;
            // then was cached
            if time_since < Duration::from_secs(60 * update_interval) {
                File::open(&absolute)
                    .ok()
                    .and_then(|file| simd_json::from_reader(file).ok())?
            }
        }
        None
    }
    fn cache<P: AsRef<Path>>(&self, loc: P) -> Result<(), SherlockError> {
        let absolute = loc.as_ref();
        if !absolute.is_file() {
            if let Some(parents) = absolute.parent() {
                create_dir_all(parents).map_err(|e| {
                    sherlock_error!(
                        SherlockErrorType::DirCreateError(String::from(
                            "~/.cache/sherlock/currency/"
                        )),
                        e.to_string()
                    )
                })?;
            }
        }
        let content = simd_json::to_string(self)
            .map_err(|e| sherlock_error!(SherlockErrorType::SerializationError, e.to_string()))?;
        std::fs::write(absolute, content).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileWriteError(absolute.to_path_buf()),
                e.to_string()
            )
        })
    }

    pub async fn get_exchange(update_interval: u64) -> Result<Currency, SherlockError> {
        let home = home_dir()?;
        let absolute = home.join(".cache/sherlock/currency/currency.json");
        match Currency::load_cached(&absolute, update_interval) {
            Some(curr) => return Ok(curr),
            _ => {}
        };

        let url = "https://scanner.tradingview.com/forex/scan?label-product=related-symbols";

        let json_body = r#"{
            "columns": [
                "name",
                "type",
                "close"
            ],
            "ignore_unknown_fields": true,
            "options": { "lang": "en" },
            "range": [0,14],
            "sort": {
                "sortBy": "popularity_rank",
                "sortOrder": "asc"
            },
            "filter2": {
                "operator": "and",
                "operands": [
                    { "expression": { "left": "type", "operation": "equal", "right": "forex" } },
                    { "expression": { "left": "exchange", "operation": "equal", "right": "FX_IDC" } },
                    { "expression": { "left": "currency_id", "operation": "equal", "right": "USD" } },
                    { "expression": { "left": "base_currency_id", "operation": "in_range", "right": ["EUR", "JPY", "GBP", "AUD", "CAD", "CHF", "CNY", "NZD", "SEK", "NOK", "MXN", "SGD", "HKD", "KRW"] } }
                ]
            }
        }"#;

        let client = reqwest::Client::new();
        let res = client
            .post(url)
            .header("Content-Type", "text/plain;charset=UTF-8")
            .header("Accept", "application/vnd.tv.rangedSelection.v1+json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0",
            )
            .header("Referer", "https://www.tradingview.com/")
            .header("Accept-Language", "en-US,en;q=0.5")
            .body(json_body)
            .send()
            .await
            .map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::HttpRequestError(String::from(
                        "GET tradingview.com || getting currencies"
                    )),
                    e.to_string()
                )
            })?;

        let body = res
            .text()
            .await
            .map_err(|e| sherlock_error!(SherlockErrorType::DeserializationError, e.to_string()))?;

        // simd-json requires &mut str
        let mut buf = body.into_bytes();
        let parsed: simd_json::OwnedValue = simd_json::to_owned_value(&mut buf)
            .map_err(|e| sherlock_error!(SherlockErrorType::DeserializationError, e.to_string()))?;

        let currencies: HashMap<String, f32> =
            if let Some(array) = parsed.get("data").and_then(OwnedValue::as_array) {
                array
                    .iter()
                    .filter_map(|item| {
                        let symbol = item.get("s")?.as_str()?;
                        let (_, pair) = symbol.split_once(":")?;
                        let (to, _from) = pair.split_at(3);
                        let price = item.get("d")?.as_array()?.get(2)?.as_f32()?;
                        Some((to.to_lowercase(), price as f32))
                    })
                    .collect()
            } else {
                HashMap::new()
            };

        match Currency::from_map(currencies) {
            Some(curr) => {
                curr.cache(absolute)?;
                Ok(curr)
            }
            _ => Err(sherlock_error!(
                SherlockErrorType::DeserializationError,
                String::from("Failed to deserialize currency map into 'Currency' object.")
            )),
        }
    }
}
