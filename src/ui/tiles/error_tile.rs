use gtk4::prelude::*;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

use super::Tile;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::ui::g_templates::ErrorTile;
use crate::utils::errors::SherlockError;

impl Tile {
    pub fn error_tile<T: AsRef<SherlockError>>(
        index: i32,
        errors: &Vec<T>,
        icon: &str,
        tile_type: &str,
    ) -> (i32, Vec<SherlockRow>) {
        let widgets: Vec<SherlockRow> = errors
            .into_iter()
            .map(|e| {
                let err = e.as_ref();
                let tile = ErrorTile::new();
                let imp = tile.imp();
                let object = SherlockRow::new();
                object.append(&tile);

                if let Some(class) = match tile_type {
                    "ERROR" => Some("error"),
                    "WARNING" => Some("warning"),
                    _ => None,
                } {
                    object.set_css_classes(&["error-tile", class]);
                }
                let (name, message) = err.error.get_message();
                imp.title
                    .set_text(format!("{:5}{}:  {}", icon, tile_type, name).as_str());
                imp.content_title.set_markup(&message);
                imp.content_body.set_markup(&err.traceback.trim());
                object
            })
            .collect();

        (index + widgets.len() as i32, widgets)
    }
}
