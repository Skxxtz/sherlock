use gtk4::prelude::*;
use std::collections::HashMap;
use std::pin::Pin;

use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::Launcher;

use super::util::WeatherTileBuilder;
use super::Tile;

impl Tile {
    pub fn weather_tile_loader(launcher: &Launcher) -> Vec<SherlockRow> {
        let builder = WeatherTileBuilder::new("/dev/skxxtz/sherlock/ui/weather_tile.ui");
        builder.object.add_css_class("weather-tile");
        builder.object.with_launcher(&launcher);

        // Add attrs and implement double click capabilities
        let attrs: HashMap<String, String> = vec![("method", &launcher.method)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let signal_id = builder
            .object
            .connect_local("row-should-activate", false, move |args| {
                let row = args.first().map(|f| f.get::<SherlockRow>().ok())??;
                let param: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
                let param: Option<bool> = match param {
                    1 => Some(false),
                    2 => Some(true),
                    _ => None,
                };
                execute_from_attrs(&row, &attrs, param);
                None
            });
        builder.object.set_signal_id(signal_id);

        builder
            .icon
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|icon| icon.set_pixel_size(40));

        let launcher_clone = launcher.clone();
        let row_weak = builder.object.downgrade();
        let async_update_closure: Box<dyn Fn(&str) -> Pin<Box<dyn futures::Future<Output = ()>>>> =
            Box::new(move |_keyword: &str| {
                let row = row_weak.clone();
                let temperature = builder.temperature.clone();
                let spinner = builder.spinner.clone();
                let icon = builder.icon.clone();
                let location = builder.location.clone();
                let launcher = launcher_clone.clone();
                Box::pin(async move {
                    if let Some((data, was_changed)) = launcher.get_weather().await {
                        let css_class = if was_changed {
                            "weather-animate"
                        } else {
                            "weather-no-animate"
                        };
                        row.upgrade().map(|row| {
                            row.add_css_class(css_class);
                            row.add_css_class(&data.icon);
                        });
                        temperature
                            .as_ref()
                            .and_then(|tmp| tmp.upgrade())
                            .map(|tmp| tmp.set_text(&data.temperature));
                        spinner.upgrade().map(|spn| spn.set_spinning(false));
                        icon.as_ref()
                            .and_then(|tmp| tmp.upgrade())
                            .map(|ico| ico.set_icon_name(Some(&data.icon)));
                        location
                            .as_ref()
                            .and_then(|tmp| tmp.upgrade())
                            .map(|loc| loc.set_text(&data.format_str));
                    } else {
                        spinner.upgrade().map(|spn| spn.set_spinning(false));
                        location
                            .as_ref()
                            .and_then(|tmp| tmp.upgrade())
                            .map(|loc| loc.set_text("! Failed to load weather"));
                        // row.upgrade().map(|row| row.set_visible(false));
                    }
                })
            });
        builder.object.set_async_update(async_update_closure);
        return vec![builder.object];
    }
}
