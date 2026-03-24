use gpui::SharedString;
use smallvec::{SmallVec, smallvec};

use crate::{launcher::calc_launcher::CURRENCIES, utils::intent::colors::ColorConverter};

mod colors;

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    ColorConvert {
        from_space: &'static str,
        values: SmallVec<[f32; 4]>,
        to_space: &'static str,
    },
    ColorDisplay {
        from_space: &'static str,
        values: SmallVec<[f32; 4]>,
    },
    Conversion {
        value: f64,
        from: Unit,
        to: Unit,
    },
    Url {
        url: SharedString,
    },
    None,
}

impl Intent {
    #[inline]
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone)]
pub enum IntentResult {
    String(SharedString),
    Color(u32),
}
impl ToString for IntentResult {
    fn to_string(&self) -> String {
        match self {
            Self::String(s) => s.to_string(),
            Self::Color(hex) => format!("#{:06x}", hex),
        }
    }
}

impl Intent {
    pub fn execute(&self) -> Option<IntentResult> {
        match self {
            Intent::Conversion { value, from, to } => {
                // early return on domain mismatch
                if from.category() != to.category() {
                    return None;
                }

                if from.category() == UnitCategory::Currency && CURRENCIES.get().is_none() {
                    return Some(IntentResult::String(
                        "Loading exchange rates...".to_string().into(),
                    ));
                }

                // handle temperature (non-linear)
                if from.category() == UnitCategory::Temperature {
                    let result = match (from, to) {
                        (Unit::Celsius, Unit::Fahrenheit) => (value * 9.0 / 5.0) + 32.0,
                        (Unit::Fahrenheit, Unit::Celsius) => (value - 32.0) * 5.0 / 9.0,
                        _ => *value,
                    };
                    return Some(IntentResult::String(
                        format!("{:.1} {}", result, to.symbol()).into(),
                    ));
                }

                // handle linear
                // Formula: y = val * (from_factor / to_factor)
                let result = value * (from.factor() / to.factor());

                Some(IntentResult::String(self.format_result(result, to).into()))
            }
            Intent::ColorDisplay { from_space, values } => {
                ColorConverter::normalize(*from_space, values).map(IntentResult::Color)
            }
            Intent::ColorConvert {
                from_space,
                values,
                to_space,
            } => ColorConverter::convert(from_space, values, to_space)
                .map(|r| IntentResult::String(r.into())),
            _ => None,
        }
    }

    fn format_result(&self, result: f64, unit: &Unit) -> String {
        // Smart formatting based on magnitude
        let formatted = if result == 0.0 {
            "0".to_string()
        } else if result.abs() < 0.001 || result.abs() >= 1_000_000_000.0 {
            format!("{:.4e}", result) // Scientific notation for extreme sizes
        } else if result.fract() == 0.0 {
            format!("{:.0}", result) // No decimals if it's an integer
        } else {
            format!("{:.2}", result) // Standard 2 decimals
        };

        format!("{} {}", formatted, unit.symbol())
    }
}

impl Intent {
    pub fn parse(input: &str, caps: &Capabilities) -> Intent {
        let raw = input.trim();
        if raw.is_empty() {
            return Intent::None;
        }

        // Tokenization
        let mut tokens: SmallVec<[&str; 8]> = SmallVec::new();
        let bytes = raw.as_bytes();
        let mut last = 0;

        for i in 0..bytes.len() {
            let b = bytes[i];
            if matches!(b, b' ' | b'(' | b')' | b'%' | b',') {
                if last < i {
                    let word = raw[last..i].trim_matches(',');
                    Self::push_cleaned_token(&mut tokens, word);
                }
                last = i + 1;
            }
        }

        // last chunk
        if last < raw.len() {
            let word = &raw[last..].trim_matches(',');
            if !word.is_empty() {
                Self::push_cleaned_token(&mut tokens, word);
            }
        }

        // match intent
        if let Some(intent) = Intent::try_parse_color_conversion(&tokens, caps) {
            return intent;
        }

        if let Some(intent) = Intent::try_parse_unit_conversion(&tokens, caps) {
            return intent;
        }

        if let Some(intent) = Intent::try_parse_url(raw) {
            return intent;
        }

        Intent::None
    }

