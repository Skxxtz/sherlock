use gio::glib::WeakRef;
use gtk4::{Box, Builder, Label, TextView, prelude::*};

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
