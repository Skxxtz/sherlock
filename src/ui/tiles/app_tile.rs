use gtk4::prelude::*;
use levenshtein::levenshtein;
use std::collections::HashMap;

use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::{Launcher, ResultItem};
use crate::loader::util::AppData;

use super::util::{SherlockSearch, TileBuilder};
use super::Tile;

impl Tile {
    pub fn app_tile(
        launcher: &Launcher,
        keyword: &str,
        commands: &HashMap<String, AppData>,
    ) -> Vec<ResultItem> {
        let mut results: Vec<ResultItem> = Default::default();

        for (key, value) in commands.into_iter() {
            if value
                .search_string
                .to_lowercase()
                .fuzzy_match(&keyword.to_lowercase())
            {
                let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/tile.ui");

                let tile_name = key.replace("{keyword}", keyword);
                builder.display_tag_start(&value.tag_start, keyword);
                builder.display_tag_end(&value.tag_end, keyword);

                builder.category.upgrade().map(|cat| {
                    if let Some(name) = &launcher.name {
                        cat.set_text(name);
                    } else {
                        cat.set_visible(false);
                    }
                });

                // Icon stuff
                builder.icon.upgrade().map(|ico| {
                    ico.set_icon_name(Some(&value.icon));
                    value.icon_class.as_ref().map(|c| ico.add_css_class(c));
                });

                builder
                    .title
                    .upgrade()
                    .map(|title| title.set_markup(&tile_name));

                let attrs =
                    get_attrs_map(vec![("method", &launcher.method), ("exec", &value.exec)]);

                builder.object.set_spawn_focus(launcher.spawn_focus);
                builder.object.set_shortcut(launcher.shortcut);
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

                let priority = value.priority;
                let mut edits = 0.0;

                if !value.search_string.starts_with(keyword) {
                    edits = levenshtein(&value.search_string, keyword) as f32;
                }

                results.push(ResultItem {
                    priority: if keyword.is_empty() {
                        priority
                    } else {
                        edits + priority
                    },
                    row_item: builder.object,
                    shortcut_holder,
                });
            }
        }
        return results;
    }
}
