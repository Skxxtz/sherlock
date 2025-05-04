use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::Launcher;
use crate::loader::util::AppData;

use super::util::{update_tag, TileBuilder};
use super::Tile;

impl Tile {
    pub fn app_tile(launcher: &Launcher, commands: &HashSet<AppData>) -> Vec<SherlockRow> {
        commands
            .into_iter()
            .map(|value| {
                let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/tile.ui");

                // Icon stuff
                if let Some(icon_name) = value.icon.as_ref().or_else(|| launcher.icon.as_ref()) {
                    builder.icon.and_then(|tmp| tmp.upgrade()).map(|icon| {
                        if icon_name.starts_with("/") {
                            icon.set_from_file(Some(icon_name));
                        } else {
                            icon.set_icon_name(Some(icon_name));
                        }
                        value.icon_class.as_ref().map(|c| icon.add_css_class(c));
                    });
                } else {
                    builder
                        .icon
                        .and_then(|tmp| tmp.upgrade())
                        .map(|icon| icon.set_visible(false));
                }

                let update_closure = {
                    // Construct attrs and enable action capabilities
                    let tag_start = builder.tag_start.clone();
                    let tag_end = builder.tag_end.clone();
                    let tag_start_content = launcher.tag_start.clone();
                    let tag_end_content = launcher.tag_end.clone();
                    let title = builder.title.clone();
                    let category = builder.category.clone();
                    let row_weak = builder.object.downgrade();

                    let launcher = launcher.clone();
                    let attrs =
                        get_attrs_map(vec![("method", &launcher.method), ("exec", &value.exec)]);
                    let attrs_rc = Rc::new(RefCell::new(attrs));
                    let name = value.name.clone();
                    move |keyword: &str| -> bool {
                        let attrs = Rc::clone(&attrs_rc);
                        attrs
                            .borrow_mut()
                            .insert(String::from("keyword"), keyword.to_string());

                        let tile_name = name.replace("{keyword}", keyword);

                        // update first tag
                        if let Some(tag_start) = &tag_start {
                            update_tag(&tag_start, &tag_start_content, keyword);
                        }

                        // update second tag
                        if let Some(tag_end) = &tag_end {
                            update_tag(&tag_end, &tag_end_content, keyword);
                        }

                        title
                            .as_ref()
                            .and_then(|tmp| tmp.upgrade())
                            .map(|title| title.set_text(&tile_name));

                        category.as_ref().and_then(|tmp| tmp.upgrade()).map(|cat| {
                            if let Some(name) = &launcher.name {
                                cat.set_text(name);
                            } else {
                                cat.set_visible(false);
                            }
                        });

                        row_weak.upgrade().map(|row| {
                            let signal_id =
                                row.connect_local("row-should-activate", false, move |row| {
                                    let row =
                                        row.first().map(|f| f.get::<SherlockRow>().ok())??;
                                    execute_from_attrs(&row, &attrs.borrow());
                                    None
                                });
                            row.set_signal_id(signal_id);
                        });
                        false
                    }
                };

                builder.object.set_update(update_closure);
                builder.object.set_search(&value.search_string);
                builder.object.with_launcher(launcher);
                builder.object.set_priority(value.priority);
                if launcher.shortcut {
                    builder.object.set_shortcut_holder(builder.shortcut_holder);
                }
                builder.object
            })
            .collect()
    }
}
