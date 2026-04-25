use crate::{
    launcher::{LauncherProvider, LauncherType},
    loader::utils::RawLauncher,
    ui::widgets::{RenderableChild, calculator::CalcData},
    utils::{errors::SherlockMessage, intent::Capabilities},
};
use serde_json::Value;
use std::sync::OnceLock;

mod currency;
mod trading_view_api;

pub use currency::Currency;

pub static CURRENCIES: OnceLock<Option<Currency>> = OnceLock::new();

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
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let capabilities: Vec<String> = match opts.get("capabilities") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => vec![String::from("calc.math"), String::from("calc.units")],
        };
        let caps = Capabilities::from_strings(&capabilities);
        let inner = CalcData::new(caps);

        Ok(vec![RenderableChild::Calc { launcher, inner }])
    }
}
