use crate::g_subclasses::sherlock_row::SherlockRow;

use super::util::SherlockSearch;
use super::util::TileBuilder;
use super::Tile;

impl Tile {
    pub fn simple_text_tile(lines: &Vec<String>, method: &str, keyword: &str) -> Vec<SherlockRow> {
        let mut results: Vec<SherlockRow> = Default::default();

        for item in lines {
            if item.fuzzy_match(keyword) {
                let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/simple_text_tile.ui");
                builder.object.set_spawn_focus(true);

                builder.title.set_text(item);
                builder.add_default_attrs(Some(method), Some(item), Some(keyword), None, None);

                results.push(builder.object);
            }
        }

        return results;
    }
}
