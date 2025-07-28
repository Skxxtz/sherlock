use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::object::{Cast, ObjectExt};
use gio::glib::WeakRef;
use gtk4::prelude::WidgetExt;
use gtk4::Box;

use super::util::update_tag;
use super::Tile;
use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::web_launcher::WebLauncher;
use crate::launcher::Launcher;
use crate::prelude::{IconComp, TileHandler};
use crate::ui::tiles::app_tile::AppTile;

impl Tile {
    pub fn web(launcher: Rc<Launcher>, web: &WebLauncher) -> AppTile {
        let tile = AppTile::new();
        let imp = tile.imp();

        if let Some(name) = &launcher.name {
            imp.category.set_text(&name);
        } else {
            imp.category.set_visible(false);
        }

        imp.icon.set_icon(Some(&web.icon), None, None);

        tile
    }
}

#[derive(Debug)]
pub struct WebTileHandler {
    attrs: Rc<RefCell<HashMap<String, String>>>,
    tile: WeakRef<AppTile>,
}
impl WebTileHandler {
    pub fn new(tile: &AppTile) -> Self {
        Self {
            attrs: Rc::new(RefCell::new(HashMap::new())),
            tile: tile.downgrade(),
        }
    }
    pub fn update(&self, keyword: &str, launcher: Rc<Launcher>, web: &WebLauncher) -> Option<()> {
        let tile_name = web.display_name.clone();
        if self.attrs.borrow().is_empty() {
            let mut attrs = get_attrs_map(vec![
                ("method", Some(&launcher.method)),
                ("engine", Some(&web.engine)),
                ("exit", Some(&launcher.exit.to_string())),
            ]);
            if let Some(next) = launcher.next_content.as_deref() {
                attrs.insert(String::from("next_content"), next.to_string());
            }
            *self.attrs.borrow_mut() = attrs;
        }
        {
            let mut attrs = self.attrs.borrow_mut();
            attrs.insert("keyword".to_string(), keyword.to_string());
            attrs.insert("result".to_string(), keyword.to_string());
        }
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();
        // Update title
        imp.title.set_text(&tile_name.replace("{keyword}", keyword));

        // update first tag
        update_tag(&imp.tag_start, &launcher.tag_start, keyword);

        // update second tag
        update_tag(&imp.tag_end, &launcher.tag_end, keyword);

        Some(())
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
                None
            }
        });
        row.set_signal_id(signal_id);
    }
    pub fn shortcut(&self) -> Option<Box> {
        self.tile.upgrade().map(|t| t.imp().shortcut_holder.get())
    }
}
impl Default for WebTileHandler {
    fn default() -> Self {
        Self {
            attrs: Rc::new(RefCell::new(HashMap::new())),
            tile: WeakRef::new(),
        }
    }
}
impl TileHandler for WebTileHandler {
    fn replace_tile(&mut self, tile: &gtk4::Widget) {
        if let Some(tile) = tile.downcast_ref::<AppTile>(){
            self.tile = tile.downgrade();
        }
    }
}
