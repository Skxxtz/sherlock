use gio::glib::Bytes;
use gtk4::{gdk, prelude::*, Image};
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::calc_launcher::CalculatorLauncher;
use crate::launcher::clipboard_launcher::ClipboardLauncher;
use crate::launcher::Launcher;

use super::util::TileBuilder;
use super::Tile;

struct RGB {
    r: u8,
    g: u8,
    b: u8,
}
impl RGB {
    fn from_hex(hex: &str) -> Self {
        let default = Self { r: 0, g: 0, b: 0 };
        if hex.len() >= 6 {
            let Ok(r) = u8::from_str_radix(&hex[0..2], 16) else {
                return default;
            };
            let Ok(g) = u8::from_str_radix(&hex[2..4], 16) else {
                return default;
            };
            let Ok(b) = u8::from_str_radix(&hex[4..6], 16) else {
                return default;
            };
            return Self { r, g, b };
        }
        default
    }
    fn from_hsl(hsl: Vec<u32>) -> Self {
        if hsl.len() != 3 {
            return Self { r: 0, g: 0, b: 0 };
        }
        let (h, s, l) = (hsl[0], hsl[1], hsl[2]);

        let h = (h as f64) / 360.0;
        let s = (s as f64) / 100.0;
        let l = (l as f64) / 100.0;

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h * 6.0).fract() - 1.0).abs());
        let m = l - c / 2.0;

        let (r_prime, g_prime, b_prime) = if h >= 0.0 && h < 1.0 / 6.0 {
            (c, x, 0.0)
        } else if h >= 1.0 / 6.0 && h < 2.0 / 6.0 {
            (x, c, 0.0)
        } else if h >= 2.0 / 6.0 && h < 3.0 / 6.0 {
            (0.0, c, x)
        } else if h >= 3.0 / 6.0 && h < 4.0 / 6.0 {
            (0.0, x, c)
        } else if h >= 4.0 / 6.0 && h < 5.0 / 6.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        let r = ((r_prime + m) * 255.0).round() as u8;
        let g = ((g_prime + m) * 255.0).round() as u8;
        let b = ((b_prime + m) * 255.0).round() as u8;
        Self { r, g, b }
    }
    fn from_str(rgb: &str) -> Self {
        let rgb: Vec<u8> = rgb
            .split(",")
            .map(|s| s.trim())
            .filter_map(|s| s.parse::<u8>().ok())
            .collect();
        if rgb.len() != 3 {
            return Self { r: 0, g: 0, b: 0 };
        }
        Self {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        }
    }
    fn to_vec(&self) -> Vec<u8> {
        vec![self.r, self.g, self.b]
    }
}

