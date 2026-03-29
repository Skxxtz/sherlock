use crate::ui::model::file::{
    backends::{
        command::CommandBackend, fd::FdFactory, ripgrep::RgFactory, walkdir::WalkdirBackend,
    },
    utils::{FileResult, ResultHeap},
};
use std::path::PathBuf;
use tokio::sync::mpsc::{Receiver, Sender};

pub mod command;
pub mod fd;
pub mod ripgrep;
pub mod walkdir;

macro_rules! define_backend {
    ( enum $name:ident { $( $variant:ident( $inner:ty ) ),* $(,)? }) => {
        #[derive(Clone, Debug)]
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

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    $(
                        Self::$variant(_) => serializer.serialize_str(&stringify!($variant).to_lowercase()),
                    )*
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?.to_lowercase();
                match s.as_str() {
                    $(
                        s if s == stringify!($variant).to_lowercase() => {
                            Ok(Self::$variant(<$inner>::default()))
                        }
                    )*
                    _ => Err(serde::de::Error::unknown_variant(&s, &[ $( stringify!($variant) ),* ])),
                }
            }
        }
    }
}

define_backend! {
    enum FileSearchBackend {
        Walkdir(WalkdirBackend),
        Rg(CommandBackend<RgFactory>),
        Fd(CommandBackend<FdFactory>),
    }
}

impl Default for FileSearchBackend {
    fn default() -> Self {
        Self::Fd(Default::default())
    }
}

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_serialization() {
        // Test Rg variant
        let rg = FileSearchBackend::Rg(CommandBackend::default());
        let json = serde_json::to_string(&rg).unwrap();
        assert_eq!(json, "\"rg\"");

        // Test Fd variant
        let fd = FileSearchBackend::Fd(CommandBackend::default());
        let json = serde_json::to_string(&fd).unwrap();
        assert_eq!(json, "\"fd\"");

        // Test Walkdir variant
        let wd = FileSearchBackend::Walkdir(WalkdirBackend::default());
        let json = serde_json::to_string(&wd).unwrap();
        assert_eq!(json, "\"walkdir\"");
    }

    #[test]
    fn test_deserialization() {
        // Test valid string to Rg
        let rg_json = "\"rg\"";
        let backend: FileSearchBackend = serde_json::from_str(rg_json).unwrap();
        match backend {
            FileSearchBackend::Rg(_) => {}
            _ => panic!("Expected Rg variant"),
        }

        // Test case insensitivity (if your logic handles it)
        let fd_json = "\"FD\"";
        let backend: FileSearchBackend = serde_json::from_str(fd_json).unwrap();
        match backend {
            FileSearchBackend::Fd(_) => {}
            _ => panic!("Expected Fd variant from uppercase input"),
        }
    }

    #[test]
    fn test_deserialization_unknown() {
        let invalid_json = "\"not-a-backend\"";
        let result: Result<FileSearchBackend, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }
}
