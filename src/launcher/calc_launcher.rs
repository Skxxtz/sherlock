use crate::CONFIG;
use regex::Regex;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct CalculatorLauncher {
    pub capabilities: Option<HashSet<String>>,
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
            let postfix = if res == 1.0 { "" } else { "s" };
            return Some((res.to_string(), format!("= {:.2} {}{}", res, name, postfix)));
        }
        // Support for partial ones
        let part_pattern = r"(?i)(\d+(?:\.\d+)?)\s*([a-zA-Z]+)";
        let part_re = Regex::new(part_pattern).unwrap();
        if let Some(caps) = part_re.captures(keyword) {
            let config = CONFIG.get()?;
            let value: f32 = caps[1].parse().ok()?;
            let from = caps[2].to_lowercase();
            let to = match unit_str {
                "weights" => config.units.weights.to_lowercase(),
                "volumes" => config.units.volumes.to_lowercase(),
                _ => config.units.lengths.to_lowercase(),
            };

            let (factor_from, _) = Measurements::match_unit(&from, unit_str)?;
            let (factor_to, name) = Measurements::match_unit(&to, unit_str)?;

            let base = Calculator::to_basis(factor_from, value);
            let res = Calculator::to_unit(factor_to, base);
            let postfix = if res == 1.0 { "" } else { "s" };
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
