use std::sync::Arc;

use gpui::App;

use crate::{launcher::Launcher, ui::model::Model};

pub struct FileView {
    pub model: Model,
}

impl FileView {
    pub fn new(launcher: Arc<Launcher>, cx: &mut App) -> Self {
        Self {
            model: Model::file_search(launcher, cx),
        }
    }
}
