use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::process_launcher::ProcessLauncher;
use crate::launcher::Launcher;

use super::util::TileBuilder;
use super::Tile;

impl Tile {
    pub fn process_tile(launcher: &Launcher, proc: &ProcessLauncher) -> Vec<SherlockRow> {
        proc.processes
            .iter()
            .map(|(key, value)| {
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
                    .title
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|title| title.set_markup(&value));

                builder
                    .icon
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|icon| {
                        if proc.icon.starts_with("/") {
                            icon.set_from_file(Some(&proc.icon));
                        } else {
                            icon.set_icon_name(Some(&proc.icon));
                        }
                    });

                // parent and child process ids
                let (ppid, cpid) = key;
                let parent = ppid.to_string();
                let child = cpid.to_string();

                let row_weak = builder.object.downgrade();
                let update_closure = {
                    // Construct attrs and enable action capabilities
                    let row = row_weak.clone();
                    let attrs = get_attrs_map(vec![
                        ("method", "kill-process"),
                        ("result", &value),
                        ("parent-pid", &parent),
                        ("child-pid", &child),
                    ]);
                    let attrs_rc = Rc::new(RefCell::new(attrs));
                    move |keyword: &str| -> bool {
                        let attrs = Rc::clone(&attrs_rc);
                        attrs
                            .borrow_mut()
                            .insert(String::from("keyword"), keyword.to_string());

                        row.upgrade().map(|row| {
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
                builder.object.with_launcher(&launcher);
                builder.object.set_search(&value);
                if launcher.shortcut {
                    builder.object.set_shortcut_holder(builder.shortcut_holder);
                }

                builder.object
            })
            .collect()
    }
}
