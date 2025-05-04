use std::collections::HashMap;
use std::pin::Pin;

use gio::glib::object::ObjectExt;
use gio::glib::Bytes;
use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{gdk, Image, Overlay};

use super::util::TileBuilder;
use super::Tile;
use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::audio_launcher::MusicPlayerLauncher;
use crate::launcher::Launcher;

impl Tile {
    pub fn mpris_tile(launcher: &Launcher, mpris: &MusicPlayerLauncher) -> Vec<SherlockRow> {
        let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/mpris_tile.ui");
        builder.object.add_css_class("mpris-tile");
        builder.object.set_overflow(gtk4::Overflow::Hidden);
        builder.object.with_launcher(&launcher);

        builder
            .category
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|category| category.set_text(&mpris.mpris.metadata.artists.join(", ")));
        builder
            .title
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|title| title.set_text(&mpris.mpris.metadata.title));

        let overlay = Overlay::new();

        builder
            .icon
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|icon| icon.set_visible(false));

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
        builder
            .icon_holder
            .as_ref()
            .and_then(|tmp| tmp.upgrade())
            .map(|holder| {
                holder.append(&overlay);
                holder.set_overflow(gtk4::Overflow::Hidden);
                holder.set_widget_name("mpris-icon-holder");
                holder.set_margin_top(10);
                holder.set_margin_bottom(10);
            });

        // Add attrs and implement double click capabilities
        let attrs: HashMap<String, String> =
            vec![("method", &launcher.method), ("player", &mpris.player)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

        // Make shortcut holder
        if launcher.shortcut {
            builder.object.set_shortcut_holder(builder.shortcut_holder);
        }
        let overlay = overlay.downgrade();
        let overlay_clone = overlay.clone();
        let launcher = launcher.clone();

        let async_update_closure: Box<dyn Fn(&str) -> Pin<Box<dyn futures::Future<Output = ()>>>> =
            Box::new(move |_keyword: &str| {
                let icon_overlay = overlay_clone.clone();
                let launcher = launcher.clone();
                Box::pin(async move {
                    if let Some((image, was_cached)) = launcher.get_image().await {
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
            });

        // attatch signal
        builder.object.set_async_update(async_update_closure);
        let signal_id = builder
            .object
            .connect_local("row-should-activate", false, move |row| {
                let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                execute_from_attrs(&row, &attrs);
                None
            });
        builder.object.set_signal_id(signal_id);

        // return
        vec![builder.object]
    }
}
