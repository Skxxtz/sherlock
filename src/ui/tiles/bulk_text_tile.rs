use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::object::ObjectExt;
use gtk4::prelude::*;
use std::pin::Pin;
use std::rc::Rc;
use std::vec;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::bulk_text_launcher::{AsyncCommandResponse, BulkTextLauncher};
use crate::launcher::Launcher;
use crate::prelude::IconComp;

use super::Tile;

impl Tile {
    pub async fn bulk_text_tile(
        launcher: Rc<Launcher>,
        bulk_text: &BulkTextLauncher,
    ) -> Vec<SherlockRow> {
        let tile = ApiTile::new();
        let imp = tile.imp();
        let object = SherlockRow::new();
        object.append(&tile);

        // Set category name
        if let Some(name) = &launcher.name {
            imp.category.set_text(name);
        } else {
            imp.category.set_visible(false);
        }

        // Set icons
        imp.icon.set_icon(Some(&bulk_text.icon), None, None);
        imp.icon.set_pixel_size(15);

        object.add_css_class("bulk-text");
        object.with_launcher(launcher.clone());
        object.set_keyword_aware(true);

        let row_weak = object.downgrade();
        let launcher_clone = launcher.clone();
        let async_update_closure: Box<dyn Fn(&str) -> Pin<Box<dyn futures::Future<Output = ()>>>> = {
            let attrs = get_attrs_map(vec![
                ("method", Some(&launcher.method)),
                ("exit", Some(&launcher.exit.to_string())),
            ]);
            let content_title = imp.content_title.downgrade();
            let content_body = imp.content_body.downgrade();

            Box::new(move |keyword: &str| {
                let launcher = launcher_clone.clone();
                let row = row_weak.clone();
                let content_title = content_title.clone();
                let content_body = content_body.clone();
                let mut attrs = attrs.clone();
                let keyword = keyword.to_string();

                Box::pin(async move {
                    content_title.upgrade().map(|t| t.set_text(&keyword));

                    if let Some(response) = launcher.get_result(&keyword).await {
                        let AsyncCommandResponse {
                            title,
                            content,
                            next_content,
                            actions,
                            result,
                        } = response;
                        if let Some(title) = title {
                            content_title.upgrade().map(|t| t.set_text(&title));
                        }
                        if let Some(content) = content {
                            content_body.upgrade().map(|b| b.set_markup(&content));
                        }

                        if let Some(action) = actions {
                            row.upgrade().map(|row| {
                                let open = !action.is_empty();
                                let _ = row
                                    .activate_action("win.context-mode", Some(&open.to_variant()));
                                row.set_actions(action);
                            });
                        }

                        if let Some(next_content) = next_content {
                            attrs.insert(String::from("next_content"), next_content.to_string());
                            attrs.insert(String::from("keyword"), keyword.to_string());
                            result.map(|result| attrs.insert(String::from("result"), result));

                            row.upgrade().map(|row| {
                                let signal_id =
                                    row.connect_local("row-should-activate", false, move |args| {
                                        let row =
                                            args.first().map(|f| f.get::<SherlockRow>().ok())??;
                                        let param: u8 =
                                            args.get(1).and_then(|v| v.get::<u8>().ok())?;
                                        let param: Option<bool> = match param {
                                            1 => Some(false),
                                            2 => Some(true),
                                            _ => None,
                                        };
                                        execute_from_attrs(&row, &attrs, param);
                                        None
                                    });
                                row.set_signal_id(signal_id);
                            });
                        }
                    }
                })
            })
        };
        object.set_async_update(async_update_closure);
        if launcher.shortcut {
            object.set_shortcut_holder(Some(imp.shortcut_holder.downgrade()));
        }
        return vec![object];
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
