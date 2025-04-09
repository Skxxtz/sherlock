use gio::glib::object::ObjectExt;
use gtk4::prelude::WidgetExt;
use meval::eval_str;
use std::collections::HashSet;

use super::util::TileBuilder;
use super::Tile;
use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{calc_launcher::Calculator, Launcher, ResultItem},
};

impl Tile {
    pub fn calc_tile(
        launcher: &Launcher,
        calc_launcher: &Calculator,
        keyword: &str,
    ) -> Vec<ResultItem> {
        let capabilities: HashSet<&str> = match &calc_launcher.capabilities {
            Some(c) => c.iter().map(|s| s.as_str()).collect(),
            _ => HashSet::from(["calc.math", "calc.length", "calc.weights"]),
        };
        let mut result: Option<String> = None;

        if capabilities.contains("calc.math") {
            if let Ok(r) = eval_str(keyword.trim()) {
                if r.to_string().as_str() != keyword.trim() {
                    result = Some(format!("= {}", r.to_string()));
                }
            }
        }

        if capabilities.contains("calc.length") && result.is_none() {
            result = calc_launcher.measurement(&keyword, "length").map(|r| r.to_string());
        }

        if capabilities.contains("calc.weight") && result.is_none() {
            result = calc_launcher.measurement(&keyword, "weight").map(|r| r.to_string());
        }

        if let Some(r) = result {
            let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/calc_tile.ui");
            builder.object.add_css_class("calc-tile");
            builder.object.set_spawn_focus(launcher.spawn_focus);
            builder.object.set_shortcut(launcher.shortcut);

            builder.equation_holder.set_text(&keyword);
            builder.result_holder.set_text(&r);

            let result = r.to_string();

            // Add action capabilities
            let attrs = get_attrs_map(vec![("method", &launcher.method), ("result", &result)]);
            builder
                .object
                .connect("row-should-activate", false, move |row| {
                    let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                    execute_from_attrs(&row, &attrs);
                    None
                });

            let shortcut_holder = match launcher.shortcut {
                true => builder.shortcut_holder,
                _ => None,
            };

            let res = ResultItem {
                priority: launcher.priority as f32,
                row_item: builder.object,
                shortcut_holder,
            };

            vec![res]
        } else {
            return vec![];
        }
    }
}
