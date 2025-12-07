use crate::loader::util::AppData;

#[derive(Clone, Debug)]
pub struct WebLauncher {
    pub display_name: String,
    pub icon: String,
    pub engine: String,
    pub browser: Option<String>,
    pub app_data: Vec<AppData>,
}
