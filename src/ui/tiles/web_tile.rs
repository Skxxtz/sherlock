use std::cell::RefCell;
use std::rc::Rc;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::object::ObjectExt;
use gtk4::prelude::{BoxExt, WidgetExt};

use super::app_tile::AppTile;
use super::util::update_tag;
use super::Tile;
use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::web_launcher::WebLauncher;
use crate::launcher::Launcher;
use crate::prelude::IconComp;

impl Tile {
    pub fn web(launcher: Rc<Launcher>, web: &WebLauncher) -> Option<SherlockRow> {
        let tile = AppTile::new();
        let imp = tile.imp();
        let object = SherlockRow::new();
        object.append(&tile);

        if let Some(name) = &launcher.name {
            imp.category.set_text(&name);
        } else {
            imp.category.set_visible(false);
        }

        imp.icon.set_icon(Some(&web.icon), None, None);

        // Construct attrs and enable action capabilities
        object.with_launcher(launcher.clone());
        object.set_keyword_aware(true);

        let update_closure = {
            let tile_ref = tile.downgrade();
            let tag_start_content = launcher.tag_start.clone();
            let tag_end_content = launcher.tag_end.clone();
            let row_weak = object.downgrade();
            let tile_name = web.display_name.clone();
            let mut attrs = get_attrs_map(vec![
                ("method", Some(&launcher.method)),
                ("engine", Some(&web.engine)),
                ("exit", Some(&launcher.exit.to_string())),
            ]);
            if let Some(next) = launcher.next_content.as_deref() {
                attrs.insert(String::from("next_content"), next.to_string());
            }
            let attrs_rc = Rc::new(RefCell::new(attrs));
            move |keyword: &str| -> bool {
                let Some(tile) = tile_ref.upgrade() else {
                    return false;
                };
                let imp = tile.imp();
                let attrs_clone = Rc::clone(&attrs_rc);

                // Update title
                imp.title.set_text(&tile_name.replace("{keyword}", keyword));

                // update first tag
                update_tag(&imp.tag_start, &tag_start_content, keyword);

                // update second tag
                update_tag(&imp.tag_end, &tag_end_content, keyword);

                // update attributes to activate correct action
                let keyword_clone = keyword.to_string();
                row_weak.upgrade().map(|row| {
                    let signal_id = row.connect_local("row-should-activate", false, move |args| {
                        let row = args.first().map(|f| f.get::<SherlockRow>().ok())??;
                        let param: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
                        let param: Option<bool> = match param {
                            1 => Some(false),
                            2 => Some(true),
                            _ => None,
                        };
                        {
                            let mut attrs = attrs_clone.borrow_mut();
                            attrs.insert("keyword".to_string(), keyword_clone.clone());
                            attrs.insert("result".to_string(), keyword_clone.clone());
                        }
                        execute_from_attrs(&row, &attrs_clone.borrow(), param);
                        None
                    });
                    row.set_signal_id(signal_id);
                });

                // Set to false to not always show this tile
                false
            }
        };
        object.set_update(update_closure);

        if launcher.shortcut {
            object.set_shortcut_holder(Some(imp.shortcut_holder.downgrade()));
        }

        Some(object)
    }
}
