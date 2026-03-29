use crate::{launcher::children::RenderableChild, ui::model::file::FileSearchModel};
use gpui::{App, AppContext, Entity, Task};
use std::sync::Arc;

pub mod emoji;
pub mod file;
pub mod home;
pub mod message;

pub struct Model {
    pub deferred_render_task: Option<Task<Option<()>>>,
    pub data: Entity<Arc<Vec<RenderableChild>>>,
    pub filtered_indices: Arc<[usize]>,
    pub last_query: Option<String>,
    pub file_search: Option<FileSearchModel>,
}

impl Model {
    pub fn new(data: Vec<RenderableChild>, cx: &mut App) -> Self {
        let range: Arc<[usize]> = (0..data.len()).collect::<Vec<_>>().into();

        Self {
            deferred_render_task: None,
            data: cx.new(|_| Arc::new(data)),
            filtered_indices: range,
            last_query: None,
            file_search: None,
        }
    }
    pub fn new_with_entity(entity: Entity<Arc<Vec<RenderableChild>>>, cx: &mut App) -> Self {
        let range: Arc<[usize]> = (0..entity.read(cx).len()).collect::<Vec<_>>().into();

        Self {
            deferred_render_task: None,
            data: entity,
            filtered_indices: range,
            last_query: None,
            file_search: None,
        }
    }
    pub fn len(&self) -> usize {
        self.filtered_indices.len()
    }
}
