use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::WeakRef;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::Tile;
use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::Launcher;
use crate::prelude::TileHandler;

impl Tile {
    pub fn weather() -> WeatherTile {
        WeatherTile::new()
    }
}

mod imp {
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{glib, Spinner};
    use gtk4::{Box as GtkBox, Image, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/weather_tile.ui")]
    pub struct WeatherTile {
        #[template_child(id = "temperature")]
        pub temperature: TemplateChild<Label>,

        #[template_child(id = "location")]
        pub location: TemplateChild<Label>,

        #[template_child(id = "icon-name")]
        pub icon: TemplateChild<Image>,

        #[template_child(id = "spinner")]
        pub spinner: TemplateChild<Spinner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WeatherTile {
        const NAME: &'static str = "WeatherTile";
        type Type = super::WeatherTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WeatherTile {}
    impl WidgetImpl for WeatherTile {}
    impl BoxImpl for WeatherTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct WeatherTile(ObjectSubclass<imp::WeatherTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl WeatherTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}

#[derive(Debug, Default)]
pub struct WeatherTileHandler {
    tile: WeakRef<WeatherTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
}
impl WeatherTileHandler {
    pub fn new(tile: &WeatherTile, launcher: Rc<Launcher>) -> Self {
        let attrs: HashMap<String, String> = vec![("method", &launcher.method)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Self {
            tile: tile.downgrade(),
            attrs: Rc::new(RefCell::new(attrs)),
        }
    }
    pub async fn async_update(&self, row: &SherlockRow, launcher: Rc<Launcher>) -> Option<()> {
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();
        if let Some((data, was_changed)) = launcher.get_weather().await {
            let css_class = if was_changed {
                "weather-animate"
            } else {
                "weather-no-animate"
            };

            row.add_css_class(css_class);
            row.add_css_class(&data.icon);

            imp.temperature.set_text(&data.temperature);
            imp.icon.set_icon_name(Some(&data.icon));
            imp.location.set_text(&data.format_str);
            imp.spinner.set_spinning(false);
        } else {
            imp.location.set_text("! Failed to load weather");
            imp.spinner.set_spinning(false);
        }
        Some(())
    }
    pub fn bind_signal(&self, row: &SherlockRow) {
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
            execute_from_attrs(&row, &attrs.borrow(), param);
            None
        });
        row.set_signal_id(signal_id);
    }
}
impl TileHandler for WeatherTileHandler {
    fn replace_tile(&mut self, tile: &gtk4::Widget) {
        if let Some(tile) = tile.downcast_ref::<WeatherTile>(){
            self.tile = tile.downgrade();
        }
    }
}
