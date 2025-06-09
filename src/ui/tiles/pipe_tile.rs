use std::io::Cursor;

use crate::actions::execute_from_attrs;
use crate::actions::get_attrs_map;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::loader::pipe_loader::PipedElements;
use crate::prelude::IconComp;
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gdk_pixbuf::Pixbuf;
use gio::glib::object::ObjectExt;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::Image;

use super::app_tile::AppTile;
use super::Tile;

impl Tile {
    pub fn pipe_data(lines: &Vec<PipedElements>, method: &str) -> Vec<SherlockRow> {
        let mut results: Vec<SherlockRow> = Vec::with_capacity(lines.len());

        for item in lines {
            let search = format!(
                "{};{}",
                item.title.as_deref().unwrap_or(""),
                item.description.as_deref().unwrap_or("")
            );
            if search.as_str() != ";" || item.binary.is_some() {
                let tile = AppTile::new();
                let imp = tile.imp();
                let object = SherlockRow::new();
                object.append(&tile);

                // Set texts
                if let Some(title) = &item.title {
                    imp.title.set_text(&title);
                }
                if let Some(name) = &item.description {
                    imp.category.set_text(&name);
                } else {
                    imp.category.set_visible(false);
                }

                // Set icon
                imp.icon.set_icon(item.icon.as_deref(), None, None);

                // Custom Image Data
                if let Some(bin) = item.binary.clone() {
                    let cursor = Cursor::new(bin);
                    if let Some(pixbuf) = Pixbuf::from_read(cursor).ok() {
                        let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                        let image = Image::from_paintable(Some(&texture));
                        imp.icon_holder.append(&image);
                        if let Some(size) = &item.icon_size {
                            image.set_pixel_size(*size);
                        }
                    }
                } else {
                    let opacity: f64 = if item.icon.is_some() { 1.0 } else { 0.0 };
                    imp.icon.set_opacity(opacity);
                }

                // Create attributes and enable action capability
                let method = item.method.as_deref().unwrap_or(method);
                let result = item.result.as_deref().or(item.title.as_deref());
                let exit = item.exit.to_string();
                let mut constructor: Vec<(&str, Option<&str>)> =
                    item.hidden.as_ref().map_or_else(Vec::new, |a| {
                        a.iter()
                            .map(|(k, v)| (k.as_str(), Some(v.as_str())))
                            .collect()
                    });
                constructor.extend(vec![
                    ("method", Some(method)),
                    ("result", result),
                    ("field", item.field.as_deref()),
                    ("exit", Some(&exit)),
                ]);
                let attrs = get_attrs_map(constructor);

                object.set_spawn_focus(true);
                object.set_home(true);
                object.set_priority(1.0);
                object.set_search(&search);
                object.connect_local("row-should-activate", false, move |args| {
                    let row = args.first().map(|f| f.get::<SherlockRow>().ok())??;
                    let param: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
                    let param: Option<bool> = match param {
                        1 => Some(false),
                        2 => Some(true),
                        _ => None,
                    };
                    execute_from_attrs(&row, &attrs, param);
                    None
                });
                results.push(object);
            }
        }
        return results;
    }
}
