pub struct ColorConverter;

impl ColorConverter {
    pub fn normalize(from: &str, values: &[f32]) -> Option<u32> {
        let (r, g, b) = match from {
            "rgb" | "rgba" if values.len() >= 3 => (values[0], values[1], values[2]),
            "hex" if values.len() >= 3 => (values[0], values[1], values[2]),
            "hsl" if values.len() >= 3 => Self::hsl_to_rgb(values[0], values[1], values[2]),
            "hsv" if values.len() >= 3 => Self::hsv_to_rgb(values[0], values[1], values[2]),
            "lab" if values.len() >= 3 => Self::lab_to_rgb(values[0], values[1], values[2]),
            _ => return None,
        };

        let r = r.round() as u32;
        let g = g.round() as u32;
        let b = b.round() as u32;

        Some((r << 16) | (g << 8) | b)
    }
    pub fn convert(from: &str, values: &[f32], to: &str) -> Option<String> {
        let rgb = match from {
            "rgb" | "rgba" if values.len() >= 3 => Some((values[0], values[1], values[2])),
            "hex" => Some((values[0], values[1], values[2])),
            "hsl" if values.len() >= 3 => Some(Self::hsl_to_rgb(values[0], values[1], values[2])),
            "hsv" if values.len() >= 3 => Some(Self::hsv_to_rgb(values[0], values[1], values[2])),
            "lab" if values.len() >= 3 => Some(Self::lab_to_rgb(values[0], values[1], values[2])),
            _ => None,
        }?;

        // output format
        match to {
            "rgb" => Some(format!(
                "rgb({}, {}, {})",
                rgb.0.round(),
                rgb.1.round(),
                rgb.2.round()
            )),
            "hex" => Some(format!(
                "#{:02x}{:02x}{:02x}",
                rgb.0.round() as u8,
                rgb.1.round() as u8,
                rgb.2.round() as u8
            )),
            "hsl" => {
                let (h, s, l) = Self::rgb_to_hsl(rgb.0, rgb.1, rgb.2);
                Some(format!("hsl({:.0}, {:.0}%, {:.0}%)", h, s, l))
            }
            "hsv" => {
                let (h, s, v) = Self::rgb_to_hsv(rgb.0, rgb.1, rgb.2);
                Some(format!("hsv({:.0}, {:.0}%, {:.0}%)", h, s, v))
            }
            "lab" => {
                let (l, a, b) = Self::rgb_to_lab(rgb.0, rgb.1, rgb.2);
                Some(format!("lab({:.1}, {:.1}, {:.1})", l, a, b))
            }
            _ => None,
        }
    }
}

// --- hex conversions
impl ColorConverter {
    pub fn hex_to_rgb(hex: &str) -> Option<(f32, f32, f32)> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some(((r * 17) as f32, (g * 17) as f32, (b * 17) as f32))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some((r as f32, g as f32, b as f32))
            }
            _ => None,
        }
    }
}

// --- Hsl conversions
impl ColorConverter {
    fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
        let s = s / 100.0;
        let l = l / 100.0;
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r_prime, g_prime, b_prime) = match h as u32 {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        (
            (r_prime + m) * 255.0,
            (g_prime + m) * 255.0,
            (b_prime + m) * 255.0,
        )
    }

    fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let r = r / 255.0;
        let g = g / 255.0;
        let b = b / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let l = (max + min) / 2.0;
        let s = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };

        let mut h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        if h < 0.0 {
            h += 360.0;
        }
        (h, s * 100.0, l * 100.0)
    }
}

// --- Hsv conversions
impl ColorConverter {
    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
        let s = s / 100.0;
        let v = v / 100.0;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match (h as u32) / 60 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        ((r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0)
    }

    fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let r = r / 255.0;
        let g = g / 255.0;
        let b = b / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let s = if max == 0.0 { 0.0 } else { delta / max };
        let v = max;

        (if h < 0.0 { h + 360.0 } else { h }, s * 100.0, v * 100.0)
    }
}
// --- Lab conversions ---
impl ColorConverter {
    fn rgb_to_lab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        // 1. sRGB to XYZ
        let mut r = r / 255.0;
        let mut g = g / 255.0;
        let mut b = b / 255.0;
        let f = |n: f32| {
            if n > 0.04045 {
                ((n + 0.055) / 1.055).powf(2.4)
            } else {
                n / 12.92
            }
        };
        r = f(r);
        g = f(g);
        b = f(b);

        let x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
        let y = (r * 0.2126 + g * 0.7152 + b * 0.0722) / 1.00000;
        let z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;

        // 2. XYZ to LAB
        let f_xyz = |n: f32| {
            if n > 0.008856 {
                n.powf(1.0 / 3.0)
            } else {
                (7.787 * n) + (16.0 / 116.0)
            }
        };
        let lx = f_xyz(x);
        let ly = f_xyz(y);
        let lz = f_xyz(z);

