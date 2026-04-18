use std::sync::Arc;

use gpui::{Hsla, SharedString, hsla};

#[derive(Clone)]
pub struct ActiveTheme(pub Arc<ThemeData>);

pub struct ThemeData {
    pub font_family: SharedString,
    pub monospace: SharedString,
    // Cursor and Selection
    pub cursor: Hsla,
    pub selection: Hsla,
    // Backgrounds
    pub bg_app: Hsla,
    pub bg_selected: Hsla,
    pub bg_idle: Hsla,
    pub bg_status_bar: Hsla,
    pub bg_keybind: Hsla,
    pub bg_code: Hsla,
    pub bg_muted: Hsla,
    pub bg_badge: Hsla,
    // Borders
    pub border: Hsla,
    pub border_selected: Hsla,
    pub border_idle: Hsla,
    // Text
    pub primary_text: Hsla,
    pub secondary_text: Hsla,
    pub tertiary_text: Hsla,
    pub text_status_bar: Hsla,
    pub text_mode_label: Hsla,
    pub text_search_icon: Hsla,
    pub text_code: Hsla,
    pub text_placeholder: Hsla,
    // Status colors
    pub color_warn: Hsla,
    pub color_err: Hsla,
    pub color_succ: Hsla,
    // Config banner
    pub banner_bg: Hsla,
    pub banner_border: Hsla,
    pub banner_text: Hsla,
}
impl gpui::Global for ActiveTheme {}

impl ThemeData {
    pub fn dark() -> Self {
        Self {
            font_family: "Inter".into(),
            monospace: "Noto Sans Mono".into(),
            // Cursor and Selection
            cursor: hsla(0.0, 0.0, 0.8, 1.0),
            selection: hsla(0.639, 1.0, 0.53, 0.19),
            // Backgrounds
            bg_app: hsla(0.0, 0.0, 0.059, 1.0), // 0x0F0F0F
            bg_selected: hsla(0.0, 0.0, 1.0, 0.1),
            bg_idle: hsla(0.0, 0.0, 0.0, 0.0),
            bg_status_bar: hsla(0.0, 0.0, 0.098, 1.0),
            bg_keybind: hsla(0.0, 0.0, 0.149, 1.0), // 0x262626
            bg_code: hsla(0.0, 0.0, 0.118, 1.0),    // 0x1e1e1e
            bg_muted: hsla(0.0, 0.0, 1.0, 0.05),
            bg_badge: hsla(0.0, 0.0, 1.0, 0.08),
            // Borders
            border: hsla(0.0, 0.0, 0.1882, 1.0),
            border_selected: hsla(0.0, 0.0, 1.0, 0.2),
            border_idle: hsla(0.0, 0.0, 1.0, 0.05),
            // Text
            primary_text: hsla(0.0, 0.0, 0.95, 1.0),
            secondary_text: hsla(0.0, 0.0, 0.6, 1.0),
            tertiary_text: hsla(0.0, 0.0, 0.35, 1.0),
            text_status_bar: hsla(0.6, 0.0217, 0.3608, 1.0),
            text_mode_label: hsla(0.0, 0.0, 0.18, 1.0), // 0x2e2e2e
            text_search_icon: hsla(0.0, 0.0, 0.533, 1.0), // 0x888888
            text_code: hsla(0.558, 0.85, 0.76, 1.0),    // 0x89d4f5
            text_placeholder: hsla(0.0, 0.0, 1.0, 0.2),
            // Status
            color_warn: hsla(45.0 / 360.0, 0.85, 0.65, 1.0),
            color_err: hsla(0.0, 0.85, 0.65, 1.0),
            color_succ: hsla(145.0 / 360.0, 0.75, 0.60, 1.0),
            // Config banner
            banner_bg: hsla(0.11, 0.8, 0.12, 1.0),
            banner_border: hsla(0.11, 0.9, 0.35, 1.0),
            banner_text: hsla(0.11, 1.0, 0.65, 1.0),
        }
    }

