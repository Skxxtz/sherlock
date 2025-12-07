use super::Tile;
use crate::{
    actions::{execute_from_attrs, get_attrs_map},
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{calc_launcher::Calculator, Launcher},
    prelude::TileHandler,
    ui::g_templates::CalcTile,
};
use gio::glib::{
    object::{Cast, ObjectExt},
    WeakRef,
};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
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
    pub fn change_attrs(&self, key: String, value: String) {
        self.attrs.borrow_mut().insert(key, value);
    }
    pub fn bind_signal(&self, row: &SherlockRow, launcher: Rc<Launcher>) {
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
            execute_from_attrs(&row, &attrs.borrow(), param, Some(launcher.clone()));
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
