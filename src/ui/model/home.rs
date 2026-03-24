use std::sync::Arc;

use gpui::{App, Entity};

use crate::{launcher::children::RenderableChild, ui::model::Model};

pub struct HomeView {
    pub model: Model,
}

impl HomeView {
    pub fn new(entity: Entity<Arc<Vec<RenderableChild>>>, cx: &mut App) -> Self {
        Self {
            model: Model::new_with_entity(entity, cx),
        }
    }
}
