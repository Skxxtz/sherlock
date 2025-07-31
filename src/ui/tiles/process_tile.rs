use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::WeakRef;
use gtk4::{prelude::*, Box};

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::g_subclasses::tile_item::TileItem;
use crate::launcher::Launcher;
use crate::loader::util::AppData;
use crate::prelude::{IconComp, TileHandler};
use crate::ui::tiles::util::update_tag;

use super::app_tile::AppTile;
use super::Tile;

impl Tile {
    pub fn process(value: &AppData, launcher: Rc<Launcher>, item: &TileItem) -> AppTile {
        let tile = AppTile::new();
        let imp = tile.imp();

        // Icon stuff
        imp.icon.set_icon(
            launcher.icon.as_deref(),
            value.icon_class.as_deref(),
            Some("sherlock-process"),
        );

        item.add_actions(&launcher.add_actions);
        tile
    }
}

#[derive(Default, Debug)]
pub struct ProcTileHandler {
    tile: WeakRef<AppTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
}
impl ProcTileHandler {
    pub fn new(tile: &AppTile) -> Self {
        Self {
            tile: tile.downgrade(),
            attrs: Rc::new(RefCell::new(HashMap::new())),
        }
    }
    pub fn update(&self, keyword: &str, launcher: Rc<Launcher>, value: &AppData) -> Option<()> {
        // Construct attrs and enable action capabilities
        let tag_start_content = launcher.tag_start.clone();
        let tag_end_content = launcher.tag_end.clone();

        let launcher = Rc::clone(&launcher);
        if self.attrs.borrow().is_empty() {
            let exec = value.exec.clone().unwrap_or_default();
            let mut parts = exec.splitn(2, ",");
            let ppid = parts.next().unwrap_or("");
            let cpid = parts.next().unwrap_or("");

            let attrs = get_attrs_map(vec![
                ("method", Some("kill-process")),
                ("result", Some(&value.name)),
                ("parent-pid", Some(ppid)),
                ("child-pid", Some(cpid)),
                ("exit", Some(&launcher.exit.to_string())),
            ]);
            *self.attrs.borrow_mut() = attrs;
        }
        let name = value.name.clone();
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();

        {
            let mut attrs_ref = self.attrs.borrow_mut();
            attrs_ref.insert(String::from("keyword"), keyword.to_string());
        }
        let tile_name = name.replace("{keyword}", keyword);

        // update first tag
        update_tag(&imp.tag_start, &tag_start_content, keyword);

        // update second tag
        update_tag(&imp.tag_end, &tag_end_content, keyword);

        imp.title.set_text(&tile_name);

        if let Some(name) = &launcher.name {
            imp.category.set_text(name);
        } else {
            imp.category.set_visible(false);
        }

        Some(())
    }
    pub fn bind_signal(&self, row: &SherlockRow, launcher: Rc<Launcher>) {
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
                execute_from_attrs(&row, &attrs.borrow(), param, Some(launcher.clone()));
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
impl TileHandler for ProcTileHandler {
    fn replace_tile(&mut self, tile: &gtk4::Widget) {
        if let Some(tile) = tile.downcast_ref::<AppTile>() {
            self.tile = tile.downgrade();
        }
    }
}