    #[inline]
    fn push_cleaned_token<'a>(tokens: &mut SmallVec<[&'a str; 8]>, word: &'a str) {
        let is_noise = matches!(word, w if
            w.eq_ignore_ascii_case("how") ||
            w.eq_ignore_ascii_case("much") ||
            w.eq_ignore_ascii_case("is") ||
            w.eq_ignore_ascii_case("are") ||
            w.eq_ignore_ascii_case("convert") ||
            w.eq_ignore_ascii_case("what")
        );

        if !is_noise {
            tokens.push(word);
        }
    }

    fn try_parse_color_conversion(tokens: &[&str], caps: &Capabilities) -> Option<Intent> {
        fn to_static_space(s: &str) -> Option<&'static str> {
            match s {
                "rgb" => Some("rgb"),
                "rgba" => Some("rgba"),
                "hex" => Some("hex"),
                "hsl" => Some("hsl"),
                "hsv" => Some("hsv"),
                "lab" => Some("lab"),
                _ => None,
            }
        }
        if !caps.allows(Capabilities::COLORS) {
            return None;
        }

        let spaces = ["rgb", "rgba", "hex", "hsl", "hsv", "lab"];

        let explicit_space_idx = tokens.iter().position(|t| spaces.contains(t));
        let (from_space, from_idx) = if tokens.first().map_or(false, |t| t.starts_with('#')) {
            ("hex", 0)
        } else if let Some(idx) = explicit_space_idx {
            (to_static_space(tokens[idx])?, idx)
        } else {
            return None;
        };

        // collect values — everything after the space name that isn't a connector
        let connector_idx = tokens.iter().position(|t| matches!(*t, "to" | "in" | "as"));

        let values_end = connector_idx.unwrap_or(tokens.len());

        let values: SmallVec<[f32; 4]> = if from_idx >= values_end {
            smallvec![]
        } else if from_space == "hex" || tokens.get(from_idx).map_or(false, |t| t.starts_with('#'))
        {
            let token = tokens.get(from_idx)?;
            if let Some((r, g, b)) = ColorConverter::hex_to_rgb(token) {
                smallvec![r, g, b]
            } else {
                return None;
            }
        } else {
            tokens[from_idx + 1..values_end]
                .iter()
                .filter_map(|t| t.parse::<f32>().ok())
                .collect()
        };

        if values.is_empty() {
            return None;
        }

        // no connector → ColorShow
        let Some(connector_idx) = connector_idx else {
            return Some(Intent::ColorDisplay { from_space, values });
        };

        // connector must be after space name
        if connector_idx <= from_idx {
            return None;
        }

        let to_space = to_static_space(tokens.get(connector_idx + 1)?)?;
        if spaces.contains(&to_space) {
            return Some(Intent::ColorConvert {
                from_space,
                values,
                to_space,
            });
        }

        None
    }

    fn try_parse_unit_conversion(tokens: &[&str], caps: &Capabilities) -> Option<Intent> {
        let connector_idx = tokens
            .iter()
            .position(|t| matches!(*t, "to" | "in" | "as"))?;

        let to_token = tokens.get(connector_idx + 1)?;

        let (value, from) = if connector_idx >= 2 {
            // Case: ["100", "kg", "to", "lbs"]
            let v = tokens[0].parse::<f64>().ok()?;
            let f = Unit::parse_with_capabilities(tokens[1], caps)?;
            (v, f)
        } else if connector_idx == 1 {
            let first = &tokens[0];
            let split_at = first.find(|c: char| !c.is_numeric() && c != '.' && c != ',');

            if let Some(idx) = split_at {
                // Case: ["100kg", "to", "lbs"]
                let (v_str, u_str) = first.split_at(idx);
                let v = v_str.replace(',', "").parse::<f64>().ok()?;
                let f = Unit::parse_with_capabilities(u_str, caps)?;
                (v, f)
            } else {
                // Case: ["$100", "to", "eur"]
                let first_char_len = first.chars().next()?.len_utf8();
                let (u_str, v_str) = first.split_at(first_char_len);
                let f = Unit::parse_with_capabilities(u_str, caps)?;
                let v = v_str.replace(',', "").parse::<f64>().ok()?;
                (v, f)
            }
        } else {
            return None;
        };

        let to = Unit::parse_in_category(to_token, from.category())?;

        Some(Intent::Conversion { value, from, to })
    }

    fn try_parse_url(input: &str) -> Option<Intent> {
        let s = input.trim();

        // explicit scheme
        if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://") {
            return Some(Intent::Url {
                url: s.to_string().into(),
            });
        }

        // no spaces + contains a dot + looks like a domain
        if !s.contains(' ') && s.contains('.') {
            // must have something before and after the dot
            let parts: Vec<&str> = s.splitn(2, '.').collect();
            if parts[0].len() >= 1 && parts[1].len() >= 2 {
                // avoid matching things like "50.0" (numbers) or ".hidden"
                let first = parts[0];
                let is_numeric = first.chars().all(|c| c.is_numeric());
                if !is_numeric && !first.is_empty() {
                    return Some(Intent::Url {
                        url: s.to_string().into(),
                    });
                }
            }
        }

        None
    }
}

