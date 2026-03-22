use std::collections::HashMap;
use std::os::fd::{AsRawFd, FromRawFd};
use std::sync::{OnceLock, RwLock};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    event_created_child,
    protocol::{
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::{self, ZwlrDataControlManagerV1},
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
};

pub static CLIPBOARD: OnceLock<RwLock<Option<String>>> = OnceLock::new();

pub fn get_clipboard() -> Option<String> {
    CLIPBOARD.get()?.read().ok()?.clone()
}

struct ClipboardState {
    seat: Option<WlSeat>,
    manager: Option<ZwlrDataControlManagerV1>,
    device: Option<ZwlrDataControlDeviceV1>,
    offer_mime_types: HashMap<ObjectId, Vec<String>>,
}

impl ClipboardState {
    fn new() -> Self {
        Self {
            seat: None,
            manager: None,
            device: None,
            offer_mime_types: HashMap::new(),
        }
    }
}

impl Dispatch<WlRegistry, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_seat" => {
                    state.seat = Some(registry.bind(name, version.min(7), qh, ()));
                }
                "zwlr_data_control_manager_v1" => {
                    state.manager = Some(registry.bind(name, version.min(2), qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlSeat, ()> for ClipboardState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlManagerV1, ()> for ClipboardState {
    fn event(
        _: &mut Self,
        _: &ZwlrDataControlManagerV1,
        _: zwlr_data_control_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        _: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _: &(),
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                state.offer_mime_types.insert(id.id(), Vec::new());
            }
            zwlr_data_control_device_v1::Event::Selection { id } => {
                let Some(offer) = id else {
                    if let Some(lock) = CLIPBOARD.get() {
                        if let Ok(mut c) = lock.write() {
                            *c = None;
                        }
                    }
                    return;
                };

                let mime_types = state
                    .offer_mime_types
                    .get(&offer.id())
                    .cloned()
                    .unwrap_or_default();

                let mime = mime_types
                    .iter()
                    .find(|m| {
                        let m = m.as_str();
                        m == "text/plain;charset=utf-8"
                            || m == "UTF8_STRING"
                            || m == "text/plain"
                            || m.starts_with("text/")
                    })
                    .cloned();

                if let Some(mime_type) = mime {
                    let (read_fd, write_fd) = nix::unistd::pipe().unwrap();
                    offer.receive(mime_type, unsafe {
                        std::os::fd::BorrowedFd::borrow_raw(write_fd.as_raw_fd())
                    });
                    drop(write_fd);
                    conn.flush().ok();

                    // read in background so we don't block the event loop
                    let mut bytes = Vec::new();
                    use std::io::Read;
                    use std::os::fd::IntoRawFd;
                    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd.into_raw_fd()) };
                    file.read_to_end(&mut bytes).ok();
                    if let Ok(text) = String::from_utf8(bytes) {
                        let text = text.replace("\r\n", "\n");
                        if let Some(lock) = CLIPBOARD.get() {
                            if let Ok(mut c) = lock.write() {
                                *c = Some(text);
                            }
                        }
                    }
                }

                state.offer_mime_types.retain(|k, _| *k == offer.id());
            }
            _ => {}
        }
    }

    event_created_child!(ClipboardState, ZwlrDataControlDeviceV1, [
        zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ())
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        offer: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            state
                .offer_mime_types
                .entry(offer.id())
                .or_default()
                .push(mime_type);
        }
    }
}

pub fn spawn_clipboard_watcher() {
    CLIPBOARD.get_or_init(|| RwLock::new(None));
    let conn = Connection::connect_to_env().expect("failed to connect to Wayland");
    let mut event_queue = conn.new_event_queue::<ClipboardState>();
    let qh = event_queue.handle();
    conn.display().get_registry(&qh, ());
    let mut state = ClipboardState::new();

    event_queue.roundtrip(&mut state).unwrap();

    // wire up data device
    if let (Some(manager), Some(seat)) = (&state.manager, &state.seat) {
        state.device = Some(manager.get_data_device(seat, &qh, ()));
    } else {
        eprintln!("zwlr_data_control_manager_v1 not supported by compositor");
        return;
    }
    event_queue.blocking_dispatch(&mut state).ok();

    std::thread::spawn(move || {
        loop {
            event_queue.blocking_dispatch(&mut state).ok();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_watcher() {
        spawn_clipboard_watcher();
        let content = get_clipboard();
        println!("Current clipboard: {:?}", content);
        assert!(
            CLIPBOARD.get().is_some(),
            "CLIPBOARD static was not initialized"
        );
    }
}
