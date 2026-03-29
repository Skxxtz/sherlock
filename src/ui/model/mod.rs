use crate::{
    launcher::{Launcher, children::RenderableChild},
    ui::model::file::FileSearchModel,
};
use gpui::{App, AppContext, Entity, Task};
use std::sync::Arc;

pub mod emoji;
pub mod file;
pub mod home;
pub mod message;

pub enum Model {
    Standard {
        data: Entity<Arc<Vec<RenderableChild>>>,
        filtered_indices: Arc<[usize]>,
        last_query: Option<String>,
        deferred_render_task: Option<Task<Option<()>>>,
    },
    FileSearch {
        data: Entity<Arc<Vec<RenderableChild>>>,
        filtered_indices: Arc<[usize]>,
        search: FileSearchModel,
    },
}

impl Model {
    pub fn standard(data: Vec<RenderableChild>, cx: &mut App) -> Self {
        let range: Arc<[usize]> = (0..data.len()).collect::<Vec<_>>().into();

        Self::Standard {
            data: cx.new(|_| Arc::new(data)),
            filtered_indices: range,
            last_query: None,
            deferred_render_task: None,
        }
    }
    pub fn standard_with_entity(entity: Entity<Arc<Vec<RenderableChild>>>, cx: &mut App) -> Self {
        let range: Arc<[usize]> = (0..entity.read(cx).len()).collect::<Vec<_>>().into();

        Self::Standard {
            data: entity,
            filtered_indices: range,
            last_query: None,
            deferred_render_task: None,
        }
    }

    pub fn file_search(launcher: Arc<Launcher>, cx: &mut App) -> Self {
        Self::FileSearch {
            data: cx.new(|_| Arc::new(Vec::new())),
            filtered_indices: Arc::from([]),
            search: FileSearchModel::new(launcher),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Standard {
                filtered_indices, ..
            } => filtered_indices.len(),
            Self::FileSearch {
                filtered_indices, ..
            } => filtered_indices.len(),
        }
    }

    pub fn data(&self) -> Entity<Arc<Vec<RenderableChild>>> {
        match self {
            Self::Standard { data, .. } => data.clone(),
            Self::FileSearch { data, .. } => data.clone(),
        }
    }

    pub fn filtered_indices(&self) -> Arc<[usize]> {
        match self {
            Self::Standard {
                filtered_indices, ..
            } => Arc::clone(filtered_indices),
            Self::FileSearch {
                filtered_indices, ..
            } => Arc::clone(filtered_indices),
        }
    }
}