impl Tile {
    pub fn clipboard_tile(
        launcher: &Launcher,
        clp: &ClipboardLauncher,
        calc: &CalculatorLauncher,
    ) -> Vec<SherlockRow> {
        let mut results: Vec<SherlockRow> = Vec::with_capacity(1);
        let mut is_valid = false;
        let clipboard_content = &clp.clipboard_content;
        let capabilities: HashSet<&str> = match &clp.capabilities {
            Some(c) => c.iter().map(|s| s.as_str()).collect(),
            _ => HashSet::from(["url", "calc.math", "calc.units", "colors.all"]),
        };

        //TODO implement searchstring before clipboard content
        if !clipboard_content.is_empty() {
            let mut builder = TileBuilder::default();
            let name = "From Clipboard";
            let mut method = "";
            let mut icon = "";

            let known_pages = HashMap::from([
                ("google", "google"),
                ("chatgpt", "chat-gpt"),
                ("youtube", "sherlock-youtube"),
            ]);

            // Check if clipboard content is a url:
            let url_raw = r"^(https?:\/\/)?(www\.)?([\da-z\.-]+)\.([a-z]{2,6})([\/\w\.-]*)*\/?$";
            let color_raw = r"^(rgb|hsl)*\(?(\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3})\)?|\(?(\s*\d{1,3}\s*,\s*\d{1,3}%\s*,\s*\d{1,3}\s*%\w*)\)?|^#([a-fA-F0-9]{6,8})$";
            let url_re = Regex::new(url_raw).unwrap();
            let color_re = Regex::new(color_raw).unwrap();
            if capabilities.contains("url") {
                if let Some(captures) = url_re.captures(clipboard_content) {
                    if let Some(main_domain) = captures.get(3) {
                        // setting up builder
                        builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/tile.ui");
                        builder.object.set_spawn_focus(launcher.spawn_focus);
                        builder.object.set_shortcut(launcher.shortcut);

                        is_valid = true;
                        method = "web_launcher";
                        let main_domain = main_domain.as_str();
                        icon = known_pages.get(main_domain).map_or("sherlock-link", |m| m);
                    }
                };
            };
            if !is_valid {
                if let Some(captures) = color_re.captures(clipboard_content) {
                    // Groups: 2: RGB, 3: HSL, 4: HEX
                    let rgb = if let Some(rgb) = captures.get(2) {
                        if capabilities.contains("colors.rgb")
                            || capabilities.contains("colors.all")
                        {
                            Some(RGB::from_str(rgb.as_str()))
                        } else {
                            None
                        }
                    } else if let Some(hsl) = captures.get(3) {
                        if capabilities.contains("colors.hsl")
                            || capabilities.contains("colors.all")
                        {
                            let mut res: Vec<u32> = Vec::with_capacity(3);
                            let mut tmp = 0;
                            let mut was_changed: u8 = 0;
                            hsl.as_str()
                                .chars()
                                .filter(|s| !s.is_whitespace())
                                .for_each(|s| {
                                    if let Some(digit) = s.to_digit(10) {
                                        tmp = tmp * 10 + digit;
                                        was_changed = 1;
                                    } else if was_changed > 0 {
                                        res.push(tmp);
                                        was_changed = 0;
                                        tmp = 0;
                                    }
                                });
                            Some(RGB::from_hsl(res))
                        } else {
                            None
                        }
                    } else if let Some(hex) = captures.get(4) {
                        if capabilities.contains("colors.hex")
                            || capabilities.contains("colors.all")
                        {
                            Some(RGB::from_hex(hex.as_str()))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(rgb) = rgb {
                        builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/tile.ui");
                        builder.object.with_launcher(launcher);
                        builder.object.set_search(clipboard_content);

                        let pix_buf = rgb.to_vec();
                        let image_buf = gdk::gdk_pixbuf::Pixbuf::from_bytes(
                            &Bytes::from_owned(pix_buf),
                            gdk::gdk_pixbuf::Colorspace::Rgb,
                            false,
                            8,
                            1,
                            1,
                            3,
                        );
                        if let Some(image_buf) =
                            image_buf.scale_simple(30, 30, gdk::gdk_pixbuf::InterpType::Nearest)
                        {
                            let texture = gtk4::gdk::Texture::for_pixbuf(&image_buf);
                            let image = Image::from_paintable(Some(&texture));
                            image.set_widget_name("icon");
                            image.set_pixel_size(22);

                            builder
                                .icon_holder
                                .as_ref()
                                .and_then(|tmp| tmp.upgrade())
                                .map(|holder| {
                                    holder.append(&image);
                                    holder.set_overflow(gtk4::Overflow::Hidden);
                                    holder.set_widget_name("color-icon-holder");
                                });
                            builder
                                .icon
                                .as_ref()
                                .and_then(|tmp| tmp.upgrade())
                                .map(|icon| icon.set_visible(false));
                            is_valid = true;
                        };
                    }
                }
            };
            if !is_valid {
                // calc capabilities will be checked inside of calc tile
                let mut calc_tile = Tile::calc_tile(launcher, calc);
                if calc_tile.len() >= 1 {
                    let tile = calc_tile.remove(0);
                    if tile.update(clipboard_content) {
                        tile.set_only_home(true);
                        results.push(tile)
                    }
                }
            } else {
                builder
                    .category
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|category| {
                        category.set_visible(!name.is_empty());
                        category.set_text(name);
                    });

                builder
                    .icon
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|ico| {
                        if icon.starts_with("/") {
                            ico.set_from_file(Some(&icon));
                        } else {
                            ico.set_icon_name(Some(&icon));
                        }
                        ico.set_pixel_size(15);
                    });
                builder
                    .title
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|title| title.set_text(clipboard_content));

                // Add action capabilities
                let attrs = get_attrs_map(vec![
                    ("method", method),
                    ("keyword", clipboard_content),
                    ("engine", "plain"),
                ]);

                builder.object.with_launcher(&launcher);
                builder
                    .object
                    .connect_local("row-should-activate", false, move |row| {
                        let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                        execute_from_attrs(&row, &attrs);
                        None
                    });
                let update_closure = |_: &str| -> bool { true };
                builder.object.set_update(update_closure);

                if launcher.shortcut {
                    builder.object.set_shortcut_holder(builder.shortcut_holder);
                }
                results.push(builder.object);
            }
        }

        return results;
    }
}
