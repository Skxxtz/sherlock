use std::{
    fmt::Debug,
    fs::{self, File},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use bincode;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    sherlock_msg,
    utils::errors::{
        SherlockMessage,
        types::{DirAction, FileAction, SherlockErrorType},
    },
};

pub struct BinaryCache;
impl BinaryCache {
    pub fn write<T: Serialize + Debug, P: AsRef<Path>>(
        path: P,
        data: &T,
    ) -> Result<(), SherlockMessage> {
        let cache = path.as_ref();

        // Encode to binary
        let cfg = bincode::config::standard().with_fixed_int_encoding();
        let encoded = bincode::serde::encode_to_vec(data, cfg)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::SerializationError, e))?;

        std::fs::write(cache, encoded).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, cache.to_path_buf()),
                e
            )
        })?;

        Ok(())
    }
    pub fn read<T: DeserializeOwned + Default + Clone + Debug, P: AsRef<Path>>(
        path: P,
    ) -> Result<T, SherlockMessage> {
        let cache = path.as_ref();

        let bytes = std::fs::read(cache).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, cache.to_path_buf()),
                e
            )
        })?;

        // Decode binary
        let cfg = bincode::config::standard().with_fixed_int_encoding();
        match bincode::serde::decode_from_slice::<T, _>(&bytes, cfg) {
            Ok(decoded) => Ok(decoded.0),
            Err(e) => {
                let _ = fs::remove_file(path);
                Err(sherlock_msg!(
                    Warning,
                    SherlockErrorType::DeserializationError,
                    e
                ))
            }
        }
    }
}

pub trait JsonCache: Serialize + DeserializeOwned + Default + Clone + Debug {
    fn cache_path() -> PathBuf;

    fn write_to_cache(&self) -> Result<(), SherlockMessage> {
        let path = Self::cache_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::DirError(DirAction::Create, parent.to_path_buf()),
                    e
                )
            })?;
        }

        let content = simd_json::to_string(self)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))?;

        fs::write(&path, content).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, path),
                e
            )
        })
    }

    fn read_from_cache(age_minutes: u64) -> Result<Option<Self>, SherlockMessage> {
        let path = Self::cache_path();

        if !path.exists() {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path),
                "File not found."
            ));
        }

        let mtime = fs::metadata(&path)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::IO, e))?
            .modified()
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::IO, e))?;

        let time_since = SystemTime::now()
            .duration_since(mtime)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::IO, e))?;

        if time_since < Duration::from_secs(60 * age_minutes) {
            let reader = File::open(&path).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::FileError(FileAction::Read, path.clone()),
                    e
                )
            })?;

            return simd_json::from_reader(reader)
                .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::DeserializationError, e))
                .map(Some);
        }

        Ok(None)
    }
}
