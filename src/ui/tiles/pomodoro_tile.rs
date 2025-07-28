use crate::{
    actions::commandlaunch::command_launch, daemon::daemon::SizedMessage, g_subclasses::sherlock_row::SherlockRow, launcher::{
        pomodoro_launcher::{Pomodoro, PomodoroStyle},
        Launcher,
    }, prelude::TileHandler, sherlock_error, utils::errors::{SherlockError, SherlockErrorType}
};
use std::os::unix::net::UnixStream;
use std::{
    cell::{Cell, RefCell},
    io::Write,
    path::PathBuf,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
    usize,
};

use super::Tile;

impl Tile {
    pub fn pomodoro(launcher: Rc<Launcher>) -> TimerTile {
        let tile = TimerTile::new();
        let imp = tile.imp();

        if let Some(title) = &launcher.name {
            imp.timer_title.set_text(title);
        }

        tile
    }
}

#[derive(Debug, Default)]
struct PomodoroInterface {
    socket: PathBuf,
    exec: PathBuf,
    update_field: WeakRef<Label>,
    animation: WeakRef<Picture>,
    handle: Option<SourceId>,
    running: Rc<Cell<bool>>,
    frames: Rc<Option<Vec<Pixbuf>>>,
    style: PomodoroStyle,
}
impl PomodoroInterface {
    fn new(pomodoro: &Pomodoro, label: WeakRef<Label>, anim: WeakRef<Picture>) -> Self {
        let instance = Self {
            socket: pomodoro.socket.clone(),
            exec: pomodoro.program.clone(),
            update_field: label,
            animation: anim,
            handle: None,
            running: Rc::new(Cell::new(false)),
            frames: Rc::new(Self::get_animation(pomodoro.style.clone())),
            style: pomodoro.style.clone(),
        };

        match pomodoro.style {
            PomodoroStyle::Minimal => {
                if let Some(label) = instance.update_field.upgrade() {
                    label.set_halign(gtk4::Align::Start);
                    label.set_width_chars(0);
                }
            }
            _ => {}
        }

        match instance.send_message("test") {
            Ok(_) => {}
            Err(e) if matches!(e.error, SherlockErrorType::SocketConnectError(_)) => {
                // start pomodoro service
                let exec = instance.exec.display().to_string();
                if let Err(e) = command_launch(&exec, "") {
                    let _result = e.insert(false);
                }
            }
            Err(e) => {
                let _result = e.insert(false);
            }
        }
        instance
    }
    fn replace_tile(&mut self, remaining: WeakRef<Label>, anim: WeakRef<Picture>) {
        self.animation = anim;
        self.update_field = remaining;
        match self.style {
            PomodoroStyle::Minimal => {
                if let Some(label) = self.update_field.upgrade() {
                    label.set_halign(gtk4::Align::Start);
                    label.set_width_chars(0);
                }
            }
            _ => {}
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
    fn get_animation(style: PomodoroStyle) -> Option<Vec<Pixbuf>> {
        if style == PomodoroStyle::Minimal {
            return None;
        }
        let animation =
            gdk_pixbuf::PixbufAnimation::from_resource("/dev/skxxtz/sherlock/ui/timer.gif").ok()?;
        let mut frames: Vec<Pixbuf> = vec![];
        let mut start_time = SystemTime::now();
        let iter = animation.iter(Some(start_time));
        loop {
            match iter.delay_time() {
                Some(delay) => {
                    start_time = start_time.checked_add(delay).unwrap();
                }
                _ => {
                    break;
                }
            };

            if let Some(buf) = iter.pixbuf().copy() {
                frames.push(buf);
            }

            let advanced = iter.advance(start_time);
            if !advanced {
                break;
            }
        }
        Some(frames)
    }
    fn toggle(&mut self) {
        if self.running.get() {
            self.stop();
        } else {
            self.start();
        }
        self.update_ui();
    }
    fn reset(&mut self) {
        match self.send_message("reset") {
            Ok(_) => {
                if let Some(handle) = self.handle.take() {
                    handle.remove();
                }
                self.running.set(false);
                self.update_ui();
            }
            Err(e) => {
                let _result = e.insert(false);
            }
        }
    }
    fn start(&mut self) {
        if let Err(e) = self.send_message("start") {
            let _result = e.insert(false);
        }
    }
    fn stop(&mut self) {
        if !self.running.get() {
            return;
        }

        if let Err(e) = self.send_message("stop") {
            let _result = e.insert(false);
        } else {
            if let Some(handle) = self.handle.take() {
                handle.remove();
            }
            self.running.set(false);
        }
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
        if let Some(timer) = self.get_timer() {
            if self.running.get() {
                if let Some(handle) = self.handle.take() {
                    handle.remove();
                }
            }
            self.running.set(timer.active);
            let label = self.update_field.clone();
            let animation = self.animation.clone();
            let length = self.frames.as_deref().map_or(0, |f| f.len()) as u64;
            let frames = Rc::clone(&self.frames);
            let mut remaining = timer.remaining().as_secs();
            let current_frame = Rc::new(Cell::new(length - length * remaining / 1500));
            let update_label = move |time: u64| {
                if let Some(label) = label.upgrade() {
                    let mins = time / 60;
                    let secs = time % 60;
                    label.set_text(&format!("{:0>2}:{:0>2}", mins, secs));
                }
            };
            let update_anim = {
                let current_frame = Rc::clone(&current_frame);
                move || {
                    if let Some(image) = animation.upgrade() {
                        let curr = current_frame.get();
                        if let Some(pix) = frames
                            .as_deref()
                            .and_then(|f| f.get(curr.checked_sub(1).unwrap_or(curr) as usize))
                        {
                            let paintable = Texture::for_pixbuf(pix);
                            image.set_paintable(Some(&paintable));
                        }
                    }
                }
            };
            update_label(remaining);
            update_anim();
            if timer.active {
                let handle = glib::timeout_add_local(Duration::new(1, 0), {
                    let is_running = Rc::clone(&self.running);
                    move || {
                        while remaining > 0 {
                            remaining -= 1;
                            let new_frame = length - length * remaining / 1500;
                            if current_frame.get() != new_frame {
                                current_frame.set(new_frame);
                                update_anim();
                            }
                            update_label(remaining);
                            return true.into();
                        }
                        is_running.set(false);
                        false.into()
                    }
                });
                self.handle = Some(handle);
            }
        } else {
            self.update_label(1500);
            self.update_anim(0);
        }
    }
    fn update_label(&self, time: u64) {
        if let Some(label) = self.update_field.upgrade() {
            let mins = time / 60;
            let secs = time % 60;
            label.set_text(&format!("{:0>2}:{:0>2}", mins, secs));
        }
    }
    fn update_anim(&self, frame: usize) {
        if let Some(image) = self.animation.upgrade() {
            if let Some(pix) = self.frames.as_deref().and_then(|f| f.get(frame)) {
                let paintable = Texture::for_pixbuf(pix);
                image.set_paintable(Some(&paintable));
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
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{glib, Picture};
    use gtk4::{Box as GtkBox, Label};

    use crate::g_subclasses::sherlock_row::SherlockRow;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/timer_tile.ui")]
    pub struct TimerTile {
        #[template_child(id = "timer_title")]
        pub timer_title: TemplateChild<Label>,

        #[template_child(id = "remaining_time")]
        pub remaining_label: TemplateChild<Label>,

        #[template_child(id = "animation")]
        pub animation: TemplateChild<Picture>,

        #[template_child(id = "shortcut-holder")]
        pub shortcut_holder: TemplateChild<GtkBox>,

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

use gdk_pixbuf::{
    prelude::PixbufAnimationExtManual, subclass::prelude::ObjectSubclassIsExt, Pixbuf,
};
use gio::glib::{object::{Cast, ObjectExt}, SourceId, WeakRef};
use gtk4::{gdk::Texture, glib, prelude::WidgetExt, Box, Label, Picture, Widget};
use serde::Deserialize;

glib::wrapper! {
    pub struct TimerTile(ObjectSubclass<imp::TimerTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl TimerTile {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
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

#[derive(Debug, Default)]
pub struct PomodoroTileHandler {
    tile: WeakRef<TimerTile>,
    api: Rc<RefCell<PomodoroInterface>>,
}
impl PomodoroTileHandler {
    pub fn new(pomodoro: &Pomodoro) -> Self {
        let pomodoro_api = Rc::new(RefCell::new(PomodoroInterface::new(
            pomodoro,
            WeakRef::new(),
            WeakRef::new(),
        )));
        pomodoro_api.borrow_mut().update_ui();

        Self {
            tile: WeakRef::new(),
            api: pomodoro_api,
        }
    }
    pub fn bind_signal(&self, row: &SherlockRow, pomodoro: &Pomodoro) {
        let style = match pomodoro.style {
            PomodoroStyle::Minimal => "minimal",
            _ => "normal",
        };
        row.set_css_classes(&vec!["tile", "timer-tile", style]);

        let signal_id = row.connect_local("row-should-activate", false, {
            let pomodoro_api = self.api.clone();
            move |args| {
                let _ = args[1]
                    .get::<u8>()
                    .expect("Failed to get u8 from signal args");

                let callback = args[2]
                    .get::<String>()
                    .expect("Failed to get String from signal args");

                match callback.as_str() {
                    "reset" => {
                        pomodoro_api.borrow_mut().reset();
                    }
                    "unset" => return None,
                    _ => {
                        pomodoro_api.borrow_mut().toggle();
                    }
                }
                None
            }
        });
        row.set_signal_id(signal_id);
    }
    pub fn shortcut(&self) -> Option<Box> {
        self.tile.upgrade().map(|t| t.imp().shortcut_holder.get())
    }
}
impl TileHandler for PomodoroTileHandler {
    fn replace_tile(&mut self, tile: &Widget) {
        if let Some(tile) = tile.downcast_ref::<TimerTile>(){
            let imp = tile.imp();
            let remaining = imp.remaining_label.downgrade();
            let anim = imp.animation.downgrade();
            {
                let mut api = self.api.borrow_mut();
                api.replace_tile(remaining, anim);
                api.update_ui();
            }
            self.tile = tile.downgrade()
        } 
    }
}
