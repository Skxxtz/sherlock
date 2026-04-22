use crate::{
    app::RenderableChildEntity,
    launcher::Launcher,
    ui::{model::file::FileSearchModel, widgets::RenderableChild},
};
use gpui::{App, AppContext, SharedString, Task};
use std::{rc::Rc, sync::Arc};

pub mod emoji;
pub mod file;
pub mod home;
pub mod message;

pub enum Model {
    Standard {
        data: RenderableChildEntity,
        filtered_indices: Arc<[usize]>,
        last_query: Option<SharedString>,
        deferred_render_task: Option<Task<Option<()>>>,
    },
    FileSearch {
        data: RenderableChildEntity,
        filtered_indices: Arc<[usize]>,
        last_query: Option<SharedString>,
        search: FileSearchModel,
    },
}

impl Model {
    pub fn standard(data: Vec<RenderableChild>, cx: &mut App) -> Self {
        let range: Arc<[usize]> = (0..data.len()).collect::<Vec<_>>().into();

        Self::Standard {
            data: cx.new(|_| Rc::new(data)),
            filtered_indices: range,
            last_query: None,
            deferred_render_task: None,
        }
    }
    pub fn standard_with_entity(entity: RenderableChildEntity, cx: &mut App) -> Self {
        let range: Arc<[usize]> = (0..entity.read(cx).len()).collect::<Vec<_>>().into();

        Self::Standard {
            data: entity,
            filtered_indices: range,
            last_query: None,
            deferred_render_task: None,
        }
    }

    pub fn file_search(launcher: Arc<Launcher>, dir: Option<SharedString>, cx: &mut App) -> Self {
        Self::FileSearch {
            data: cx.new(|_| Rc::new(Vec::new())),
            filtered_indices: Arc::from([]),
            last_query: None,
            search: FileSearchModel::new(launcher, dir),
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

    pub fn data(&self) -> RenderableChildEntity {
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

    pub fn last_query(&self) -> Option<SharedString> {
        match self {
            Self::Standard { last_query, .. } => last_query.clone(),
            Self::FileSearch { last_query, .. } => last_query.clone(),
        }
    }
}
