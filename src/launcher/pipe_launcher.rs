use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct PipeLauncher {
    pub binary: Option<Vec<u8>>,
    pub description: Option<String>,
    pub hidden: Option<HashMap<String, String>>,
    pub field: Option<String>,
    pub icon_size: Option<i32>,
    pub result: Option<String>,
}
