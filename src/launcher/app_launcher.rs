use crate::loader::util::AppData;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct AppLauncher {
    pub apps: HashSet<AppData>,
}
