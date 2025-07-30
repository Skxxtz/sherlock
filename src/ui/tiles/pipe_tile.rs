use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::rc::Rc;

use crate::actions::execute_from_attrs;
use crate::actions::get_attrs_map;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::g_subclasses::tile_item::TileItem;
use crate::g_subclasses::tile_item::UpdateHandler;
use crate::launcher::pipe_launcher::PipeLauncher;
use crate::launcher::Launcher;
use crate::launcher::LauncherType;
use crate::loader::pipe_loader::PipedElements;
use crate::prelude::IconComp;
use crate::prelude::TileHandler;
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gdk_pixbuf::Pixbuf;
use gio::glib::object::Cast;
use gio::glib::object::ObjectExt;
use gio::glib::property::PropertySet;
use gio::glib::variant::ToVariant;
use gio::glib::WeakRef;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::Box;
use gtk4::Image;
use gtk4::Widget;

use super::app_tile::AppTile;
use super::Tile;

impl Tile {
    pub fn pipe_items(elements: Vec<PipedElements>, method: &str) -> Vec<TileItem> {
        elements
            .into_iter()
            .map(|piped| {
                let launcher = Rc::new(Launcher::from_piped_element(piped, method.to_string()));
                let tile = TileItem::new();
                let handler = PipeTileHandler::new(launcher.clone());
                tile.imp().update_handler.set(UpdateHandler::Pipe(handler));
                tile.set_launcher(launcher);
                tile
            })
            .collect()
    }
    pub fn pipe(launcher: Rc<Launcher>, pipe: &PipeLauncher) -> Option<AppTile> {
        let search = format!(
            "{};{}",
            launcher.name.as_deref().unwrap_or(""),
            pipe.description.as_deref().unwrap_or("")
        );
        let tile = AppTile::new();
        let imp = tile.imp();

        if search.as_str() == ";" && pipe.binary.is_none() {
            return None;
        }
        // Set texts
        if let Some(title) = &launcher.name {
            imp.title.set_text(title.trim());
        }
        if let Some(name) = &pipe.description {
            imp.category.set_text(&name);
        } else {
            imp.category.set_visible(false);
        }

        imp.icon.set_icon(launcher.icon.as_deref(), None, None);
        // Custom Image Data
        if let Some(bin) = pipe.binary.clone() {
            let cursor = Cursor::new(bin);
            if let Some(pixbuf) = Pixbuf::from_read(cursor).ok() {
                let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                let image = Image::from_paintable(Some(&texture));
                imp.icon_holder.append(&image);
                if let Some(size) = &pipe.icon_size {
                    image.set_pixel_size(*size);
                }
            }
        } else {
            let opacity: f64 = if launcher.icon.is_some() { 1.0 } else { 0.0 };
            imp.icon.set_opacity(opacity);
        }
        Some(tile)
    }
}

#[derive(Default, Debug)]
pub struct PipeTileHandler {
    tile: WeakRef<AppTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
}
impl PipeTileHandler {
    pub fn new(launcher: Rc<Launcher>) -> Self {
        let LauncherType::Pipe(pipe) = &launcher.launcher_type else {
            return Self::default()
        };
        let method = launcher.method.as_ref();
        let result = pipe.result.as_deref().or(launcher.name.as_deref());
        let exit = launcher.exit.to_string();
        let mut constructor: Vec<(&str, Option<&str>)> =
            pipe.hidden.as_ref().map_or_else(Vec::new, |a| {
                a.iter()
                    .map(|(k, v)| (k.as_str(), Some(v.as_str())))
                    .collect()
            });
        constructor.extend(vec![
            ("method", Some(method)),
            ("result", result),
            ("field", pipe.field.as_deref()),
            ("exit", Some(&exit)),
        ]);
        let attrs = get_attrs_map(constructor);
        Self {
            tile: WeakRef::new(),
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
impl TileHandler for PipeTileHandler {
    fn replace_tile(&mut self, tile: &Widget) {
        if let Some(tile) = tile.downcast_ref::<AppTile>() {
            self.tile = tile.downgrade()
        }
    }
}
