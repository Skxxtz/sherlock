use crate::{
    actions::commandlaunch::command_launch,
    daemon::daemon::SizedMessage,
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{pomodoro_launcher::Pomodoro, Launcher},
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};
use std::os::unix::net::UnixStream;
use std::{
    cell::RefCell,
    io::Write,
    path::PathBuf,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::Tile;

impl Tile {
    pub fn pomodoro_tile(launcher: &Launcher, pomodoro: &Pomodoro) -> Vec<SherlockRow> {
        let object = SherlockRow::new();
        let tile = TimerTile::new(object.downgrade());
        let imp = tile.imp();
        object.append(&tile);
        object.set_css_classes(&vec!["tile", "timer-tile"]);
        object.with_launcher(launcher);

        if let Some(title) = &launcher.name {
            imp.timer_title.set_text(title);
        }
        let pomodoro_api = Rc::new(RefCell::new(PomodoroInterface::new(
            pomodoro,
            imp.remaining_label.downgrade(),
        )));
        pomodoro_api.borrow_mut().update_ui();
        // initialize view
        // gather result from pomodoro tile
        let signal_id = object.connect_local("row-should-activate", false, {
            move |_args| {
                pomodoro_api.borrow_mut().toggle();
                None
            }
        });
        object.set_signal_id(signal_id);

        vec![object]
    }
}

struct PomodoroInterface {
    socket: PathBuf,
    exec: PathBuf,
    update_field: WeakRef<Label>,
    handle: Option<SourceId>,
    running: bool,
}
impl PomodoroInterface {
    fn new(pomodoro: &Pomodoro, label: WeakRef<Label>) -> Self {
        Self {
            socket: pomodoro.socket.clone(),
            exec: pomodoro.program.clone(),
            update_field: label,
            handle: None,
            running: false,
        }
    }
    fn send_message(&self, message: &str) -> Result<(), SherlockError> {
        let mut stream = UnixStream::connect(&self.socket).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketConnectError(self.socket.display().to_string()),
                e.to_string()
            )
        })?;
        stream.write_all(message.as_bytes()).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::SocketWriteError(self.socket.display().to_string()),
                e.to_string()
            )
        })?;
        Ok(())
    }
    fn toggle(&mut self) {
        if self.running {
            self.stop();
        } else {
            self.start();
        }
        self.update_ui();
    }
    fn start(&mut self) {
        match self.send_message("start") {
            Ok(_) => {
                self.running = true;
            }
            Err(e) if matches!(e.error, SherlockErrorType::SocketConnectError(_)) => {
                // start pomodoro service
                let exec = self.exec.display().to_string();
                if let Err(e) = command_launch(&exec, "") {
                    let _result = e.insert(false);
                }
                // start timer
                if let Err(e) = self.send_message("start") {
                    let _result = e.insert(false);
                } else {
                    self.running = true;
                }
            }
            Err(e) => {
                let _result = e.insert(false);
            }
        }
    }
    fn stop(&mut self) {
        if let Err(e) = self.send_message("stop") {
            let _result = e.insert(false);
        }
        if let Some(handle) = self.handle.take() {
            handle.remove();
        }
        self.running = false;
    }
    fn get_timer(&self) -> Option<Timer> {
        let mut stream = UnixStream::connect(&self.socket).ok()?;
        stream.write_all(b"show").ok();

        let response = stream.read_sized().ok()?;
        if let Ok(raw) = serde_json::from_slice::<RawTimer>(&response) {
            return Some(Timer::from(raw));
        }
        None
    }
    fn update_ui(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.remove();
        }
        if let Some(timer) = self.get_timer() {
            self.running = timer.active;
            let label = self.update_field.clone();
            let update_label = move |time: u64| {
                if let Some(label) = label.upgrade() {
                    let mins = time / 60;
                    let secs = time % 60;
                    label.set_text(&format!("{:0>2}:{:0>2}", mins, secs));
                }
            };
            let mut remaining = timer.remaining().as_secs();
            update_label(remaining);
            if timer.active {
                let handle = glib::timeout_add_local(Duration::new(1, 0), {
                    move || {
                        while remaining > 0 {
                            remaining -= 1;
                            update_label(remaining);
                            return true.into();
                        }
                        false.into()
                    }
                });
                self.handle = Some(handle);
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct RawTimer {
    end: Option<u64>,
    remaining: Option<u64>,
    active: bool,
}

#[derive(Debug, Clone)]
struct Timer {
    end: Option<SystemTime>,
    remaining: Option<Duration>,
    active: bool,
}
impl Timer {
    fn remaining(&self) -> Duration {
        if self.active {
            self.end
                .and_then(|end| end.duration_since(SystemTime::now()).ok())
                .unwrap_or_default()
        } else {
            self.remaining.unwrap_or_default()
        }
    }
}
impl From<RawTimer> for Timer {
    fn from(value: RawTimer) -> Self {
        let end = value.end.map(|end| UNIX_EPOCH + Duration::from_secs(end));
        let remaining = value.remaining.map(|rem| Duration::from_secs(rem));
        Self {
            end,
            remaining,
            active: value.active,
        }
    }
}

mod imp {
    use std::cell::RefCell;

    use gio::glib::{SignalHandlerId, SourceId, WeakRef};
    use gtk4::glib;
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{Box as GtkBox, Label};

    use crate::g_subclasses::sherlock_row::SherlockRow;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/timer_tile.ui")]
    pub struct TimerTile {
        #[template_child(id = "timer_title")]
        pub timer_title: TemplateChild<Label>,

        #[template_child(id = "remaining_time")]
        pub remaining_label: TemplateChild<Label>,

        pub return_action: RefCell<Option<SignalHandlerId>>,
        pub time_out_handle: RefCell<Option<SourceId>>,
        pub parent: RefCell<WeakRef<SherlockRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimerTile {
        const NAME: &'static str = "TimerTile";
        type Type = super::TimerTile;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TimerTile {}
    impl WidgetImpl for TimerTile {}
    impl BoxImpl for TimerTile {}
}

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::ObjectExt, SourceId, WeakRef};
use gtk4::{
    glib,
    prelude::{BoxExt, WidgetExt},
    Label,
};
use serde::Deserialize;

glib::wrapper! {
    pub struct TimerTile(ObjectSubclass<imp::TimerTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl TimerTile {
    pub fn new(parent: WeakRef<SherlockRow>) -> Self {
        let obj = glib::Object::new::<Self>();
        *obj.imp().parent.borrow_mut() = parent;
        obj
    }
    pub fn clear_timeout(&self) {
        let imp = self.imp();
        if let Some(handle) = imp.time_out_handle.borrow_mut().take() {
            handle.remove();
        }
    }
    pub fn clear_action(&self) {
        let imp = self.imp();
        if let Some(parent) = imp.parent.borrow().upgrade() {
            parent.clear_signal_id();
        }
    }
}
