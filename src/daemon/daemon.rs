use std::fs::remove_file;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

use crate::api::api::RESPONSE_SOCKET;
use crate::api::call::ApiCall;
use crate::loader::Loader;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::{sher_log, sherlock_error, SOCKET_DIR, SOCKET_PATH};

pub struct SherlockDaemon {
    socket: String,
}
impl SherlockDaemon {
    pub async fn new(pipeline: async_channel::Sender<String>) -> Self {
        let _ = std::fs::remove_file(SOCKET_PATH);
        let listener = UnixListener::bind(SOCKET_PATH).expect("Failed to bind socket");
        let _ = sher_log!(format!("Daemon listening on {}", SOCKET_PATH));

        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                loop {
                    match stream.read_sized() {
                        Ok(buf) if !buf.is_empty() => {
                            let received_data = String::from_utf8_lossy(&buf);
                            let received_data = received_data.trim();
                            let _ = pipeline.send(received_data.to_string()).await;
                        }
                        Ok(_) | Err(_) => break,
                    }
                }
            }
        }
        Self {
            socket: SOCKET_PATH.to_string(),
        }
    }
    fn remove(&self) -> Result<(), SherlockError> {
        std::fs::remove_file(&self.socket).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketRemoveError(self.socket.clone()),
                e.to_string()
            )
        })?;
        Ok(())
    }
    pub fn instance() -> Result<(), SherlockError> {
        let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketConnectError(SOCKET_PATH.to_string()),
                e.to_string()
            )
        })?;
        // Send pipe request
        let pipe = Loader::load_pipe_args();
        if pipe.is_empty() {
            stream.write_sized(br#""Show""#)?;
        } else {
            // Send return pipe request
            let addr = format!("{}sherlock-pipe.socket", SOCKET_DIR);

            // remove existing socket
            let _ = remove_file(&addr);

            // create new socket
            let listener = UnixListener::bind(&addr).expect("Failed to bind socket for responses.");

            // tell sherlock to use this socket
            let request = ApiCall::Socket(Some(addr));
            let request_json = simd_json::to_string(&request).map_err(|e| {
                sherlock_error!(SherlockErrorType::SerializationError, e.to_string())
            })?;
            stream.write_sized(request_json.as_bytes())?;

            // Send piped content and show
            stream.write_sized(&pipe)?;
            stream.write_sized(br#""Show""#)?;

            // Close so it wont block main
            drop(stream);

            // Await response
            'server_loop: for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    loop {
                        match stream.read_sized() {
                            Ok(buf) if !buf.is_empty() => {
                                let received_data = String::from_utf8_lossy(&buf);
                                let received_data = received_data.trim();
                                if received_data == "EXIT" {
                                    break 'server_loop;
                                }
                                println!("{}", received_data);
                            }
                            Ok(_) | Err(_) => break,
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for SherlockDaemon {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

pub trait SizedMessage {
    fn write_sized(&mut self, buf: &[u8]) -> Result<(), SherlockError>;
    fn read_sized(&mut self) -> Result<Vec<u8>, SherlockError>;
}
impl SizedMessage for UnixStream {
    fn write_sized(&mut self, buf: &[u8]) -> Result<(), SherlockError> {
        let buf_len = buf.len();
        if buf_len > u32::MAX as usize {
            // return error for size too big
        }
        let buf_len = buf_len as u32;
        self.write_all(&buf_len.to_be_bytes()).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketWriteError(SOCKET_PATH.to_string()),
                e.to_string()
            )
        })?;
        self.write_all(buf).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketWriteError(SOCKET_PATH.to_string()),
                e.to_string()
            )
        })?;

        Ok(())
    }
    fn read_sized(&mut self) -> Result<Vec<u8>, SherlockError> {
        let mut buf_len = [0u8; 4];
        self.read_exact(&mut buf_len).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketWriteError(SOCKET_PATH.to_string()),
                e.to_string()
            )
        })?;
        let msg_len = u32::from_be_bytes(buf_len) as usize;

        let mut buf = vec![0u8; msg_len];
        self.read_exact(&mut buf).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketWriteError(SOCKET_PATH.to_string()),
                e.to_string()
            )
        })?;

        Ok(buf)
    }
}

pub fn print_reponse<T: AsRef<[u8]>>(response: T) -> Result<(), SherlockError> {
    let guard = RESPONSE_SOCKET.read().unwrap();
    let response = response.as_ref();
    if let Some(addr) = guard.as_ref() {
        let mut stream = UnixStream::connect(addr).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketConnectError(addr.to_string()),
                e.to_string()
            )
        })?;
        stream.write_sized(response)?;
    } else {
        let response = String::from_utf8_lossy(response);
        println!("{}", response);
    }
    Ok(())
}
pub fn close_response() -> Result<(), SherlockError> {
    {
        let guard = RESPONSE_SOCKET.read().unwrap();
        if let Some(addr) = guard.as_ref() {
            let mut stream = UnixStream::connect(addr).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::SocketConnectError(addr.to_string()),
                    e.to_string()
                )
            })?;
            stream.write_sized(b"EXIT")?;
        }
    }
    {
        let mut guard = RESPONSE_SOCKET.write().unwrap();
        *guard = None;
    }

    Ok(())
}
