use gio::glib::MainContext;
use std::{cell::RefCell, os::unix::net::UnixStream, rc::Rc, thread};

use crate::{
    daemon::daemon::{SherlockDaemon, SizedMessage},
    loader::pipe_loader::PipedData,
    sher_log, sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
    CONFIG, SOCKET_PATH,
};

use super::{api::SherlockAPI, call::ApiCall};

pub struct SherlockServer;
impl SherlockServer {
    pub fn listen(api: Rc<RefCell<SherlockAPI>>) {
        // Create async pipeline
        let (sender, receiver) = async_channel::bounded(1);
        thread::spawn(move || {
            async_std::task::block_on(async {
                let _daemon = SherlockDaemon::new(sender).await;
            });
        });

        // Handle receiving using pipline
        MainContext::default().spawn_local({
            async move {
                while let Ok(msg) = receiver.recv().await {
                    if let Ok(cmd) = serde_json::from_str::<ApiCall>(&msg) {
                        sher_log!(format!("Incoming api request: {}", cmd));
                        api.borrow_mut().await_request(cmd);
                    } else if let Some(mut data) = PipedData::new(&msg) {
                        if let Some(settings) = data.settings.take() {
                            let mut api = api.borrow_mut();
                            settings.into_iter().for_each(|request| {
                                api.await_request(request);
                            });
                        }
                        if let Some(elements) = data.elements.take() {
                            let raw = CONFIG.get().map_or(false, |c| c.runtime.display_raw);
                            let request = if raw {
                                ApiCall::DisplayRaw(elements)
                            } else {
                                ApiCall::Pipe(elements)
                            };
                            api.borrow_mut().await_request(request);
                        }
                    } else {
                        sher_log!(format!("Failed to deserialize api call(s): {}", msg));
                    }
                    api.borrow_mut().flush();
                }
            }
        });
    }
    pub fn _send<T: AsRef<[u8]>>(message: T) -> Result<(), SherlockError> {
        let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketConnectError(SOCKET_PATH.to_string()),
                e.to_string()
            )
        })?;
        stream.write_sized(message.as_ref())?;
        Ok(())
    }
    pub fn send_action(api_call: ApiCall) -> Result<(), SherlockError> {
        let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketConnectError(SOCKET_PATH.to_string()),
                e.to_string()
            )
        })?;
        let msg = simd_json::to_string(&api_call)
            .map_err(|e| sherlock_error!(SherlockErrorType::SerializationError, e.to_string()))?;
        stream.write_sized(msg.as_bytes())?;
        Ok(())
    }
}
