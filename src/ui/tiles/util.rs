use crate::{g_subclasses::sherlock_row::SherlockRow, utils::config::ConfigGuard};
use gio::glib::WeakRef;
use gtk4::{prelude::*, Box, Builder, Image, Label, Overlay, Spinner, TextView};

#[derive(Default)]
pub struct TextViewTileBuilder {
    pub object: Option<Box>,
    pub content: Option<WeakRef<TextView>>,
}
impl TextViewTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let object = builder.object::<Box>("next_tile");
        let content = builder.object::<TextView>("content").map(|w| w.downgrade());
        TextViewTileBuilder { object, content }
    }
}

#[derive(Default)]
pub struct EventTileBuilder {
    pub object: SherlockRow,
    pub title: Option<WeakRef<Label>>,
    pub icon: Option<WeakRef<Image>>,
    pub start_time: Option<WeakRef<Label>>,
    pub end_time: Option<WeakRef<Label>>,
    pub shortcut_holder: Option<WeakRef<Box>>,
}
impl EventTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let holder: Box = builder.object("holder").unwrap_or_default();

        // Append content to the sherlock row
        let object = SherlockRow::new();
        object.append(&holder);
        object.set_css_classes(&vec!["tile"]);

        let title = builder
            .object::<Label>("title-label")
            .map(|w| w.downgrade());
        let start_time = builder.object::<Label>("time-label").map(|w| w.downgrade());
        let end_time = builder
            .object::<Label>("end-time-label")
            .map(|w| w.downgrade());
        let icon = builder.object::<Image>("icon-name").map(|w| w.downgrade());
        let shortcut_holder = builder
            .object::<Box>("shortcut_holder")
            .map(|w| w.downgrade());

        EventTileBuilder {
            object,
            title,
            start_time,
            end_time,
            icon,
            shortcut_holder,
        }
    }
}

#[derive(Clone, Default)]
pub struct WeatherTileBuilder {
    pub object: SherlockRow,
    pub icon: Option<WeakRef<Image>>,
    pub location: Option<WeakRef<Label>>,
    pub temperature: Option<WeakRef<Label>>,
    pub spinner: WeakRef<Spinner>,
}

impl WeatherTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let body = builder.object::<Box>("holder").unwrap_or_default();
        let icon = builder.object::<Image>("icon-name").map(|w| w.downgrade());
        let location = builder.object::<Label>("location").map(|w| w.downgrade());
        let temperature = builder
            .object::<Label>("temperature")
            .map(|w| w.downgrade());

        // Append content to the sherlock row
        let object = SherlockRow::new();
        object.set_css_classes(&["tile"]);

        let overlay = Overlay::new();
        overlay.set_child(Some(&body));

        let spinner = Spinner::new();
        spinner.set_spinning(true);
        spinner.set_size_request(20, 20);
        spinner.set_halign(gtk4::Align::Center);
        spinner.set_valign(gtk4::Align::Center);
        overlay.add_overlay(&spinner);

        object.append(&overlay);

        // Set the icon size to the user-specified one
        if let Ok(c) = ConfigGuard::read() {
            icon.as_ref()
                .and_then(|tmp| tmp.upgrade())
                .map(|icon| icon.set_pixel_size(c.appearance.icon_size));
        }

        WeatherTileBuilder {
            object,
            icon,
            location,
            temperature,
            spinner: spinner.downgrade(),
        }
    }
}

/// Used to update tag_start or tag_end
/// * **label**: The UI label holding the result
/// * **content**: The content for the label, as specified by the user
/// * **keyword**: The current keyword of the search
pub fn update_tag(label: &Label, content: &Option<String>, keyword: &str) -> Option<()> {
    if let Some(content) = &content {
        let content = content.replace("{keyword}", keyword);
        if keyword.is_empty() {
            label.set_visible(false);
            return None;
        }
        label.set_text(&content);
        label.set_visible(true);
    }
    None
}
