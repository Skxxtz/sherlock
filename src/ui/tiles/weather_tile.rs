use chrono::Local;
use gio::glib::WeakRef;
use gtk4::prelude::*;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::Tile;
use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::weather_launcher::WeatherData;
use crate::launcher::Launcher;
use crate::prelude::TileHandler;
use crate::ui::g_templates::WeatherTile;

impl Tile {
    pub fn weather() -> WeatherTile {
        WeatherTile::new()
    }
}

#[derive(Debug, Default)]
pub struct WeatherTileHandler {
    tile: WeakRef<WeatherTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
    data: RefCell<Option<WeatherData>>,
}
impl WeatherTileHandler {
    pub fn new(launcher: Rc<Launcher>) -> Self {
        let attrs: HashMap<String, String> = vec![("method", &launcher.method)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Self {
            tile: WeakRef::new(),
            attrs: Rc::new(RefCell::new(attrs)),
            data: RefCell::new(None),
        }
    }
    pub async fn async_update(&self, row: &SherlockRow, launcher: Rc<Launcher>) -> Option<()> {
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();
        if let Some((mut data, was_changed)) = launcher.get_weather().await {
            let css_class = if was_changed {
                "weather-animate"
            } else {
                "weather-no-animate"
            };

            row.add_css_class(css_class);
            row.add_css_class(&data.css);
            
            let current_time = Local::now().time();
            if (data.sunset - current_time).num_seconds() < 0 {
                data.icon.push_str("-night");
                row.add_css_class("night");
            }

            imp.temperature.set_text(&data.temperature);
            imp.icon.set_icon_name(Some(&data.icon));
            imp.location.set_text(&data.format_str);
            imp.spinner.set_spinning(false);
            self.data.borrow_mut().replace(data);
        } else {
            imp.location.set_text("! Failed to load weather");
            imp.icon.set_icon_name(Some("weather-none-available"));
            imp.spinner.set_spinning(false);
        }
        Some(())
    }
    pub fn update(&self, row: &SherlockRow) -> Option<()> {
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();
        if let Some(data) = &*self.data.borrow() {
            row.add_css_class("weather-no-animate");
            row.add_css_class(&data.icon);

            let current_time = Local::now().time();
            if (data.sunset - current_time).num_seconds() < 0 {
                row.add_css_class("night");
                imp.icon.set_icon_name(Some(&format!("{}-night", data.icon)));
            } else {
                imp.icon.set_icon_name(Some(&data.icon));
            }

            imp.temperature.set_text(&data.temperature);
            imp.location.set_text(&data.format_str);
            imp.spinner.set_spinning(false);
        } else {
            imp.location.set_text("! Failed to load weather");
            imp.icon.set_icon_name(Some("weather-none-available"));
            imp.spinner.set_spinning(false);
        }
        Some(())
    }
    pub fn bind_signal(&self, row: &SherlockRow, launcher: Rc<Launcher>) {
        row.add_css_class("tile");
        row.add_css_class("weather-tile");
        let attrs = self.attrs.clone();

        let signal_id = row.connect_local("row-should-activate", false, move |args| {
            let row = args.first().map(|f| f.get::<SherlockRow>().ok())??;
            let param: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
            let param: Option<bool> = match param {
                1 => Some(false),
                2 => Some(true),
                _ => None,
            };
            execute_from_attrs(&row, &attrs.borrow(), param, Some(launcher.clone()));
            None
        });
        row.set_signal_id(signal_id);
    }
}
impl TileHandler for WeatherTileHandler {
    fn replace_tile(&mut self, tile: &gtk4::Widget) {
        if let Some(tile) = tile.downcast_ref::<WeatherTile>() {
            self.tile = tile.downgrade();
        }
    }
}
