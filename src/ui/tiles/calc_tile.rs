use gio::glib::object::ObjectExt;
use gtk4::prelude::WidgetExt;
use meval::eval_str;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use super::util::TileBuilder;
use super::Tile;
use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{
        calc_launcher::{Calculator, CalculatorLauncher},
        Launcher,
    },
};

impl Tile {
    pub fn calc_tile(launcher: &Launcher, calc_launcher: &CalculatorLauncher) -> Vec<SherlockRow> {
        let capabilities: HashSet<String> = match &calc_launcher.capabilities {
            Some(c) => c.iter().map(|s| s.to_string()).collect(),
            _ => HashSet::from([String::from("calc.math"), String::from("calc.units")]),
        };
        let capability_rc = Rc::new(RefCell::new(capabilities));

        let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/calc_tile.ui");

        // Add action capabilities
        builder.object.add_css_class("calc-tile");
        builder.object.with_launcher(launcher);

        let method_clone = launcher.method.clone();
        let object_weak = builder.object.downgrade();
        let capability_clone = Rc::clone(&capability_rc);
        let update_closure = move |search_query: &str| -> bool {
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

            if (capabilities.contains("calc.temperatures") || capabilities.contains("calc.units"))
                && result.is_none()
            {
                result = Calculator::temperature(&search_query)
            }
            if let Some((num, result_text)) = result {
                builder
                    .equation_holder
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|eq| eq.set_text(&search_query));
                builder
                    .result_holder
                    .as_ref()
                    .and_then(|tmp| tmp.upgrade())
                    .map(|result| result.set_text(&result_text));
                let attrs = get_attrs_map(vec![("method", &method_clone), ("result", &num)]);

                object_weak.upgrade().map(|row| {
                    let signal_id = row.connect_local("row-should-activate", false, move |row| {
                        let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                        execute_from_attrs(&row, &attrs);
                        None
                    });
                    row.set_signal_id(signal_id);
                });
                return true;
            }
            false
        };
        builder.object.set_update(update_closure);
        if launcher.shortcut {
            builder.object.set_shortcut_holder(builder.shortcut_holder);
        }
        vec![builder.object]
    }
}
