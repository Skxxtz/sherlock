use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::Launcher;
use crate::loader::util::AppData;
use crate::prelude::IconComp;
use crate::utils::config::ConfigGuard;

use super::util::update_tag;
use super::Tile;

impl Tile {
    pub async fn app_tile(launcher: Rc<Launcher>, commands: &HashSet<AppData>) -> Vec<SherlockRow> {
        commands
            .into_iter()
            .map(|value| {
                // Append content to the sherlock row
                let tile = AppTile::new();
                let imp = tile.imp();
                let object = SherlockRow::new();
                object.append(&tile);
                object.set_css_classes(&vec!["tile"]);

                // Icon stuff
                imp.icon.set_icon(
                    value.icon.as_deref(),
                    value.icon_class.as_deref(),
                    launcher.icon.as_deref(),
                );

                let update_closure = {
                    // Construct attrs and enable action capabilities
                    let tag_start = imp.tag_start.downgrade();
                    let tag_end = imp.tag_end.downgrade();
                    let tag_start_content = launcher.tag_start.clone();
                    let tag_end_content = launcher.tag_end.clone();
                    let title = imp.title.downgrade();
                    let category = imp.category.downgrade();
                    let row_weak = object.downgrade();

                    let launcher = Rc::clone(&launcher);
                    let attrs = get_attrs_map(vec![
                        ("method", Some(&launcher.method)),
                        ("exec", value.exec.as_deref()),
                        ("term", Some(&value.terminal.to_string())),
                        ("exit", Some(&launcher.exit.to_string())),
                    ]);
                    let attrs_rc = Rc::new(RefCell::new(attrs));
                    let name = value.name.clone();
                    move |keyword: &str| -> bool {
                        let attrs = Rc::clone(&attrs_rc);
                        {
                            let mut attrs_ref = attrs.borrow_mut();
                            attrs_ref.insert(String::from("keyword"), keyword.to_string());
                        }
                        let tile_name = name.replace("{keyword}", keyword);

                        // update first tag
                        update_tag(&tag_start, &tag_start_content, keyword);

                        // update second tag
                        update_tag(&tag_end, &tag_end_content, keyword);

                        title.upgrade().map(|title| title.set_text(&tile_name));

                        category.upgrade().map(|cat| {
                            if let Some(name) = &launcher.name {
                                cat.set_text(name);
                            } else {
                                cat.set_visible(false);
                            }
                        });

                        row_weak.upgrade().map(|row| {
                            let signal_id =
                                row.connect_local("row-should-activate", false, move |args| {
                                    let row =
                                        args.first().map(|f| f.get::<SherlockRow>().ok())??;
                                    let param: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
                                    let param: Option<bool> = match param {
                                        1 => Some(false),
                                        2 => Some(true),
                                        _ => None,
                                    };
                                    execute_from_attrs(&row, &attrs.borrow(), param);
                                    // To reload ui according to mode
                                    let _ = row.activate_action(
                                        "win.update-items",
                                        Some(&false.to_variant()),
                                    );
                                    None
                                });
                            row.set_signal_id(signal_id);
                        });
                        false
                    }
                };

                object.set_update(update_closure);
                object.with_launcher(launcher.clone());
                object.with_appdata(&value);
                object.add_actions(&launcher.add_actions);
                if launcher.shortcut {
                    object.set_shortcut_holder(Some(imp.shortcut_holder.downgrade()));
                }
                object
            })
            .collect()
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
