use std::collections::HashSet;

use crate::loader::util::AppData;

#[derive(Clone, Debug)]
pub struct CommandLauncher {
    pub commands: HashSet<AppData>,
}
