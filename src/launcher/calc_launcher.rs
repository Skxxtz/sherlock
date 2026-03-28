use crate::{
    launcher::{
        LauncherProvider, LauncherType,
        children::{RenderableChild, calc_data::CalcData},
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    utils::{
        errors::{
            SherlockMessage,
            types::{DirAction, FileAction, NetworkAction, SherlockErrorType},
        },
        files::home_dir,
        intent::Capabilities,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simd_json::{
    OwnedValue,
    base::{ValueAsArray, ValueAsScalar},
    derived::ValueObjectAccess,
};
use std::{
    collections::HashMap,
    fs::{File, create_dir_all},
    path::Path,
    sync::OnceLock,
    time::{Duration, SystemTime},
};

#[derive(Clone, Debug)]
pub struct CalculatorLauncher {}

impl LauncherProvider for CalculatorLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        // initialize currencies
        let update_interval = raw
            .args
            .get("currency_update_interval")
            .and_then(|interval| interval.as_u64())
            .unwrap_or(60 * 60 * 24);

        tokio::spawn(async move {
            match Currency::get_exchange(update_interval).await {
                Ok(r) => {
                    let _result = CURRENCIES.set(Some(r));
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            }
        });

        LauncherType::Calculator(CalculatorLauncher {})
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::Launcher>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
    ) -> Result<Vec<super::children::RenderableChild>, SherlockMessage> {
        let capabilities: Vec<String> = match opts.get("capabilities") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => vec![String::from("calc.math"), String::from("calc.units")],
        };
        let caps = Capabilities::from_strings(&capabilities);
        let inner = CalcData::new(caps);

        Ok(vec![RenderableChild::CalcLike { launcher, inner }])
    }
}

pub static CURRENCIES: OnceLock<Option<Currency>> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize)]
pub struct Currency {
    pub usd: f32, // US Dollar
    pub eur: f32, // Euro
    pub jpy: f32, // Japanese Yen
    pub gbp: f32, // British Pound Sterling
    pub aud: f32, // Australian Dollar
    pub cad: f32, // Canadian Dollar
    pub chf: f32, // Swiss Franc
    pub cny: f32, // Chinese Yuan
    pub nzd: f32, // New Zealand Dollar
    pub sek: f32, // Swedish Krona
    pub nok: f32, // Norwegian Krone
    pub mxn: f32, // Mexican Peso
    pub sgd: f32, // Singapore Dollar
    pub hkd: f32, // Hong Kong Dollar
    pub krw: f32, // South Korean Won
    pub pln: f32, // Polish złoty
    pub pen: f32, // Peruvian Sole
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
            pln: map.remove("pln")?,
            pen: map.remove("pen")?,
        })
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
    fn cache<P: AsRef<Path>>(&self, loc: P) -> Result<(), SherlockMessage> {
        let absolute = loc.as_ref();
        if !absolute.is_file() {
            if let Some(parents) = absolute.parent() {
                create_dir_all(parents).map_err(|e| {
                    sherlock_msg!(
                        Warning,
                        SherlockErrorType::DirError(DirAction::Create, parents.to_path_buf()),
                        e
                    )
                })?;
            }
        }
        let content = simd_json::to_string(self)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;
        std::fs::write(absolute, content).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, absolute.to_path_buf()),
                e
            )
        })
    }

    pub async fn get_exchange(update_interval: u64) -> Result<Currency, SherlockMessage> {
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
            "range": [0,16],
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
                    { "expression": { "left": "base_currency_id", "operation": "in_range", "right": ["EUR", "JPY", "GBP", "AUD", "CAD", "CHF", "CNY", "NZD", "SEK", "NOK", "MXN", "SGD", "HKD", "KRW", "PLN", "PEN"] } }
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
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::NetworkError(
                        NetworkAction::Get,
                        "TradingView (Currencies)".into()
                    ),
                    e
                )
            })?;

        let body = res
            .text()
            .await
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;

        // simd-json requires &mut str
        let mut buf = body.into_bytes();
        let parsed: simd_json::OwnedValue = simd_json::to_owned_value(&mut buf)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;

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
            _ => Err(sherlock_msg!(
                Warning,
                SherlockErrorType::DeserializationError,
                "Failed to deserialize currency map into 'Currency' object."
            )),
        }
    }
}
