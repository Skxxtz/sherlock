use std::{fmt::Debug, fs, path::Path};

use bincode;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    sherlock_msg,
    utils::errors::{
        SherlockMessage,
        types::{FileAction, SherlockErrorType},
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
        let encoded = bincode::serde::encode_to_vec(&data, cfg)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::SerializationError, e))?;

        std::fs::write(&cache, encoded).map_err(|e| {
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

        let bytes = std::fs::read(&cache).map_err(|e| {
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
