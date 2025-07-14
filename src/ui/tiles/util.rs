use crate::g_subclasses::sherlock_row::SherlockRow;
use gio::glib::WeakRef;
use gtk4::{prelude::*, Box, Builder, Image, Label, TextView};

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