macro_rules! define_units {
    ($(
        $category:ident, $cap_const:ident {
            cap: $cap_val:expr,
            $($variant:ident: [$($alias:literal),*] => $factor:expr, $canonical_symbol:literal),* $(,)?
        }
    )*) => {
        #[derive(PartialEq, Eq, Hash)]
        #[allow(dead_code)]
        pub enum UnitCategory { $($category),* }
        #[allow(dead_code)]
        impl UnitCategory {
            pub fn capability_mask(&self) -> u32 {
                match self {
                    $( UnitCategory::$category => Capabilities::$cap_const, )*
                }
            }
        }

        #[derive(Clone, Copy)]
        pub struct Capabilities(u32);
        #[allow(dead_code)]
        impl Capabilities {
            pub const NONE: u32 = 0;
            $( pub const $cap_const: u32 = $cap_val; )*
            pub const EVERYTHING: u32 = u32::MAX;

            #[inline]
            pub fn allows(&self, cap: u32) -> bool {
                (self.0 & cap) != 0
            }
        }

        impl std::fmt::Debug for Capabilities {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut active: Vec<&'static str> = Vec::new();
                $( if self.allows(Self::$cap_const) { active.push(stringify!($cap_const)); } )*
                write!(f, "Capabilities({})", active.join(" | "))
            }
        }

        impl std::ops::BitOr for Capabilities {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitOrAssign<u32> for Capabilities {
            fn bitor_assign(&mut self, rhs: u32) {
                self.0 |= rhs;
            }
        }

        #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub enum Unit {
            $( $( $variant, )* )*
        }

        impl Unit {
            pub fn category(&self) -> UnitCategory {
                match self {
                    $( $(Unit::$variant => UnitCategory::$category,)* )*
                }
            }

            pub fn symbol(&self) -> &'static str {
                match self {
                    $( $(Unit::$variant => $canonical_symbol,)* )*
                }
            }

            // The raw factor (for static units)
            fn raw_factor(&self) -> f64 {
                match self {
                    $( $(Unit::$variant => $factor as f64,)* )*
                }
            }

            fn parse_in_category(s: &str, cat: UnitCategory) -> Option<Self> {
                let s = s.trim().to_lowercase();
                if s.is_empty() { return None; } // Guard against empty strings

                match cat {
                    $(
                        UnitCategory::$category => {
                            // 1. Exact Match Path
                            $(
                                if [$($alias),*].contains(&s.as_str()) {
                                    return Some(Unit::$variant);
                                }
                            )*
                                if s.len() >= 2 {
                                    $(
                                        for alias in [$($alias),*] {
                                            if alias.starts_with(&s) {
                                                return Some(Unit::$variant);
                                            }
                                        }
                                    )*
                                }
                            None
                        },
                    )*
                }
            }

            pub fn parse_with_capabilities(s: &str, caps: &Capabilities) -> Option<Self> {
                let s = s.trim();
                if s.is_empty() { return None; }
                let s_lower = s.to_lowercase();
                let s_ptr = s_lower.as_str();

                $(
                    if caps.allows(Capabilities::$cap_const) {
                        $(
                            if [$($alias),*].contains(&s_ptr) {
                                return Some(Unit::$variant);
                            }
                        )*
                    }
                )*

                    if s_lower.len() >= 3 {
                        $(
                            if caps.allows(Capabilities::$cap_const) {
                                $(
                                    for alias in [$($alias),*] {
                                        if alias.len() > s_lower.len() && alias.starts_with(&s_lower) {
                                            return Some(Unit::$variant);
                                        }
                                    }
                                )*
                            }
                        )*
                    }

                None
            }
        }
    };
}
impl Unit {
    pub fn factor(&self) -> f64 {
        // use dynamic factors for currencies
        if self.category() == UnitCategory::Currency {
            if let Some(Some(rates)) = CURRENCIES.get() {
                let rate = match self {
                    Unit::Usd => rates.usd,
                    Unit::Eur => rates.eur,
                    Unit::Jpy => rates.jpy,
                    Unit::Gbp => rates.gbp,
                    Unit::Aud => rates.aud,
                    Unit::Cad => rates.cad,
                    Unit::Chf => rates.chf,
                    Unit::Cny => rates.cny,
                    Unit::Nzd => rates.nzd,
                    Unit::Sek => rates.sek,
                    Unit::Nok => rates.nok,
                    Unit::Mxn => rates.mxn,
                    Unit::Sgd => rates.sgd,
                    Unit::Hkd => rates.hkd,
                    Unit::Krw => rates.krw,
                    Unit::Pln => rates.pln,
                    _ => 1.0,
                };
                return 1.0 / rate as f64;
            }
        }
        // use hardcoded factor
        self.raw_factor()
    }
}
impl Capabilities {
    pub fn from_strings(strs: &[String]) -> Self {
        let mut mask = Self::NONE;
        for s in strs {
            mask |= match s.as_str() {
                "calc.currencies" => Self::CURRENCY,
                "calc.math" => Self::MATH,
                "colors" => Self::COLORS,

                // all units
                "calc.units" => {
                    Self::LENGTH
                        | Self::VOLUME
                        | Self::WEIGHT
                        | Self::TEMPERATURE
                        | Self::PRESSURE
                        | Self::DIGITAL
                        | Self::TIME
                        | Self::AREA
                        | Self::SPEED
                }

                // individual units
                "calc.length" => Self::LENGTH,
                "calc.volume" => Self::VOLUME,
                "calc.weight" => Self::WEIGHT,
                "calc.temperature" => Self::TEMPERATURE,
                "calc.pressure" => Self::PRESSURE,
                "calc.digital" => Self::DIGITAL,
                "calc.time" => Self::TIME,
                "calc.area" => Self::AREA,
                "calc.speed" => Self::SPEED,

                _ => Self::NONE,
            }
        }

        Self(mask)
    }
}

