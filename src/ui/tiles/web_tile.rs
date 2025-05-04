use std::cell::RefCell;
use std::rc::Rc;

use gio::glib::object::ObjectExt;
use gtk4::prelude::WidgetExt;

use super::util::{update_tag, TileBuilder};
use super::Tile;
use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::web_launcher::WebLauncher;
use crate::launcher::Launcher;

impl Tile {
    pub fn web_tile(launcher: &Launcher, web: &WebLauncher) -> Vec<SherlockRow> {
        let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/tile.ui");
        builder
            .category
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|category| {
                if let Some(name) = &launcher.name {
                    category.set_text(name);
                } else {
                    category.set_visible(false);
                }
            });

        builder
            .icon
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|icon| {
                if web.icon.starts_with("/") {
                    icon.set_from_file(Some(&web.icon));
                } else {
                    icon.set_icon_name(Some(&web.icon));
                }
            });

        // Construct attrs and enable action capabilities
        builder.object.with_launcher(&launcher);
        builder.object.set_keyword_aware(true);

        let update_closure = {
            let tag_start = builder.tag_start.clone();
            let tag_end = builder.tag_end.clone();
            let tag_start_content = launcher.tag_start.clone();
            let tag_end_content = launcher.tag_end.clone();
            let title = builder.title.clone();
            let row_weak = builder.object.downgrade();
            let tile_name = web.display_name.clone();
            let mut attrs =
                get_attrs_map(vec![("method", &launcher.method), ("engine", &web.engine)]);
            if let Some(next) = launcher.next_content.as_deref() {
                attrs.insert(String::from("next_content"), next.to_string());
            }
            let attrs_rc = Rc::new(RefCell::new(attrs));
            move |keyword: &str| -> bool {
                let attrs_clone = Rc::clone(&attrs_rc);

                // Update title
                if let Some(title) = title.as_ref().and_then(|tmp| tmp.upgrade()) {
                    title.set_text(&tile_name.replace("{keyword}", keyword));
                }

                // update first tag
                if let Some(tag_start) = &tag_start {
                    update_tag(&tag_start, &tag_start_content, keyword);
                }

                // update second tag
                if let Some(tag_end) = &tag_end {
                    update_tag(&tag_end, &tag_end_content, keyword);
                }

                // update attributes to activate correct action
                let keyword_clone = keyword.to_string();
                row_weak.upgrade().map(|row| {
                    let signal_id = row.connect_local("row-should-activate", false, move |row| {
                        let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                        {
                            let mut attrs = attrs_clone.borrow_mut();
                            attrs.insert("keyword".to_string(), keyword_clone.clone());
                            attrs.insert("result".to_string(), keyword_clone.clone());
                        }
                        execute_from_attrs(&row, &attrs_clone.borrow());
                        None
                    });
                    row.set_signal_id(signal_id);
                });

                // Set to false to not always show this tile
                false
            }
        };
        builder.object.set_update(update_closure);

        if launcher.shortcut {
            builder.object.set_shortcut_holder(builder.shortcut_holder);
        }
        return vec![builder.object];
    }
}
