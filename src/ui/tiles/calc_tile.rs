use super::Tile;
use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{calc_launcher::Calculator, Launcher},
    prelude::TileHandler,
};
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{
    object::{Cast, ObjectExt},
    WeakRef,
};
use gtk4::{prelude::WidgetExt, Widget};
use meval::eval_str;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

impl Tile {
    pub fn calculator() -> CalcTile {
        CalcTile::new()
    }
}

mod imp {
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/calc_tile.ui")]
    pub struct CalcTile {
        #[template_child(id = "equation-holder")]
        pub equation_holder: TemplateChild<Label>,

        #[template_child(id = "result-holder")]
        pub result_holder: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CalcTile {
        const NAME: &'static str = "CalcTile";
        type Type = super::CalcTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CalcTile {}
    impl WidgetImpl for CalcTile {}
    impl BoxImpl for CalcTile {}
}

use gtk4::glib;

glib::wrapper! {
    pub struct CalcTile(ObjectSubclass<imp::CalcTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl CalcTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}

#[derive(Debug, Default)]
pub struct CalcTileHandler {
    tile: WeakRef<CalcTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
    pub result: RefCell<Option<(String, String)>>,
}
impl CalcTileHandler {
    pub fn new(launcher: Rc<Launcher>) -> Self {
        let attrs = get_attrs_map(vec![
            ("method", Some(&launcher.method)),
            ("exit", Some(&launcher.exit.to_string())),
        ]);
        Self {
            tile: WeakRef::new(),
            attrs: Rc::new(RefCell::new(attrs)),
            result: RefCell::new(None),
        }
    }
    pub fn based_show(&self, keyword: &str, capabilities: &HashSet<String>) -> bool {
        if keyword.trim().is_empty() {
            return false;
        }

        let mut result = None;

        if capabilities.contains("calc.math") {
            let trimmed_keyword = keyword.trim();
            if let Ok(r) = eval_str(trimmed_keyword) {
                let r = r.to_string();
                if &r != trimmed_keyword {
                    result = Some((r.clone(), format!("= {}", r)));
                }
            }
        }

        if (capabilities.contains("calc.lengths") || capabilities.contains("calc.units"))
            && result.is_none()
        {
            result = Calculator::measurement(&keyword, "lengths");
        }

        if (capabilities.contains("calc.weights") || capabilities.contains("calc.units"))
            && result.is_none()
        {
            result = Calculator::measurement(&keyword, "weights");
        }

        if (capabilities.contains("calc.volumes") || capabilities.contains("calc.units"))
            && result.is_none()
        {
            result = Calculator::measurement(&keyword, "volumes");
        }

        if (capabilities.contains("calc.temperatures") || capabilities.contains("calc.units"))
            && result.is_none()
        {
            result = Calculator::temperature(&keyword);
        }

        if (capabilities.contains("calc.currencies") || capabilities.contains("calc.units"))
            && result.is_none()
        {
            result = Calculator::measurement(&keyword, "currencies");
        }

        *self.result.borrow_mut() = result;
        self.result.borrow().is_some()
    }
    pub fn update(&self, search_query: &str) -> Option<()> {
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();

        if let Some((num, result_text)) = &*self.result.borrow() {
            imp.equation_holder.set_text(&search_query);
            imp.result_holder.set_text(&result_text);
            self.attrs
                .borrow_mut()
                .entry("result".to_string())
                .or_insert(num.to_string());
        }

        Some(())
    }
    pub fn bind_signal(&self, row: &SherlockRow) {
        row.add_css_class("calc-tile");
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
impl TileHandler for CalcTileHandler {
    fn replace_tile(&mut self, tile: &Widget) {
        if let Some(tile) = tile.downcast_ref::<CalcTile>() {
            self.tile = tile.downgrade()
        }
    }
}