define_units! {
    Math, MATH {
        cap: 1 << 0,
    }
    Colors, COLORS {
        cap: 1 << 1,
    }
    Currency, CURRENCY {
        cap: 1 << 2,
        Usd: ["usd", "dollar", "dollars", "bucks", "$"] => 1.0, "$",
        Eur: ["eur", "euro", "euros", "€"] => 1.0, "€",
        Jpy: ["jpy", "yen", "japanese yen", "¥"] => 1.0, "¥",
        Gbp: ["gbp", "pound", "pounds", "sterling", "£"] => 1.0, "£",
        Aud: ["aud", "australian dollar", "aussie", "a$"] => 1.0, "A$",
        Cad: ["cad", "canadian dollar", "loonie", "c$"] => 1.0, "C$",
        Chf: ["chf", "swiss franc", "franc"] => 1.0, "CHF",
        Cny: ["cny", "chinese yuan", "renminbi", "yuan"] => 1.0, "¥",
        Nzd: ["nzd", "new zealand dollar", "kiwi", "nz$"] => 1.0, "NZ$",
        Sek: ["sek", "swedish krona", "krona", "kr"] => 1.0, "kr",
        Nok: ["nok", "norwegian krone", "krone"] => 1.0, "kr",
        Mxn: ["mxn", "mexican peso", "peso", "mex$"] => 1.0, "Mex$",
        Sgd: ["sgd", "singapore dollar", "s$"] => 1.0, "S$",
        Hkd: ["hkd", "hong kong dollar", "hk$"] => 1.0, "HK$",
        Krw: ["krw", "south korean won", "won", "₩"] => 1.0, "₩",
        Pln: ["pln", "polish", "złoty", "zł"] => 1.0, "zł",
    }
    Length, LENGTH {
        cap: 1 << 3,
        Millimeter: ["mm", "millimeter", "millimeters"] => 0.001, "mm",
        Centimeter: ["cm", "centimeter", "centimeters"] => 0.01, "cm",
        Meter: ["m", "meter", "meters"] => 1.0, "m",
        Kilometer: ["km", "kilometer", "kilometers", "kilos"] => 1000.0, "km",
        Inch: ["in", "inch", "inches", "\""] => 0.0254, "in",
        Feet: ["ft", "feet", "foot", "'"] => 0.3048, "ft",
        Yard: ["yd", "yard", "yards"] => 0.9144, "yd",
        Mile: ["mi", "mile", "miles"] => 1609.34, "mi",
        NauticalMile: ["nm", "nautical mile"] => 1852.0, "nmi",
    }
    Volume, VOLUME {
        cap: 1 << 4,
        Milliliter: ["ml", "milliliter", "milliliters", "cc"] => 0.001, "ml",
        Centiliter: ["cl", "centiliter"] => 0.01, "cl",
        Liter: ["l", "liter", "liters"] => 1.0, "l",
        Kiloliter: ["kl", "kiloliter"] => 1000.0, "kl",
        CubicMeter: ["m3", "cubic meter", "cubic meters"] => 1000.0, "m³",
        // US Liquid
        Teaspoon: ["tsp", "teaspoon"] => 0.00492892, "tsp",
        Tablespoon: ["tbsp", "tablespoon"] => 0.0147868, "tbsp",
        FluidOunce: ["fl oz", "fluid ounce", "fluid ounces"] => 0.0295735, "fl oz",
        Cup: ["cup", "cups"] => 0.236588, "cup",
        Pint: ["pt", "pint", "pints"] => 0.473176, "pt",
        Quart: ["qt", "quart", "quarts"] => 0.946353, "qt",
        Gallon: ["gal", "gallon", "gallons"] => 3.78541, "gal",
        // Imperial
        ImperialGallon: ["imp gal"] => 4.54609, "imp gal",
    }
    Weight, WEIGHT {
        cap: 1 << 5,
        Milligram: ["mg", "milligram", "milligrams"] => 0.000001, "mg",
        Gram: ["g", "gram", "grams"] => 0.001, "g",
        Kilogram: ["kg", "kilogram", "kilograms", "kilo", "kilos"] => 1.0, "kg",
        MetricTon: ["t", "tonne", "metric ton", "metric tons"] => 1000.0, "t",
        // Imperial/US
        Ounce: ["oz", "ounce", "ounces"] => 0.0283495, "oz",
        Pound: ["lb", "lbs", "pound", "pounds"] => 0.453592, "lb",
        Stone: ["st", "stone", "stones"] => 6.35029, "st",
        ShortTon: ["ton", "tons", "us ton"] => 907.185, "ton",
        LongTon: ["imperial ton", "uk ton"] => 1016.05, "ton",
        // Precious Metals
        TroyOunce: ["ozt", "troy ounce", "troy ounces"] => 0.0311035, "ozt",
    }
    Temperature, TEMPERATURE {
        cap: 1 << 6,
        Celsius: ["c", "celsius", "°c", "°"] => 1.0, "°C",
        Fahrenheit: ["f", "fahrenheit", "°f"] => 1.0, "°F",
    }
    Pressure, PRESSURE {
        cap: 1 << 7,
        Pascal: ["pa", "pascal", "pascals"] => 0.00001, "Pa",
        Kilopascal: ["kpa", "kilopascal"] => 0.01, "kPa",
        Bar: ["bar", "bars"] => 1.0, "bar",
        Atmosphere: ["atm", "atmosphere", "atmospheres"] => 1.01325, "atm",
        Psi: ["psi", "pounds per square inch"] => 0.06894757, "psi",
        Torr: ["torr", "mmhg"] => 0.00133322, "mmHg",
    }
    Digital, DIGITAL {
        cap: 1 << 8,
        Bit: ["bit", "bits", "b"] => 0.125, "bit",
        Kilobit: ["kb", "kilobit"] => 128.0, "kb",
        Megabit: ["mb", "megabit"] => 131072.0, "Mb",
        Gigabit: ["gb", "gigabit"] => 134217728.0, "Gb",
        Byte: ["byte", "bytes", "B"] => 1.0, "B",
        Kilobyte: ["kb", "kilobyte", "KB"] => 1024.0, "KB",
        Megabyte: ["mb", "megabyte", "MB"] => 1048576.0, "MB",
        Gigabyte: ["gb", "gigabyte", "GB"] => 1073741824.0, "GB",
        Terabyte: ["tb", "terabyte", "TB"] => 1099511627776.0, "TB",
        Petabyte: ["pb", "petabyte", "PB"] => 1125899906842624.0, "PB",
    }
    Time, TIME {
        cap: 1 << 9,
        Milliseconds: ["ms", "millisecond", "milliseconds"] => 0.001, "ms",
        Seconds: ["s", "sec", "second", "seconds"] => 1.0, "s",
        Minutes: ["min", "minute", "minutes"] => 60.0, "min",
        Hours: ["h", "hr", "hour", "hours"] => 3600.0, "h",
        Days: ["d", "day", "days"] => 86400.0, "d",
        Weeks: ["wk", "week", "weeks"] => 604800.0, "wk",
        Months: ["mo", "month", "months"] => 2629746.0, "mo",
        Years: ["yr", "year", "years"] => 31556952.0, "yr",
    }
    Area, AREA {
        cap: 1 << 10,
        SquareMeter: ["m2", "sq m", "sq meter"] => 1.0, "m²",
        SquareKilometer: ["km2", "sq km"] => 1000000.0, "km²",
        SquareFoot: ["ft2", "sq ft", "sq feet"] => 0.092903, "ft²",
        SquareInch: ["in2", "sq in"] => 0.00064516, "in²",
        Acre: ["acre", "acres"] => 4046.86, "ac",
        Hectare: ["ha", "hectare"] => 10000.0, "ha",
    }
    Speed, SPEED {
        cap: 1 << 11,
        MetersPerSecond: ["ms", "m/s", "meters per second"] => 1.0, "m/s",
        KilometersPerHour: ["kmh", "km/h", "kph"] => 0.277778, "km/h",
        MilesPerHour: ["mph", "mile per hour", "miles per hour"] => 0.44704, "mph",
        Knot: ["kn", "knot", "knots"] => 0.514444, "kn",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intents() {
        let caps = Capabilities(Capabilities::EVERYTHING);
        let cases = vec![
            // --- Basic Units ---
            (
                "50 meters to feet",
                Intent::Conversion {
                    value: 50.0,
                    from: Unit::parse_with_capabilities("meters", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("feet", &caps).unwrap(),
                },
            ),
            (
                "50m in yards",
                Intent::Conversion {
                    value: 50.0,
                    from: Unit::parse_with_capabilities("m", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("yards", &caps).unwrap(),
                },
            ),
            (
                "10.5 eur as usd",
                Intent::Conversion {
                    value: 10.5,
                    from: Unit::parse_with_capabilities("eur", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("usd", &caps).unwrap(),
                },
            ),
            (
                "convert 100 kg to lbs",
                Intent::Conversion {
                    value: 100.0,
                    from: Unit::parse_with_capabilities("kg", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("lbs", &caps).unwrap(),
                },
            ),
            (
                "how much is 500 miles in km",
                Intent::Conversion {
                    value: 500.0,
                    from: Unit::parse_with_capabilities("miles", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("km", &caps).unwrap(),
                },
            ),
            (
                "what is 1.5 atmospheres in psi",
                Intent::Conversion {
                    value: 1.5,
                    from: Unit::parse_with_capabilities("atmospheres", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("psi", &caps).unwrap(),
                },
            ),
            // --- No-Space & Unit Variations ---
            (
                "32c to f",
                Intent::Conversion {
                    value: 32.0,
                    from: Unit::parse_with_capabilities("c", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("f", &caps).unwrap(),
                },
            ),
            (
                "100km to miles",
                Intent::Conversion {
                    value: 100.0,
                    from: Unit::parse_with_capabilities("km", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("miles", &caps).unwrap(),
                },
            ),
            (
                "0.5in as cm",
                Intent::Conversion {
                    value: 0.5,
                    from: Unit::parse_with_capabilities("in", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("cm", &caps).unwrap(),
                },
            ),
            // --- Colors ---
            (
                "rgb(255, 0, 0) to hex",
                Intent::ColorConvert {
                    from_space: "rgb",
                    values: smallvec![255.0, 0.0, 0.0],
                    to_space: "hex",
                },
            ),
            (
                "hsl(360, 100%, 50%) in rgb",
                Intent::ColorConvert {
                    from_space: "hsl",
                    values: smallvec![360.0, 100.0, 50.0],
                    to_space: "rgb",
                },
            ),
            // Lazy color entry
            (
                "#ff0000 in rgb",
                Intent::ColorConvert {
                    from_space: "hex",
                    values: smallvec![255.0, 0., 0.],
                    to_space: "rgb",
                },
            ),
            (
                "rgb 255 255 255 as hsl",
                Intent::ColorConvert {
                    from_space: "rgb",
                    values: smallvec![255.0, 255.0, 255.0],
                    to_space: "hsl",
                },
            ),
            // --- Messy Input ---
            (
                "   50m   to   ft  ",
                Intent::Conversion {
                    value: 50.0,
                    from: Unit::parse_with_capabilities("m", &caps).unwrap(),
                    to: Unit::parse_with_capabilities("ft", &caps).unwrap(),
                },
            ),
            ("Convert 1,000 to hex", Intent::None),
            ("50.0.0 to m", Intent::None),
            // --- Fallbacks ---
            ("firefox", Intent::None),
            (
                "google.com",
                Intent::Url {
                    url: "google.com".to_string().into(),
                },
            ),
            ("show me the weather", Intent::None),
        ];

        for (input, expected) in cases {
            let result = Intent::parse(input, &caps);
            assert_eq!(
                result, expected,
                "Failed on input: '{}'\nGot: {:?}\nExpected: {:?}",
                input, result, expected
            );
        }
    }
}
