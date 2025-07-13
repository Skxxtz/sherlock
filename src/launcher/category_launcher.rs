use std::rc::Rc;

use crate::{g_subclasses::tile_item::TileItem, launcher::Launcher, loader::util::AppData};

#[derive(Clone, Debug)]
pub struct CategoryLauncher {
    pub categories: Vec<AppData>,
}
impl CategoryLauncher {
    pub fn get_obj(&self, launcher: Rc<Launcher>) -> Vec<TileItem>{
        self.categories.iter().enumerate().map(|(i, _app)| {
            let base = TileItem::new();
            base.set_index(i);
            base.set_launcher(launcher.clone());

            base
        }).collect()
    }
}
