use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::object::ObjectExt;
use gio::glib::variant::ToVariant;
use gio::glib::Bytes;
use gio::prelude::ListModelExt;
use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{gdk, Image, Overlay};

use super::Tile;
use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::audio_launcher::MusicPlayerLauncher;
use crate::launcher::Launcher;
use crate::ui::tiles::app_tile::AppTile;

impl Tile {
    pub fn mpris_tile(launcher: &Launcher, mpris: &MusicPlayerLauncher) -> Vec<SherlockRow> {
        let tile = AppTile::new();
        let imp = tile.imp();
        let object = SherlockRow::new();
        object.append(&tile);

        object.add_css_class("mpris-tile");
        object.set_overflow(gtk4::Overflow::Hidden);
        object.with_launcher(&launcher);

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

        // Add attrs and implement double click capabilities
        let attrs: HashMap<String, String> = vec![
            ("method", &launcher.method),
            ("player", &mpris.player),
            ("exit", &launcher.exit.to_string()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        // Make shortcut holder
        if launcher.shortcut {
            object.set_shortcut_holder(Some(imp.shortcut_holder.downgrade()));
        }

        let mpris_rc = Rc::new(RefCell::new(mpris.clone()));
        let async_update_closure: Box<dyn Fn(&str) -> Pin<Box<dyn futures::Future<Output = ()>>>> =
            Box::new({
                let overlay = overlay.downgrade();
                let row_weak = object.downgrade();
                let category = imp.category.downgrade();
                let title = imp.title.downgrade();
                let mpris = Rc::clone(&mpris_rc);
                move |_keyword: &str| {
                    let mpris = Rc::clone(&mpris);
                    let icon_overlay = overlay.clone();
                    let row = row_weak.clone();
                    let category = category.clone();
                    let title = title.clone();
                    Box::pin(async move {
                        let overlay = match icon_overlay.upgrade() {
                            Some(overlay) => overlay,
                            None => return,
                        };
                        {
                            // check if new song is playing here
                            let mut mpris = mpris.borrow_mut();
                            if let Some((new, changed)) = mpris.update() {
                                if !changed && overlay.observe_children().n_items() == 2 {
                                    //early return if it didnt change
                                    return;
                                }
                                // Update mpris and ui title and artist
                                *mpris = new;
                                category.upgrade().map(|category| {
                                    category.set_text(&mpris.mpris.metadata.artists.join(", "))
                                });
                                title
                                    .upgrade()
                                    .map(|title| title.set_text(&mpris.mpris.metadata.title));
                            } else {
                                // hide tile if nothing is playing
                                row.upgrade().map(|row| row.set_visible(false));
                                return;
                            }
                        }
                        row.upgrade().map(|row| row.set_visible(true));
                        if let Some((image, was_cached)) = mpris.borrow().get_image().await {
                            if !was_cached {
                                if let Some(overlay) = icon_overlay.upgrade() {
                                    overlay.add_css_class("image-replace-overlay");
                                }
                            }
                            let texture = gtk4::gdk::Texture::for_pixbuf(&image);
                            let gtk_image = gtk4::Image::from_paintable(Some(&texture));
                            gtk_image.set_widget_name("album-cover");
                            gtk_image.set_pixel_size(50);
                            if let Some(overlay) = icon_overlay.upgrade() {
                                overlay.add_overlay(&gtk_image);
                            }
                        }
                    })
                }
            });

        // attatch signal
        object.set_async_update(async_update_closure);
        let signal_id = object.connect_local("row-should-activate", false, move |args| {
            let row = args.first().map(|f| f.get::<SherlockRow>().ok())??;
            let exit: u8 = args.get(1).and_then(|v| v.get::<u8>().ok())?;
            let mut callback: String = args.get(2).and_then(|v| v.get::<String>().ok())?;
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
                "playpause" => {
                    let player = &mpris_rc.borrow().player;
                    if let Err(error) = MusicPlayerLauncher::playpause(player) {
                        let _result = error.insert(false);
                    }
                }
                _ => {
                    println!("{:?}", callback);
                    execute_from_attrs(&row, &attrs, exit);
                }
            }
            // To reload ui according to mode
            let _ = row.activate_action("win.update-items", Some(&false.to_variant()));
            None
        });
        object.set_signal_id(signal_id);

        // return
        vec![object]
    }
}
