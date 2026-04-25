use gpui::App;

use crate::{app::RenderableChildEntity, ui::model::Model};

pub struct HomeView {
    pub model: Model,
}

impl HomeView {
    pub fn new(entity: RenderableChildEntity, cx: &mut App) -> Self {
        Self {
            model: Model::standard_with_entity(entity, cx),
        }
    }
}
