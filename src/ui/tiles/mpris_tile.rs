use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gio::glib::object::{Cast, ObjectExt};
use gio::glib::variant::ToVariant;
use gio::glib::{Bytes, WeakRef};
use gio::prelude::ListModelExt;
use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{gdk, Box, Image, Overlay, Widget};

use super::Tile;
use crate::actions::{execute_from_attrs, get_attrs_map};
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::audio_launcher::MusicPlayerLauncher;
use crate::launcher::Launcher;
use crate::prelude::TileHandler;
use crate::ui::g_templates::AppTile;

impl Tile {
    pub fn mpris_tile() -> AppTile {
        let tile = AppTile::new();
        let imp = tile.imp();

        // object.set_overflow(gtk4::Overflow::Hidden);

        let overlay = Overlay::new();

        imp.icon.set_visible(false);
        let pix_buf = vec![0, 0, 0];
        let image_buf = gdk::gdk_pixbuf::Pixbuf::from_bytes(
            &Bytes::from_owned(pix_buf),
            gdk::gdk_pixbuf::Colorspace::Rgb,
            false,
            8,
            1,
            1,
            3,
        );
        if let Some(image_buf) =
            image_buf.scale_simple(30, 30, gdk::gdk_pixbuf::InterpType::Nearest)
        {
            let texture = gtk4::gdk::Texture::for_pixbuf(&image_buf);
            let image = Image::from_paintable(Some(&texture));
            overlay.set_child(Some(&image));
            image.set_widget_name("placeholder-icon");
            image.set_pixel_size(50);
        };

        let holder = &imp.icon_holder;
        holder.append(&overlay);
        holder.set_overflow(gtk4::Overflow::Hidden);
        holder.set_widget_name("mpris-icon-holder");
        holder.set_margin_top(10);
        holder.set_margin_bottom(10);

        // return
        tile
    }
}

#[derive(Debug, Default)]
pub struct MusicTileHandler {
    tile: WeakRef<AppTile>,
    attrs: Rc<RefCell<HashMap<String, String>>>,
    mpris: Rc<RefCell<MusicPlayerLauncher>>,
}
impl MusicTileHandler {
    pub fn new(mpris: &MusicPlayerLauncher, launcher: Rc<Launcher>) -> Self {
        let attrs = get_attrs_map(vec![
            ("method", Some(&launcher.method)),
            ("exit", Some(&launcher.exit.to_string())),
            ("player", Some(&mpris.player)),
        ]);
        Self {
            tile: WeakRef::new(),
            attrs: Rc::new(RefCell::new(attrs)),
            mpris: Rc::new(RefCell::new(mpris.clone())),
        }
    }
    pub async fn update_async(&self, row: &SherlockRow) -> Option<()> {
        let tile = self.tile.upgrade()?;
        let imp = tile.imp();
        let first_child = imp.icon_holder.last_child()?;
        let icon_overlay = first_child.downcast_ref::<Overlay>()?;
        {
            // check if new song is playing here
            let mut mpris = self.mpris.borrow_mut();
            if let Some((new, changed)) = mpris.update() {
                if !changed && icon_overlay.observe_children().n_items() == 2 {
                    //early return if it didnt change
                    return None;
                }
                // Update mpris and ui title and artist
                *mpris = new;
                let artists_text = mpris
                    .mpris
                    .metadata
                    .artists
                    .as_ref()
                    .map(|artists| artists.join(", "))
                    .unwrap_or_else(|| "Unknown Artist".to_string());
                imp.category.set_text(&artists_text);

                let title_text = mpris
                    .mpris
                    .metadata
                    .title
                    .as_ref()
                    .map(|title| title.clone())
                    .unwrap_or_else(|| "Unknown Title".to_string());
                imp.title.set_text(&title_text);
                self.attrs
                    .borrow_mut()
                    .entry("player".to_string())
                    .or_insert(mpris.player.clone());
            } else {
                // hide tile if nothing is playing
                row.set_visible(false);
                return None;
            }
        }

        // Set image
        row.set_visible(true);
        if let Some((image, was_cached)) = self.mpris.borrow().get_image().await {
            if !was_cached {
                icon_overlay.add_css_class("image-replace-overlay");
            }
            let texture = gtk4::gdk::Texture::for_pixbuf(&image);
            let gtk_image = gtk4::Image::from_paintable(Some(&texture));
            gtk_image.set_widget_name("album-cover");
            gtk_image.set_pixel_size(50);
            icon_overlay.add_overlay(&gtk_image);
        }
        Some(())
    }
    pub fn change_attrs(&self, key: String, value: String) {
        self.attrs.borrow_mut().insert(key, value);
    }
    pub fn bind_signal(
        &self,
        row: &SherlockRow,
        mpris: &MusicPlayerLauncher,
        launcher: Rc<Launcher>,
    ) -> Option<()> {
        row.add_css_class("mpris-tile");
        let attrs = self.attrs.clone();
        let mpris_rc = Rc::new(RefCell::new(mpris.clone()));
        let signal_id = row.connect_local("row-should-activate", false, move |args| {
            let row = args.first().map(|f| f.get::<SherlockRow>().ok())??;
            let exit: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
            let mut callback: String = args.get(2).and_then(|v| v.get::<String>().ok())?;
            if callback.is_empty() {
                callback = attrs.borrow().get("method")?.to_string();
            }

            callback.make_ascii_lowercase();

            let exit: Option<bool> = match exit {
                1 => Some(false),
                2 => Some(true),
                _ => None,
            };
            match callback.as_str() {
                "next" => {
                    let player = &mpris_rc.borrow().player;
                    if let Err(error) = MusicPlayerLauncher::next(player) {
                        let _result = error.insert(false);
                    }
                }
                "previous" => {
                    let player = &mpris_rc.borrow().player;
                    if let Err(error) = MusicPlayerLauncher::previous(player) {
                        let _result = error.insert(false);
                    }
                }
                "playpause" | "audio_sink" => {
                    let player = &mpris_rc.borrow().player;
                    if let Err(error) = MusicPlayerLauncher::playpause(player) {
                        let _result = error.insert(false);
                    }
                }
                "unbind" => return None,
                _ => {
                    execute_from_attrs(&row, &attrs.borrow(), exit, Some(launcher.clone()));
                }
            }
            // To reload ui according to mode
            let _ = row.activate_action("win.update-items", Some(&false.to_variant()));
            None
        });
        row.set_signal_id(signal_id);
        Some(())
    }
    pub fn shortcut(&self) -> Option<Box> {
        self.tile.upgrade().map(|t| t.imp().shortcut_holder.get())
    }
}
impl TileHandler for MusicTileHandler {
    fn replace_tile(&mut self, tile: &Widget) {
        if let Some(tile) = tile.downcast_ref::<AppTile>() {
            self.tile = tile.downgrade()
        }
    }
}
