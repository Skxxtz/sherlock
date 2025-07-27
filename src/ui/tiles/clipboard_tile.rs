use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{Bytes, WeakRef};
use gtk4::{gdk, prelude::*, Box, Image, Widget};
use regex::Regex;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::g_subclasses::tile_item::UpdateHandler;
use crate::launcher::clipboard_launcher::ClipboardLauncher;
use crate::launcher::Launcher;
use crate::prelude::IconComp;
use crate::ui::tiles::calc_tile::{CalcTile, CalcTileHandler};

use super::app_tile::AppTile;
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
    pub fn clipboard(
        launcher: Rc<Launcher>,
        clp: &ClipboardLauncher,
    ) -> Option<(Widget, UpdateHandler)> {
        let clipboard_content = clp.clipboard_content.clone();
        if clipboard_content.is_empty() {
            return None;
        }
        let capabilities = clp.capabilities.clone().unwrap_or(
            vec!["url", "calc.math", "calc.units", "colors.all"]
                .into_iter()
                .map(String::from)
                .collect::<HashSet<_>>(),
        );

        // Url Capabilities
        if capabilities.contains("url") {
            let url_raw = r"^(https?:\/\/)?(www\.)?([\da-z\.-]+)\.([a-z]{2,6})([\/\w\.-]*)*\/?$";
            let url_re = Regex::new(url_raw).unwrap();

            let known_pages = HashMap::from([
                ("google", "google"),
                ("chatgpt", "chat-gpt"),
                ("youtube", "sherlock-youtube"),
            ]);

            if let Some(captures) = url_re.captures(&clipboard_content) {
                if let Some(main_domain) = captures.get(3) {
                    // setting up builder
                    let tile = AppTile::new();
                    let attrs = get_attrs_map(vec![
                        ("method", Some("web_launcher")),
                        ("keyword", Some(&clipboard_content)),
                        ("engine", Some("plain")),
                    ]);
                    // TODO: Add Handler
                    let main_domain = main_domain.as_str();
                    let icon = known_pages.get(main_domain).map_or("sherlock-link", |m| m);

                    let imp = tile.imp();
                    imp.icon.set_icon(Some(icon), None, Some("sherlock-link"));
                    imp.title.set_text(clipboard_content.trim());
                    imp.category.set_text("From Clipboard");

                    let handler = ClipboardHandler::new(&tile, attrs);

                    return Some((tile.upcast::<Widget>(), UpdateHandler::Clipboard(handler)));
                }
            };
        }

        // Color Capabilities
        if capabilities
            .iter()
            .find(|c| c.starts_with("colors."))
            .is_some()
            && clipboard_content.len() <= 20
        {
            let color_raw = r"^(rgb|hsl)*\(?(\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3})\)?|\(?(\s*\d{1,3}\s*,\s*\d{1,3}%\s*,\s*\d{1,3}\s*%\w*)\)?|^#([a-fA-F0-9]{6,8})$";
            let color_re = Regex::new(color_raw).unwrap();
            let all = capabilities.contains("colors.all");
            let attrs = get_attrs_map(vec![
                ("method", None),
                ("keyword", Some(&clipboard_content)),
            ]);

            if let Some(captures) = color_re.captures(&clipboard_content) {
                if all || capabilities.contains("colors.rgb") {
                    if let Some(rgb) = captures.get(2) {
                        let color = RGB::from_str(rgb.as_str());
                        let label = format!("rbg({})", rgb.as_str().trim());
                        let tile = color_tile(color, label);
                        let handler = ClipboardHandler::new(&tile, attrs);
                        return Some((tile.upcast::<Widget>(), UpdateHandler::Clipboard(handler)));
                    }
                }

                if all || capabilities.contains("colors.hsl") {
                    if let Some(hsl) = captures.get(3) {
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
                        let color = RGB::from_hsl(res);
                        let label = format!("hls({})", hsl.as_str().trim());
                        let tile = color_tile(color, label);
                        let handler = ClipboardHandler::new(&tile, attrs);
                        return Some((tile.upcast::<Widget>(), UpdateHandler::Clipboard(handler)));
                    }
                }

                if all || capabilities.contains("colors.hex") {
                    if let Some(hex) = captures.get(4) {
                        let color = RGB::from_hex(hex.as_str());
                        let label = format!("#{}", hex.as_str().trim());
                        let tile = color_tile(color, label);
                        let handler = ClipboardHandler::new(&tile, attrs);
                        return Some((tile.upcast::<Widget>(), UpdateHandler::Clipboard(handler)));
                    }
                }
            }
        }

        // Calculator Capabilities
        if capabilities
            .iter()
            .find(|c| c.starts_with("calc."))
            .is_some()
        {
            let tile = CalcTile::new();
            let handler = CalcTileHandler::new(&tile, launcher);
            if handler.based_show(&clipboard_content, &capabilities) {
                handler.update(&clipboard_content);
                return Some((tile.upcast::<Widget>(), UpdateHandler::Calculator(handler)));
            }
        }

        None
    }
}

fn color_tile(rgb: RGB, label: String) -> AppTile {
    let tile = AppTile::new();
    let imp = tile.imp();

    imp.title.set_text(&label);
    imp.category.set_text("From Clipboard");
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
    if let Some(image_buf) = image_buf.scale_simple(30, 30, gdk::gdk_pixbuf::InterpType::Nearest) {
        let texture = gtk4::gdk::Texture::for_pixbuf(&image_buf);
        let image = Image::from_paintable(Some(&texture));
        image.set_widget_name("icon");
        image.set_pixel_size(22);

        let holder = &imp.icon_holder;
        holder.append(&image);
        holder.set_overflow(gtk4::Overflow::Hidden);
        holder.set_widget_name("color-icon-holder");

        imp.icon.set_visible(false);
    };

    tile
}

#[derive(Debug)]
pub struct ClipboardHandler {
    tile: WeakRef<AppTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
}

impl ClipboardHandler {
    fn new(tile: &AppTile, attrs: HashMap<String, String>) -> Self {
        Self {
            tile: tile.downgrade(),
            attrs: Rc::new(RefCell::new(attrs)),
        }
    }
    pub fn bind_signal(&self, row: &SherlockRow) {
        let signal_id = row.connect_local("row-should-activate", false, {
            let attrs = self.attrs.clone();
            move |args| {
                let row = args.first().map(|f| f.get::<SherlockRow>().ok())??;
                let param: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
                let param: Option<bool> = match param {
                    1 => Some(false),
                    2 => Some(true),
                    _ => None,
                };
                execute_from_attrs(&row, &attrs.borrow(), param);
                // To reload ui according to mode
                let _ = row.activate_action("win.update-items", Some(&false.to_variant()));
                None
            }
        });
        row.set_signal_id(signal_id);
    }
    pub fn shortcut(&self) -> Option<Box> {
        self.tile.upgrade().map(|t| t.imp().shortcut_holder.get())
    }
}
