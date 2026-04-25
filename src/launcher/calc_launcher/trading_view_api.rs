use crate::{
    launcher::calc_launcher::{Currency, currency::CurrencyCode},
    sherlock_msg,
    utils::errors::{
        SherlockMessage,
        types::{NetworkAction, SherlockErrorType},
    },
};
use serde::{Deserialize, Deserializer, de::IgnoredAny};
use serde_json::json;

pub(super) struct TradingViewApiRequest;
impl TradingViewApiRequest {
    const URL: &str = "https://scanner.tradingview.com/forex/scan?label-product=related-symbols";
    fn get_headers() -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "text/plain;charset=UTF-8".parse().unwrap());
        headers.insert(
            "Accept",
            "application/vnd.tv.rangedSelection.v1+json"
                .parse()
                .unwrap(),
        );
        headers.insert(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0"
                .parse()
                .unwrap(),
        );
        headers.insert("Referer", "https://www.tradingview.com/".parse().unwrap());
        headers.insert("Accept-Language", "en-US,en;q=0.5".parse().unwrap());
        headers
    }
    fn build_body() -> serde_json::Value {
        let codes = Currency::iso_codes();
        json!({
            "columns": ["name", "type", "close"],
            "ignore_unknown_fields": true,
            "options": { "lang": "en" },
            "range": [0, codes.len()],
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
                {
                    "expression": {
                        "left": "base_currency_id",
                        "operation": "in_range",
                        "right": codes
                    }
                }
                ]
            }
        })
    }
    pub async fn fetch_exchange_data() -> Result<TradingViewApiResponse, SherlockMessage> {
        let client = reqwest::Client::new();
        let res = client
            .post(Self::URL)
            .headers(Self::get_headers())
            .body(Self::build_body().to_string())
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

        let mut bytes = res
            .bytes()
            .await
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::IO, e))?
            .to_vec();

        simd_json::from_slice(&mut bytes)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))
    }
}

#[derive(Deserialize, Debug)]
pub(super) struct TradingViewApiResponse {
    data: [DataItem; 16],
}
impl TradingViewApiResponse {
    pub(super) fn currency_items(self) -> impl Iterator<Item = CurrencyItem> {
        self.data.into_iter().map(|i| i.value)
    }
}

#[derive(Deserialize, Debug)]
struct DataItem {
    #[serde(rename = "d")]
    pub value: CurrencyItem,
}

#[derive(Debug)]
pub(super) struct CurrencyItem {
    pub space: CurrencyCode,
    pub factor: f32,
}

impl<'de> Deserialize<'de> for CurrencyItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (space, _, factor) = <(CurrencyCode, IgnoredAny, f32)>::deserialize(deserializer)?;
        Ok(CurrencyItem { space, factor })
    }
}