        ((116.0 * ly) - 16.0, 500.0 * (lx - ly), 200.0 * (ly - lz))
    }

    fn lab_to_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
        let y = (l + 16.0) / 116.0;
        let x = a / 500.0 + y;
        let z = y - b / 200.0;

        let f_inv = |n: f32| {
            if n.powi(3) > 0.008856 {
                n.powi(3)
            } else {
                (n - 16.0 / 116.0) / 7.787
            }
        };
        let x = f_inv(x) * 0.95047;
        let y = f_inv(y) * 1.00000;
        let z = f_inv(z) * 1.08883;

        let r = x * 3.2406 + y * -1.5372 + z * -0.4986;
        let g = x * -0.9689 + y * 1.8758 + z * 0.0415;
        let b = x * 0.0557 + y * -0.2040 + z * 1.0570;

        let f_final = |n: f32| {
            if n > 0.0031308 {
                1.055 * n.powf(1.0 / 2.4) - 0.055
            } else {
                12.92 * n
            }
        };
        (
            f_final(r).clamp(0.0, 1.0) * 255.0,
            f_final(g).clamp(0.0, 1.0) * 255.0,
            f_final(b).clamp(0.0, 1.0) * 255.0,
        )
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;

    /// Helper to compare floats within a small margin of error
    fn assert_near(actual: f32, expected: f32, margin: f32) {
        assert!(
            (actual - expected).abs() <= margin,
            "Values not close enough: actual {}, expected {}",
            actual,
            expected
        );
    }

    #[test]
    fn test_rgb_pivot_logic() {
        // Test RGB to Hex
        let hex_res = ColorConverter::convert("rgb", &[255.0, 0.0, 0.0], "hex");
        assert_eq!(hex_res, Some("#ff0000".to_string()));

        // Test RGB to HSL (Red)
        let hsl_res = ColorConverter::convert("rgb", &[255.0, 0.0, 0.0], "hsl");
        assert_eq!(hsl_res, Some("hsl(0, 100%, 50%)".to_string()));
    }

    #[test]
    fn test_hsl_to_rgb_conversion() {
        // Pure Green in HSL: 120, 100%, 50%
        let rgb = ColorConverter::hsl_to_rgb(120.0, 100.0, 50.0);
        assert_near(rgb.0, 0.0, 0.1);
        assert_near(rgb.1, 255.0, 0.1);
        assert_near(rgb.2, 0.0, 0.1);
    }

    #[test]
    fn test_hsv_to_rgb_conversion() {
        // Pure Blue in HSV: 240, 100%, 100%
        let rgb = ColorConverter::hsv_to_rgb(240.0, 100.0, 100.0);
        assert_near(rgb.0, 0.0, 0.1);
        assert_near(rgb.1, 0.0, 0.1);
        assert_near(rgb.2, 255.0, 0.1);
    }

    #[test]
    fn test_lab_roundtrip() {
        // Start with RGB white
        let original_rgb = (255.0, 255.0, 255.0);
        let (l, a, b) = ColorConverter::rgb_to_lab(original_rgb.0, original_rgb.1, original_rgb.2);

        // Lab for white should be roughly 100, 0, 0
        assert_near(l, 100.0, 0.5);
        assert_near(a, 0.0, 0.5);
        assert_near(b, 0.0, 0.5);

        let back_to_rgb = ColorConverter::lab_to_rgb(l, a, b);
        assert_near(back_to_rgb.0, 255.0, 1.0);
        assert_near(back_to_rgb.1, 255.0, 1.0);
        assert_near(back_to_rgb.2, 255.0, 1.0);
    }

    #[test]
    fn test_cross_space_conversion() {
        // HSL -> LAB (Yellow)
        // hsl(60, 100%, 50%) -> rgb(255, 255, 0) -> lab(97.14, -21.55, 94.48)
        let result = ColorConverter::convert("hsl", &[60.0, 100.0, 50.0], "lab");
        assert!(result.is_some());
        let val = result.unwrap();
        // Check for approximate LAB values for yellow
        assert!(val.contains("lab(97.1"));
    }

    #[test]
    fn test_invalid_inputs() {
        // Not enough values
        assert_eq!(ColorConverter::convert("rgb", &[255.0, 0.0], "hex"), None);

        // Unknown space
        assert_eq!(
            ColorConverter::convert("cmyk", &[0.0, 0.0, 0.0, 0.0], "rgb"),
            None
        );

        // Target space mismatch
        assert_eq!(
            ColorConverter::convert("rgb", &[255.0, 0.0, 0.0], "invalid"),
            None
        );
    }

    #[test]
    fn test_clamping_and_rounding() {
        // Test values slightly out of bounds
        let res = ColorConverter::convert("rgb", &[255.6, -10.0, 300.0], "hex");
        // Should clamp to #ff00ff (or round appropriately)
        assert_eq!(res, Some("#ff00ff".to_string()));
    }
}
