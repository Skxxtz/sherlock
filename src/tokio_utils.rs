use serde::Serialize;
use serde::de::DeserializeOwned;
use std::future::Future;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

use crate::{
    sherlock_msg,
    utils::errors::{
        SherlockMessage,
        types::{SherlockErrorType, SocketAction},
    },
};

pub struct SizedMessageObj {
    buffer: Vec<u8>,
}

impl SizedMessageObj {
    /// The ONLY way to create a message for the wire.
    /// This guarantees Bincode is used every time.
    pub fn from_struct<T: Serialize>(data: &T) -> Result<Self, SherlockMessage> {
        let config = bincode::config::standard();
        let buffer = bincode::serde::encode_to_vec(data, config).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::SerializationError,
                e.to_string()
            )
        })?;

        Ok(Self { buffer })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buffer
    }
}

pub trait AsyncSizedMessage {
    fn write_sized<'a>(
        &'a mut self,
        what: SizedMessageObj,
    ) -> impl Future<Output = Result<(), SherlockMessage>> + Send + 'a;
    fn read_sized<'a, R: DeserializeOwned>(
        &'a mut self,
    ) -> impl Future<Output = Result<R, SherlockMessage>> + Send + 'a;
}
impl AsyncSizedMessage for UnixStream {
    #[allow(clippy::manual_async_fn)]
    fn write_sized<'a>(
        &'a mut self,
        what: SizedMessageObj,
    ) -> impl Future<Output = Result<(), SherlockMessage>> + Send + 'a {
        async move {
            // Safely convert buf_len from usize to u32
            let buf_len: u32 = what
                .bytes()
                .len()
                .try_into()
                .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::InvalidData, e))?;

            // Write message size to stream
            let len_bytes = buf_len.to_be_bytes();
            self.write_all(&len_bytes).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Write),
                    e
                )
            })?;

            // Write message to stream
            self.write(what.bytes()).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Write),
                    e
                )
            })?;

            Ok(())
        }
    }
    #[allow(clippy::manual_async_fn)]
    fn read_sized<'a, R: serde::de::DeserializeOwned>(
        &'a mut self,
    ) -> impl Future<Output = Result<R, SherlockMessage>> + Send + 'a {
        async move {
            let mut buf_len = [0u8; 4];

            // Read message length
            self.read_exact(&mut buf_len).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Read),
                    e
                )
            })?;
            let msg_len = u32::from_be_bytes(buf_len) as usize;

            let mut buf = vec![0u8; msg_len];
            self.read_exact(&mut buf).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Read),
                    e
                )
            })?;

            let cfg = bincode::config::standard();
            bincode::serde::decode_from_slice::<R, _>(&buf, cfg)
                .map(|(val, _)| val)
                .map_err(|e| {
                    sherlock_msg!(
                        Warning,
                        SherlockErrorType::DeserializationError,
                        e.to_string()
                    )
                })
        }
    }
}

impl AsyncSizedMessage for OwnedReadHalf {
    #[allow(clippy::manual_async_fn)]
    fn write_sized<'a>(
        &'a mut self,
        _what: SizedMessageObj,
    ) -> impl Future<Output = Result<(), SherlockMessage>> + Send + 'a {
        async move {
            Err(sherlock_msg!(
                Warning,
                SherlockErrorType::SocketError(SocketAction::Write),
                "Cannot write from ReadHalf"
            ))
        }
    }
    #[allow(clippy::manual_async_fn)]
    fn read_sized<'a, R: DeserializeOwned>(
        &'a mut self,
    ) -> impl Future<Output = Result<R, SherlockMessage>> + Send + 'a {
        async move {
            let mut buf_len = [0u8; 4];

            // Read message length
            self.read_exact(&mut buf_len).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Read),
                    e
                )
            })?;
            let msg_len = u32::from_be_bytes(buf_len) as usize;

            let mut buf = vec![0u8; msg_len];
            self.read_exact(&mut buf).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Read),
                    e
                )
            })?;

            let cfg = bincode::config::standard();
            bincode::serde::decode_from_slice::<R, _>(&buf, cfg)
                .map(|(val, _)| val)
                .map_err(|e| {
                    sherlock_msg!(
                        Warning,
                        SherlockErrorType::DeserializationError,
                        e.to_string()
                    )
                })
        }
    }
}

impl AsyncSizedMessage for OwnedWriteHalf {
    #[allow(clippy::manual_async_fn)]
    fn write_sized<'a>(
        &'a mut self,
        what: SizedMessageObj,
    ) -> impl Future<Output = Result<(), SherlockMessage>> + Send + 'a {
        async move {
            // Safely convert buf_len from usize to u32
            let buf_len: u32 = what.bytes().len().try_into().map_err(|_| {
                sherlock_msg!(Warning, SherlockErrorType::InvalidData, "message too long")
            })?;

            // Write message size to stream
            let len_bytes = buf_len.to_be_bytes();
            self.write_all(&len_bytes).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Write),
                    e
                )
            })?;

            // Write message to stream
            self.write(what.bytes()).await.map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::SocketError(SocketAction::Write),
                    e
                )
            })?;

            Ok(())
        }
    }
    #[allow(clippy::manual_async_fn)]
    fn read_sized<'a, R: DeserializeOwned>(
        &'a mut self,
    ) -> impl Future<Output = Result<R, SherlockMessage>> + Send + 'a {
        async move {
            Err(sherlock_msg!(
                Warning,
                SherlockErrorType::SocketError(SocketAction::Read),
                "Cannot read from WriteHalf"
            ))
        }
    }
}
