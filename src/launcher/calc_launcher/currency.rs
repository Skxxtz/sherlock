use std::path::PathBuf;

use paste::paste;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use strum_macros::EnumString;

use crate::{
    launcher::calc_launcher::trading_view_api::{
        CurrencyItem, TradingViewApiRequest, TradingViewApiResponse,
    },
    utils::{cache::JsonCache, errors::SherlockMessage, files::home_dir},
};

macro_rules! currency_struct {
    ($($iso:ident),*) => {
        paste! {
            #[derive(Clone, Debug, Deserialize, Serialize, Default)]
            #[serde(rename_all = "lowercase")]
            pub struct Currency {
                pub usd: f32,
                $(pub $iso: f32,)*
            }

            #[derive(Debug, EnumIter, EnumString, PartialEq)]
            pub enum CurrencyCode {
                $( [< $iso:camel >], )*
            }

            impl<'de> serde::Deserialize<'de> for CurrencyCode {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    struct CurrencyVisitor;

                    impl<'de> serde::de::Visitor<'de> for CurrencyVisitor {
                        type Value = CurrencyCode;

                        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                            formatter.write_str("a currency code string")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                        where
                            E: serde::de::Error,
                        {
                            match value {
                                $(
                                    concat!(stringify!([< $iso:upper >]), "USD") => Ok(CurrencyCode::[< $iso:camel >]),
                                )*
                                _ => Err(serde::de::Error::unknown_variant(value, &["valid currency codes"])),
                            }
                        }
                    }

                    deserializer.deserialize_str(CurrencyVisitor)
                }
            }

            impl Currency {
                pub(super) fn set(&mut self, item: CurrencyItem) {
                    match item.space {
                        $( CurrencyCode::[< $iso:camel >] => self.$iso = item.factor, )*
                    }
                }
                pub(super) fn iso_codes() -> &'static [&'static str] {
                    &[ $( stringify!([< $iso:upper >]), )* ]
                }
            }
        }
    };
}
currency_struct!(
    eur, // Euro
    jpy, // Japanese Yen
    gbp, // British Pound Sterling
    aud, // Australian Dollar
    cad, // Canadian Dollar
    chf, // Swiss Franc
    cny, // Chinese Yuan
    nzd, // New Zealand Dollar
    sek, // Swedish Krona
    nok, // Norwegian Krona
    mxn, // Mexican Peso
    sgd, // Singapore Dollar
    hkd, // Hong Kong Dollar
    krw, // South Korean Won
    pln, // Polish złoty
    pen  // Peruvian Sole
);

// ---------- NORMAL IMPL ----------

impl From<TradingViewApiResponse> for Currency {
    fn from(value: TradingViewApiResponse) -> Self {
        let mut currency = Self::default();
        value.currency_items().for_each(|item| currency.set(item));
        currency
    }
}

impl JsonCache for Currency {
    fn cache_path() -> std::path::PathBuf {
        home_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".cache/sherlock/currency/currency.json")
    }
}

impl Currency {
    pub async fn get_exchange(update_interval: u64) -> Result<Currency, SherlockMessage> {
        if let Ok(Some(curr)) = Currency::read_from_cache(update_interval) {
            return Ok(curr);
        }

        // Fetch new data
        let currency_response: TradingViewApiResponse =
            TradingViewApiRequest::fetch_exchange_data().await?;
        let currency = Self::from(currency_response);
        currency.write_to_cache()?;

        Ok(currency)
    }
}
