use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::object::ObjectExt;
use gio::glib::WeakRef;
use gtk4::{prelude::*, Widget};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::vec;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::bulk_text_launcher::{AsyncCommandResponse, BulkTextLauncher};
use crate::launcher::Launcher;
use crate::prelude::{IconComp, TileHandler};

use super::Tile;

impl Tile {
    pub fn api(launcher: Rc<Launcher>, api: &BulkTextLauncher) -> ApiTile {
        let tile = ApiTile::new();
        let imp = tile.imp();

        // Set category name
        if let Some(name) = &launcher.name {
            imp.category.set_text(name);
        } else {
            imp.category.set_visible(false);
        }

        // Set icons
        imp.icon.set_icon(Some(&api.icon), None, None);
        imp.icon.set_pixel_size(15);

        tile
    }
}
mod imp {
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Image, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/bulk_text_tile.ui")]
    pub struct ApiTile {
        #[template_child(id = "launcher-type")]
        pub category: TemplateChild<Label>,

        #[template_child(id = "icon-name")]
        pub icon: TemplateChild<Image>,

        #[template_child(id = "content-title")]
        pub content_title: TemplateChild<Label>,

        #[template_child(id = "content-body")]
        pub content_body: TemplateChild<Label>,

        #[template_child(id = "shortcut-holder")]
        pub shortcut_holder: TemplateChild<GtkBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApiTile {
        const NAME: &'static str = "ApiTile";
        type Type = super::ApiTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ApiTile {}
    impl WidgetImpl for ApiTile {}
    impl BoxImpl for ApiTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct ApiTile(ObjectSubclass<imp::ApiTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl ApiTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}

#[derive(Default, Debug)]
pub struct ApiTileHandler {
    tile: WeakRef<ApiTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
}
impl ApiTileHandler {
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
    pub async fn update_async(
        &self,
        keyword: &str,
        launcher: Rc<Launcher>,
        row: &SherlockRow,
    ) -> Option<()> {
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();

        imp.content_title.set_text(&keyword);

        if let Some(response) = launcher.get_result(&keyword).await {
            let AsyncCommandResponse {
                title,
                content,
                next_content,
                actions,
                result,
            } = response;
            if let Some(title) = title {
                imp.content_title.set_text(&title);
            }
            if let Some(content) = content {
                imp.content_body.set_markup(&content);
            }

            if let Some(action) = actions {
                let open = !action.is_empty();
                let _ = row.activate_action("win.context-mode", Some(&open.to_variant()));
                row.set_actions(action);
            }

            if let Some(next_content) = next_content {
                let mut attrs = self.attrs.borrow_mut();
                attrs.insert(String::from("next_content"), next_content.to_string());
                attrs.insert(String::from("keyword"), keyword.to_string());
                result.map(|result| attrs.insert(String::from("result"), result));
            }
        }
        Some(())
    }
    pub fn bind_signal(&self, row: &SherlockRow, launcher: Rc<Launcher>) {
        row.add_css_class("bulk-text");
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
}
impl TileHandler for ApiTileHandler {
    fn replace_tile(&mut self, tile: &Widget) {
        if let Some(tile) = tile.downcast_ref::<ApiTile>() {
            self.tile = tile.downgrade()
        }
    }
}
