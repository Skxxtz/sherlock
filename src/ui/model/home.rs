use std::sync::Arc;

use gpui::{App, Entity};

use crate::ui::{model::Model, widgets::RenderableChild};

pub struct HomeView {
    pub model: Model,
}

impl HomeView {
    pub fn new(entity: Entity<Arc<Vec<RenderableChild>>>, cx: &mut App) -> Self {
        Self {
            model: Model::standard_with_entity(entity, cx),
        }
    }
}
