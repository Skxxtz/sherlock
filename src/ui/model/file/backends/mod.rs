use crate::ui::model::file::{FileResult, ResultHeap};
use std::path::PathBuf;
use tokio::sync::mpsc::{Receiver, Sender};

pub mod ripgrep;
pub mod walkdir;

use ripgrep::RipgrepBackend;
use serde::{Deserialize, Serialize};
use walkdir::WalkdirBackend;

macro_rules! define_backend {
    ( enum $name:ident { $( $variant:ident( $inner:ty ) ),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        #[serde(rename_all = "lowercase")]
        pub enum $name {
            $($variant($inner),)*
        }

        impl FileSearchBackend {
            pub fn search(
                &self,
                query: String,
                paths: Vec<PathBuf>,
                heap: &mut ResultHeap,
                cancel_rx: Receiver<()>,
                result_tx: &Sender<Vec<FileResult>>,
            ) -> bool {
                match self {
                    $(
                        Self::$variant(inner) => <$inner as FileSearchProvider>::search(inner, query, paths, heap, cancel_rx, result_tx),
                    )*
                }
            }
        }
    }
}

define_backend! {
    enum FileSearchBackend {
        Walkdir(WalkdirBackend),
        Ripgrep(RipgrepBackend),
    }
}

pub trait FileSearchProvider {
    fn name(&self) -> &'static str;
    fn search(
        &self,
        query: String,
        paths: Vec<PathBuf>,
        heap: &mut ResultHeap,
        cancel_rx: Receiver<()>,
        tx: &Sender<Vec<FileResult>>,
    ) -> bool;
}
