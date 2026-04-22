use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::loader::icon::ICON_SIZE;
use crate::utils::paths::get_config_dir;

static ICON_THEME: OnceLock<Option<String>> = OnceLock::new();

/// Get the configured icon theme from KDE/GTK settings
pub fn get_current_theme() -> String {
    ICON_THEME
        .get_or_init(|| {
            read_kde_icon_theme()
                .or(read_gtk3_icon_theme())
                .or(read_gtk4_icon_theme())
                .or(read_gsettings_icon_theme())
        })
        .clone()
        .unwrap_or(String::from("hicolor"))
}

fn parse_ini_value(content: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == section;
            continue;
        }
        if in_section
            && let Some(value) = line.strip_prefix(key)
            && let Some(value) = value.strip_prefix('=')
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn read_kde_icon_theme() -> Option<String> {
    let config_path = get_config_dir().ok()?.join("kdeglobals");
    let content = fs::read_to_string(config_path).ok()?;
    parse_ini_value(&content, "[Icons]", "Theme")
}

fn read_gtk3_icon_theme() -> Option<String> {
    let config_path = get_config_dir().ok()?.join("gtk-3.0/settings.ini");
    let content = fs::read_to_string(config_path).ok()?;
    parse_ini_value(&content, "[Settings]", "gtk-icon-theme-name")
}

fn read_gtk4_icon_theme() -> Option<String> {
    let config_path = get_config_dir().ok()?.join("gtk-4.0/settings.ini");
    let content = fs::read_to_string(config_path).ok()?;
    parse_ini_value(&content, "[Settings]", "gtk-icon-theme-name")
}

fn read_gsettings_icon_theme() -> Option<String> {
    let output = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    // gsettings wraps strings in single quotes: 'Adwaita'
    Some(raw.trim().trim_matches('\'').to_string())
}

pub fn resolve_icon_internal(icon_name: &str) -> Option<PathBuf> {
    // Absolute path - use directly
    if icon_name.starts_with('/') {
        let path = PathBuf::from(icon_name);
        if path.exists() {
            return Some(path);
        }
        return None;
    }

    // Try configured theme first
    let icon = freedesktop_icons::lookup(icon_name)
        .with_size(ICON_SIZE)
        .with_theme(&get_current_theme())
        .find();

    if icon.is_some() {
        return icon;
    }

    // Last resort: no theme specified
    freedesktop_icons::lookup(icon_name)
        .with_size(ICON_SIZE)
        .find()
}
