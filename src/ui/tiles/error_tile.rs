use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gtk4::prelude::*;

use super::Tile;
use crate::g_subclasses::sherlock_row::SherlockRow;
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

mod imp {
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/error_tile.ui")]
    pub struct ErrorTile {
        #[template_child(id = "app-name")]
        pub title: TemplateChild<Label>,

        #[template_child(id = "content-title")]
        pub content_title: TemplateChild<Label>,

        #[template_child(id = "content-body")]
        pub content_body: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorTile {
        const NAME: &'static str = "ErrorTile";
        type Type = super::ErrorTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ErrorTile {}
    impl WidgetImpl for ErrorTile {}
    impl BoxImpl for ErrorTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct ErrorTile(ObjectSubclass<imp::ErrorTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl ErrorTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}
