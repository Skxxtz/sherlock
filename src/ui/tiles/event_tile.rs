use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::vec;

use gio::glib::object::{Cast, ObjectExt};
use gio::glib::variant::ToVariant;
use gio::glib::WeakRef;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{Box, Widget};

use super::Tile;
use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::event_launcher::EventLauncher;
use crate::launcher::Launcher;
use crate::prelude::TileHandler;
use crate::ui::g_templates::EventTile;

impl Tile {
    pub fn event(event_launcher: &EventLauncher) -> Option<EventTile> {
        let event = event_launcher.event.clone();
        let tile = EventTile::new();
        let imp = tile.imp();

        imp.title.set_text(&event.title);
        imp.icon.set_icon_name(Some(event_launcher.icon.as_ref()));
        imp.start_time.set_text(&event.start_time);
        imp.end_time
            .set_text(format!(".. {}", event.end_time).as_str());

        Some(tile)
    }
}

#[derive(Default, Debug)]
pub struct EventTileHandler {
    tile: WeakRef<EventTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
}
impl EventTileHandler {
    pub fn new(launcher: Rc<Launcher>, event: &EventLauncher) -> Self {
        let meeting_url = event.event.meeting_url.as_str();
        let attrs = get_attrs_map(vec![
            ("method", Some(&launcher.method)),
            ("meeting_url", Some(meeting_url)),
            ("next_content", launcher.next_content.as_deref()),
            ("exit", Some(&launcher.exit.to_string())),
        ]);

        Self {
            tile: WeakRef::new(),
            attrs: Rc::new(RefCell::new(attrs)),
        }
    }
    pub fn bind_signal(&self, row: &SherlockRow, launcher: Rc<Launcher>) {
        row.add_css_class("event-tile");
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
impl TileHandler for EventTileHandler {
    fn replace_tile(&mut self, tile: &Widget) {
        if let Some(tile) = tile.downcast_ref::<EventTile>() {
            self.tile = tile.downgrade()
        }
    }
}
