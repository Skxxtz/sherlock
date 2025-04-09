use crate::CONFIG;
use regex::Regex;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct Calculator {
    pub capabilities: Option<HashSet<String>>,
}
impl Calculator {
    pub fn measurement(&self, keyword: &str, unit_str: &str) -> Option<String> {
        let full_pattern = r"(?i)(\d+(?:\.\d+)?)\s*([a-zA-Z]+)\s*(in|to)\s*([a-zA-Z]+)";
        let full_re = Regex::new(full_pattern).unwrap();
        if let Some(caps) = full_re.captures(keyword) {
            let value: f32 = caps[1].parse().ok()?;
            let from = caps[2].to_lowercase();
            let to = caps[4].to_lowercase();
            println!("{:?} → {:?}", from, to);

            let (factor_from, _) = Measurements::match_unit(&from, unit_str)?;
            let (factor_to, name) = Measurements::match_unit(&to, unit_str)?;

            let base = self.to_basis(factor_from, value);
            let res = self.to_unit(factor_to, base);
            let postfix = if res == 1.0 { "" } else { "s" };
            return Some(format!("= {:.2} {}{}", res, name, postfix));
        }
        // Support for partial ones
        let part_pattern = r"(?i)(\d+(?:\.\d+)?)\s*([a-zA-Z]+)";
        let part_re = Regex::new(part_pattern).unwrap();
        if let Some(caps) = part_re.captures(keyword) {
            let config = CONFIG.get()?;
            let value: f32 = caps[1].parse().ok()?;
            let from = caps[2].to_lowercase();
            let to = match unit_str {
                "weight" => config.units.weights.to_lowercase(),
                _ => config.units.lengths.to_lowercase()
            };
            println!("{:?}", to);

            let (factor_from, _) = Measurements::match_unit(&from, unit_str)?;
            let (factor_to, name) = Measurements::match_unit(&to, unit_str)?;

            let base = self.to_basis(factor_from, value);
            let res = self.to_unit(factor_to, base);
            let postfix = if res == 1.0 { "" } else { "s" };
            return Some(format!("= {:.2} {}{}", res, name, postfix));
        }
        None
    }
    fn to_basis(&self, factor: f32, value: f32) -> f32 {
        value * factor
    }
    fn to_unit(&self, factor: f32, value: f32) -> f32 {
        value / factor
    }
}




enum Measurements {}
impl Measurements {
    fn match_unit(unit:&str, unit_str: &str) -> Option<(f32, String)>{
        match unit_str {
            "weight" => Weight::match_unit(unit),
            _ => Length::match_unit(unit)
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

    fn match_unit(unit:&str) -> Option<(f32, String)> {
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

    fn match_unit(unit: &str) -> Option<(f32, String)> {
        match unit.to_lowercase().as_str() {
            "kilograms" | "kilogram" | "kg" => {
                Some((Weight::KILOGRAM, String::from("Kilogram")))
            }
            "grams" | "gram" | "g" => Some((Weight::GRAM, String::from("Gram"))),
            "milligrams" | "milligram" | "mg" => {
                Some((Weight::MILLIGRAM, String::from("Milligram")))
            }
            "pounds" | "pound" | "lbs" => {
                Some((Weight::POUND, String::from("Pound")))
            }
            "ounces" | "ounce" | "oz" => {
                Some((Weight::OUNCE, String::from("Ounce")))
            }

            _ => None,
        }
    }

}
