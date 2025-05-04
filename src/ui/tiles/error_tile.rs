use gtk4::prelude::*;

use super::{util::TileBuilder, Tile};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::utils::errors::SherlockError;

impl Tile {
    pub fn error_tile(
        index: i32,
        errors: &Vec<SherlockError>,
        icon: &str,
        tile_type: &str,
    ) -> (i32, Vec<SherlockRow>) {
        let widgets: Vec<SherlockRow> = errors
            .iter()
            .map(|e| {
                let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/error_tile.ui");

                if let Some(class) = match tile_type {
                    "ERROR" => Some("error"),
                    "WARNING" => Some("warning"),
                    _ => None,
                } {
                    builder.object.set_css_classes(&["error-tile", class]);
                }
                let (name, message) = e.error.get_message();
                builder
                    .title
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|title| {
                        title.set_text(format!("{:5}{}:  {}", icon, tile_type, name).as_str())
                    });
                builder
                    .content_title
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|title| title.set_text(&message));
                builder
                    .content_body
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|body| body.set_text(&e.traceback.trim()));
                builder.object
            })
            .collect();

        (index + widgets.len() as i32, widgets)
    }
}
