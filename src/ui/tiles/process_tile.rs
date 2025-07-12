use std::cell::RefCell;
use std::rc::Rc;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gtk4::prelude::*;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::process_launcher::ProcessLauncher;
use crate::launcher::Launcher;
use crate::prelude::IconComp;

use super::app_tile::AppTile;
use super::Tile;

impl Tile {
    pub async fn process_tile(launcher: Rc<Launcher>, proc: &ProcessLauncher) -> Vec<SherlockRow> {
        let processes = ProcessLauncher::get_all_processes().await.unwrap_or_default();

        processes
            .into_iter()
            .map(|(key, value)| {
                let tile = AppTile::new();
                let imp = tile.imp();
                let object = SherlockRow::new();
                object.append(&tile);

                // Title and category
                if let Some(name) = &launcher.name {
                    imp.category.set_text(name);
                } else {
                    imp.category.set_visible(false);
                }
                imp.title.set_markup(&value);

                // Icon stuff
                imp.icon.set_icon(Some(&proc.icon), None, None);

                // parent and child process ids
                let (ppid, cpid) = key;
                let parent = ppid.to_string();
                let child = cpid.to_string();

                let row_weak = object.downgrade();
                let update_closure = {
                    // Construct attrs and enable action capabilities
                    let row = row_weak.clone();
                    let attrs = get_attrs_map(vec![
                        ("method", Some("kill-process")),
                        ("result", Some(&value)),
                        ("parent-pid", Some(&parent)),
                        ("child-pid", Some(&child)),
                        ("exit", Some(&launcher.exit.to_string())),
                    ]);
                    let attrs_rc = Rc::new(RefCell::new(attrs));
                    move |keyword: &str| -> bool {
                        let attrs = Rc::clone(&attrs_rc);
                        attrs
                            .borrow_mut()
                            .insert(String::from("keyword"), keyword.to_string());

                        row.upgrade().map(|row| {
                            let signal_id =
                                row.connect_local("row-should-activate", false, move |args| {
                                    let row =
                                        args.first().map(|f| f.get::<SherlockRow>().ok())??;
                                    let param: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
                                    let param: Option<bool> = match param {
                                        1 => Some(false),
                                        2 => Some(true),
                                        _ => None,
                                    };
                                    execute_from_attrs(&row, &attrs.borrow(), param);
                                    None
                                });
                            row.set_signal_id(signal_id);
                        });
                        false
                    }
                };

                object.set_update(update_closure);
                object.with_launcher(launcher.clone());
                object.set_search(&value);
                if launcher.shortcut {
                    object.set_shortcut_holder(Some(imp.shortcut_holder.downgrade()));
                }
                object
            })
            .collect()
    }
}
