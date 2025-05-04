use std::vec;

use gio::glib::object::ObjectExt;
use gtk4::prelude::WidgetExt;

use super::util::EventTileBuilder;
use super::Tile;
use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::event_launcher::EventLauncher;
use crate::launcher::Launcher;

impl Tile {
    pub fn event_tile(launcher: &Launcher, event_launcher: &EventLauncher) -> Vec<SherlockRow> {
        let event = match &event_launcher.event {
            Some(event) => event,
            None => return vec![],
        };
        let builder = EventTileBuilder::new("/dev/skxxtz/sherlock/ui/event_tile.ui");

        builder
            .title
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|title| {
                title.set_text(&event.title);
            });

        builder
            .icon
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|ico| ico.set_icon_name(Some(event_launcher.icon.as_ref())));
        builder
            .start_time
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|start_time| start_time.set_text(&event.start_time));
        builder
            .end_time
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|end_time| end_time.set_text(format!(".. {}", event.end_time).as_str()));

        let mut constructor: Vec<(&str, &str)> = vec![
            ("method", &launcher.method),
            ("meeting_url", &event.meeting_url),
        ];
        if let Some(next) = launcher.next_content.as_deref() {
            constructor.push(("next_content", next));
        }
        let attrs = get_attrs_map(constructor);

        builder.object.add_css_class("event-tile");
        builder.object.with_launcher(launcher);
        builder
            .object
            .connect_local("row-should-activate", false, move |row| {
                let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                execute_from_attrs(&row, &attrs);
                None
            });

        if launcher.shortcut {
            builder.object.set_shortcut_holder(builder.shortcut_holder);
        }
        return vec![builder.object];
    }
}
