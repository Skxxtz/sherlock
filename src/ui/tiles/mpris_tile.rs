use gio::glib::Bytes;
use gtk4::prelude::{BoxExt, WidgetExt};
use gtk4::{gdk, Box, Image, Label, Overlay};

use super::util::{AsyncOptions, TileBuilder};
use super::Tile;
use crate::launcher::audio_launcher::MusicPlayerLauncher;
use crate::launcher::{Launcher, ResultItem};

impl Tile {
    pub fn mpris_tile(
        launcher: &Launcher,
        mpris: &MusicPlayerLauncher,
    ) -> Option<(
        ResultItem,
        Option<Label>,
        Option<Label>,
        Option<AsyncOptions>,
        Box,
    )> {
        let builder = TileBuilder::new("/dev/skxxtz/sherlock/ui/mpris_tile.ui");
        builder.object.add_css_class("mpris-tile");
        builder.object.set_spawn_focus(launcher.spawn_focus);
        builder.object.set_shortcut(launcher.shortcut);

        builder
            .category
            .set_text(&mpris.mpris.metadata.artists.join(", "));
        builder.title.set_text(&mpris.mpris.metadata.title);
        builder.object.set_overflow(gtk4::Overflow::Hidden);

        let overlay = Overlay::new();
        let mut options = AsyncOptions::new();

        builder.icon.set_visible(false);

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
            let image = Image::from_pixbuf(Some(&image_buf));
            overlay.set_child(Some(&image));
            image.set_widget_name("placeholder-icon");
            image.set_pixel_size(50);
        };
        builder.icon_holder.append(&overlay);

        builder.icon_holder.set_overflow(gtk4::Overflow::Hidden);
        builder.icon_holder.set_widget_name("mpris-icon-holder");
        builder.icon_holder.set_margin_top(10);
        builder.icon_holder.set_margin_bottom(10);

        let attrs: Vec<(&str, &str)> = vec![("player", &mpris.player)];
        builder.add_default_attrs(Some(&launcher.method), None, None, None, Some(attrs));

        let shortcut_holder = match launcher.shortcut {
            true => builder.shortcut_holder,
            _ => None,
        };
        let res = ResultItem {
            priority: launcher.priority as f32,
            row_item: builder.object,
            shortcut_holder,
        };

        options.icon_holder_overlay = Some(overlay);

        return Some((res, None, None, Some(options), builder.attrs));
    }
}
