use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::WeakRef;
use gtk4::{prelude::*, Box, Widget};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::g_subclasses::tile_item::TileItem;
use crate::launcher::Launcher;
use crate::loader::util::AppData;
use crate::prelude::{IconComp, TileHandler};
use crate::utils::config::ConfigGuard;

use super::util::update_tag;
use super::Tile;

impl Tile {
    pub fn app(value: &AppData, launcher: Rc<Launcher>, item: &TileItem) -> AppTile {
        let tile = AppTile::new();
        let imp = tile.imp();

        // Icon stuff
        imp.icon.set_icon(
            value.icon.as_deref(),
            value.icon_class.as_deref(),
            launcher.icon.as_deref(),
        );

        item.add_actions(&launcher.add_actions);
        tile
    }
}
#[derive(Default, Debug)]
pub struct AppTileHandler {
    tile: WeakRef<AppTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
}
impl AppTileHandler {
    pub fn new(launcher: Rc<Launcher>) -> Self {
        let attrs = get_attrs_map(vec![
            ("method", Some(&launcher.method)),
            ("exit", Some(&launcher.exit.to_string())),
        ]);
        Self {
            tile: WeakRef::new(),
            attrs: Rc::new(RefCell::new(attrs)),
        }
    }
    pub fn update(&self, keyword: &str, launcher: Rc<Launcher>, value: &AppData) -> Option<()> {
        // Construct attrs and enable action capabilities
        let tag_start_content = launcher.tag_start.clone();
        let tag_end_content = launcher.tag_end.clone();

        {
            let mut attrs = self.attrs.borrow_mut();
            if let Some(exec) = &value.exec {
                attrs
                    .entry("exec".to_string())
                    .and_modify(|val| *val = exec.to_string())
                    .or_insert_with(|| exec.to_string());
            }
            attrs
                .entry("term".to_string())
                .and_modify(|val| *val = value.terminal.to_string())
                .or_insert_with(|| value.terminal.to_string());
        }

        let name = value.name.clone();
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();

        {
            let mut attrs_ref = self.attrs.borrow_mut();
            attrs_ref.insert(String::from("keyword"), keyword.to_string());
        }
        let tile_name = name.replace("{keyword}", keyword);

        // update first tag
        update_tag(&imp.tag_start, &tag_start_content, keyword);

        // update second tag
        update_tag(&imp.tag_end, &tag_end_content, keyword);

        imp.title.set_text(&tile_name);

        if let Some(name) = &launcher.name {
            imp.category.set_text(name);
        } else {
            imp.category.set_visible(false);
        }

        Some(())
    }
    pub fn bind_signal(&self, row: &SherlockRow, launcher: Rc<Launcher>) {
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
impl TileHandler for AppTileHandler {
    fn replace_tile(&mut self, tile: &Widget) {
        if let Some(tile) = tile.downcast_ref::<AppTile>() {
            self.tile = tile.downgrade()
        }
    }
}

mod imp {
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Image, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/tile.ui")]
    pub struct AppTile {
        #[template_child(id = "app-name")]
        pub title: TemplateChild<Label>,

        #[template_child(id = "launcher-type")]
        pub category: TemplateChild<Label>,

        #[template_child(id = "icon-name")]
        pub icon: TemplateChild<Image>,

        #[template_child(id = "icon-holder")]
        pub icon_holder: TemplateChild<GtkBox>,

        #[template_child(id = "app-name-tag-start")]
        pub tag_start: TemplateChild<Label>,

        #[template_child(id = "app-name-tag-end")]
        pub tag_end: TemplateChild<Label>,

        #[template_child(id = "shortcut-holder")]
        pub shortcut_holder: TemplateChild<GtkBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppTile {
        const NAME: &'static str = "AppTile";
        type Type = super::AppTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppTile {}
    impl WidgetImpl for AppTile {}
    impl BoxImpl for AppTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct AppTile(ObjectSubclass<imp::AppTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl AppTile {
    pub fn new() -> Self {
        let obj = glib::Object::new::<Self>();
        if let Ok(config) = ConfigGuard::read() {
            let imp = obj.imp();
            imp.icon.set_pixel_size(config.appearance.icon_size);
        }
        obj
    }
}
