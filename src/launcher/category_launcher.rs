use crate::loader::util::AppData;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct CategoryLauncher {
    pub categories: HashSet<AppData>,
}
