use std::{fmt::Debug, fs, path::Path};

use bincode;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};

pub struct BinaryCache;
impl BinaryCache {
    pub fn write<T: Serialize + Debug, P: AsRef<Path>>(
        path: P,
        data: &T,
    ) -> Result<(), SherlockError> {
        let cache = path.as_ref();

        // Encode to binary
        let cfg = bincode::config::standard().with_fixed_int_encoding();
        let encoded = bincode::serde::encode_to_vec(data, cfg)
            .map_err(|e| sherlock_error!(SherlockErrorType::SerializationError, e.to_string()))?;

        std::fs::write(cache, encoded).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileWriteError(cache.to_path_buf()),
                e.to_string()
            )
        })?;

        Ok(())
    }
    pub fn read<T: DeserializeOwned + Default + Clone + Debug, P: AsRef<Path>>(
        path: P,
    ) -> Result<T, SherlockError> {
        let cache = path.as_ref();

        let bytes = std::fs::read(cache).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileReadError(cache.to_path_buf()),
                e.to_string()
            )
        })?;

        // Decode binary
        let cfg = bincode::config::standard().with_fixed_int_encoding();
        match bincode::serde::decode_from_slice::<T, _>(&bytes, cfg) {
            Ok(decoded) => Ok(decoded.0),
            Err(_) => {
                let _ = fs::remove_file(path);
                Ok(T::default())
            }
        }
    }
}
