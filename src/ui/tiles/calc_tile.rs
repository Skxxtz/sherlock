use super::Tile;
use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{
        calc_launcher::{Calculator, CalculatorLauncher},
        Launcher,
    },
};
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::object::ObjectExt;
use gtk4::prelude::{BoxExt, WidgetExt};
use meval::eval_str;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

impl Tile {
    pub fn calculator(
        launcher: Rc<Launcher>,
        calc_launcher: &CalculatorLauncher,
        object: SherlockRow,
    ) -> Option<SherlockRow> {
        let capabilities: HashSet<String> = calc_launcher.capabilities.clone();
        let capability_rc = Rc::new(RefCell::new(capabilities));

        let tile = CalcTile::new();
        object.append(&tile);

        // Add action capabilities
        object.add_css_class("calc-tile");
        object.with_launcher(launcher.clone());

        let update_closure = {
            let tile_ref = tile.downgrade();
            let method_clone = launcher.method.clone();
            let row_weak = object.downgrade();
            let capability_clone = Rc::clone(&capability_rc);
            let exit = launcher.exit.clone();

            move |search_query: &str| -> bool {
                let Some(tile) = tile_ref.upgrade() else {
                    return false;
                };
                let imp = tile.imp();

                let mut result: Option<(String, String)> = None;
                let capabilities = capability_clone.borrow();
                if capabilities.contains("calc.math") {
                    let trimmed_keyword = search_query.trim();
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
                    result = Calculator::measurement(&search_query, "lengths")
                }

                if (capabilities.contains("calc.weights") || capabilities.contains("calc.units"))
                    && result.is_none()
                {
                    result = Calculator::measurement(&search_query, "weights")
                }

                if (capabilities.contains("calc.volumes") || capabilities.contains("calc.units"))
                    && result.is_none()
                {
                    result = Calculator::measurement(&search_query, "volumes")
                }

                if (capabilities.contains("calc.temperatures")
                    || capabilities.contains("calc.units"))
                    && result.is_none()
                {
                    result = Calculator::temperature(&search_query)
                }

                if (capabilities.contains("calc.currencies") || capabilities.contains("calc.units"))
                    && result.is_none()
                {
                    result = Calculator::measurement(&search_query, "currencies")
                }
                if let Some((num, result_text)) = result {
                    imp.equation_holder.set_text(&search_query);
                    imp.result_holder.set_text(&result_text);
                    let attrs = get_attrs_map(vec![
                        ("method", Some(&method_clone)),
                        ("result", Some(&num)),
                        ("exit", Some(&exit.to_string())),
                    ]);

                    row_weak.upgrade().map(|row| {
                        let signal_id =
                            row.connect_local("row-should-activate", false, move |args| {
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
                        row.set_signal_id(signal_id);
                    });
                    return true;
                }
                false
            }
        };
        object.set_update(update_closure);
        Some(object)
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