    pub fn libre() -> Self {
        Self {
            font_family: "Inter".into(),
            monospace: "Noto Sans Mono".into(),
            // Cursor and Selection - Sophisticated Indigo/Slate
            cursor: hsla(220.0 / 360.0, 0.20, 0.30, 1.0),
            selection: hsla(220.0 / 360.0, 0.40, 0.50, 0.12),

            // Backgrounds - Warm Paper & Alabaster
            bg_app: hsla(40.0 / 360.0, 0.10, 0.98, 1.0), // Off-white/Bone 0xFCFAF8
            bg_selected: hsla(220.0 / 360.0, 0.15, 0.92, 1.0), // Soft Blue-tinted White
            bg_idle: hsla(0.0, 0.0, 0.0, 0.0),
            bg_status_bar: hsla(40.0 / 360.0, 0.05, 0.94, 1.0), // Slightly darker paper
            bg_keybind: hsla(0.0, 0.0, 0.90, 1.0),              // Light Gray
            bg_code: hsla(210.0 / 360.0, 0.20, 0.96, 1.0),      // Clean Code Background
            bg_badge: hsla(220.0 / 360.0, 0.10, 0.10, 0.05),
            bg_muted: hsla(0.0, 0.0, 0.0, 0.0),

            // Borders - Very Thin and Subtle
            border: hsla(0.0, 0.0, 0.85, 1.0), // Light stroke
            border_selected: hsla(220.0 / 360.0, 0.30, 0.70, 1.0),
            border_idle: hsla(0.0, 0.0, 0.90, 1.0),

            // Text - Deep Ink and Slate
            primary_text: hsla(220.0 / 360.0, 0.25, 0.15, 1.0), // Deep "Ink" Blue-Black
            secondary_text: hsla(220.0 / 360.0, 0.10, 0.45, 1.0), // Muted Slate
            tertiary_text: hsla(220.0 / 360.0, 0.08, 0.60, 1.0),
            text_status_bar: hsla(220.0 / 360.0, 0.15, 0.40, 1.0),
            text_mode_label: hsla(0.0, 0.0, 0.20, 1.0),
            text_search_icon: hsla(0.0, 0.0, 0.50, 1.0),
            text_code: hsla(220.0 / 360.0, 0.60, 0.40, 1.0), // Professional Blue
            text_placeholder: hsla(0.0, 0.0, 0.0, 0.25),

            // Status - Classy, desaturated tones
            color_warn: hsla(38.0 / 360.0, 0.60, 0.45, 1.0), // Ochre / Gold
            color_err: hsla(0.0, 0.50, 0.50, 1.0),           // Soft Carmine
            color_succ: hsla(150.0 / 360.0, 0.40, 0.45, 1.0), // Sage Green

            // Config banner - Light Champagne/Gold
            banner_bg: hsla(45.0 / 360.0, 0.30, 0.92, 1.0),
            banner_border: hsla(45.0 / 360.0, 0.40, 0.80, 1.0),
            banner_text: hsla(45.0 / 360.0, 0.60, 0.35, 1.0),
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self {
            font_family: "Inter".into(),
            monospace: "Noto Sans Mono".into(),
            cursor: hsla(267.0 / 360.0, 0.84, 0.81, 1.0),
            selection: hsla(267.0 / 360.0, 0.84, 0.81, 0.2),
            bg_app: hsla(240.0 / 360.0, 0.21, 0.12, 1.0),
            bg_selected: hsla(248.0 / 360.0, 0.15, 0.22, 1.0),
            bg_idle: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
            bg_status_bar: hsla(240.0 / 360.0, 0.21, 0.10, 1.0),
            bg_keybind: hsla(240.0 / 360.0, 0.21, 0.18, 1.0),
            bg_code: hsla(240.0 / 360.0, 0.21, 0.17, 1.0),
            bg_badge: hsla(248.0 / 360.0, 0.15, 0.22, 0.6),
            bg_muted: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
            border: hsla(240.0 / 360.0, 0.21, 0.25, 1.0),
            border_selected: hsla(267.0 / 360.0, 0.84, 0.81, 0.3),
            border_idle: hsla(240.0 / 360.0, 0.21, 0.22, 1.0),
            primary_text: hsla(226.0 / 360.0, 0.64, 0.88, 1.0),
            secondary_text: hsla(228.0 / 360.0, 0.24, 0.57, 1.0),
            tertiary_text: hsla(228.0 / 360.0, 0.15, 0.40, 1.0),
            text_status_bar: hsla(228.0 / 360.0, 0.24, 0.40, 1.0),
            text_mode_label: hsla(228.0 / 360.0, 0.24, 0.30, 1.0),
            text_search_icon: hsla(228.0 / 360.0, 0.24, 0.45, 1.0),
            text_code: hsla(189.0 / 360.0, 0.71, 0.73, 1.0),
            text_placeholder: hsla(228.0 / 360.0, 0.24, 0.40, 1.0),
            color_warn: hsla(41.0 / 360.0, 0.86, 0.83, 1.0),
            color_err: hsla(343.0 / 360.0, 0.81, 0.75, 1.0),
            color_succ: hsla(115.0 / 360.0, 0.54, 0.76, 1.0),
            banner_bg: hsla(41.0 / 360.0, 0.86, 0.12, 1.0),
            banner_border: hsla(41.0 / 360.0, 0.86, 0.35, 1.0),
            banner_text: hsla(41.0 / 360.0, 0.86, 0.75, 1.0),
        }
    }

    pub fn nord() -> Self {
        Self {
            font_family: "Inter".into(),
            monospace: "Noto Sans Mono".into(),
            cursor: hsla(213.0 / 360.0, 0.32, 0.52, 1.0),
            selection: hsla(213.0 / 360.0, 0.32, 0.52, 0.25),
            bg_app: hsla(220.0 / 360.0, 0.17, 0.14, 1.0),
            bg_selected: hsla(220.0 / 360.0, 0.17, 0.28, 1.0),
            bg_idle: hsla(220.0 / 360.0, 0.17, 0.18, 1.0),
            bg_status_bar: hsla(220.0 / 360.0, 0.17, 0.12, 1.0),
            bg_keybind: hsla(220.0 / 360.0, 0.17, 0.22, 1.0),
            bg_code: hsla(220.0 / 360.0, 0.17, 0.20, 1.0),
            bg_muted: hsla(220.0 / 360.0, 0.17, 0.18, 1.0),
            bg_badge: hsla(220.0 / 360.0, 0.17, 0.32, 0.6),
            border: hsla(220.0 / 360.0, 0.17, 0.32, 1.0),
            border_selected: hsla(213.0 / 360.0, 0.32, 0.52, 0.4),
            border_idle: hsla(220.0 / 360.0, 0.17, 0.28, 1.0),
            primary_text: hsla(218.0 / 360.0, 0.27, 0.92, 1.0),
            secondary_text: hsla(219.0 / 360.0, 0.14, 0.65, 1.0),
            tertiary_text: hsla(219.0 / 360.0, 0.10, 0.45, 1.0),
            text_status_bar: hsla(219.0 / 360.0, 0.14, 0.40, 1.0),
            text_mode_label: hsla(219.0 / 360.0, 0.14, 0.30, 1.0),
            text_search_icon: hsla(219.0 / 360.0, 0.14, 0.50, 1.0),
            text_code: hsla(210.0 / 360.0, 0.34, 0.63, 1.0), // Nord8 Frost
            text_placeholder: hsla(219.0 / 360.0, 0.14, 0.35, 1.0),
            color_warn: hsla(40.0 / 360.0, 0.70, 0.73, 1.0),
            color_err: hsla(354.0 / 360.0, 0.42, 0.65, 1.0),
            color_succ: hsla(92.0 / 360.0, 0.28, 0.65, 1.0),
            banner_bg: hsla(40.0 / 360.0, 0.70, 0.12, 1.0),
            banner_border: hsla(40.0 / 360.0, 0.70, 0.35, 1.0),
            banner_text: hsla(40.0 / 360.0, 0.70, 0.73, 1.0),
        }
    }
}
