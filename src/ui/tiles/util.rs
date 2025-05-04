use crate::{g_subclasses::sherlock_row::SherlockRow, loader::pipe_loader::PipeData, CONFIG};
use gio::glib::WeakRef;
use gtk4::{prelude::*, Box, Builder, Image, Label, Overlay, Spinner, TextView};
use std::{collections::HashSet, fmt::Debug};

#[derive(Default)]
pub struct TextViewTileBuilder {
    pub object: Option<WeakRef<Box>>,
    pub content: Option<WeakRef<TextView>>,
}
impl TextViewTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let object = builder.object::<Box>("next_tile").map(|w| w.downgrade());
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
pub struct TileBuilder {
    pub object: SherlockRow,
    pub icon: Option<WeakRef<Image>>,
    pub icon_holder: Option<WeakRef<Box>>,
    pub title: Option<WeakRef<Label>>,
    pub category: Option<WeakRef<Label>>,
    pub tag_start: Option<WeakRef<Label>>,
    pub tag_end: Option<WeakRef<Label>>,
    pub shortcut_holder: Option<WeakRef<Box>>,

    // Specific to 'bulk_text_tile'
    pub content_title: Option<WeakRef<Label>>,
    pub content_body: Option<WeakRef<Label>>,
    // Specific to 'calc_tile'
    pub equation_holder: Option<WeakRef<Label>>,
    pub result_holder: Option<WeakRef<Label>>,
}

impl TileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let holder: Box = builder.object("holder").unwrap_or_default();
        let icon = builder.object::<Image>("icon-name").map(|w| w.downgrade());
        let title = builder.object::<Label>("app-name").map(|w| w.downgrade());
        let category = builder
            .object::<Label>("launcher-type")
            .map(|w| w.downgrade());
        let icon_holder = builder
            .object::<Box>("app-icon-holder")
            .map(|w| w.downgrade());
        let tag_start = builder
            .object::<Label>("app-name-tag-start")
            .map(|w| w.downgrade());
        let tag_end = builder
            .object::<Label>("app-name-tag-end")
            .map(|w| w.downgrade());

        // Append content to the sherlock row
        let object = SherlockRow::new();
        object.append(&holder);
        object.set_css_classes(&vec!["tile"]);

        // Specific to 'bulk_text_tile' and 'error_tile'
        let content_title = builder
            .object::<Label>("content-title")
            .map(|w| w.downgrade());
        let content_body = builder
            .object::<Label>("content-body")
            .map(|w| w.downgrade());

        // Specific to 'calc_tile'
        let equation_holder = builder
            .object::<Label>("equation-holder")
            .map(|w| w.downgrade());
        let result_holder = builder
            .object::<Label>("result-holder")
            .map(|w| w.downgrade());

        let shortcut_holder = builder
            .object::<Box>("shortcut-holder")
            .map(|w| w.downgrade());

        // Set the icon size to the user-specified one
        if let Some(c) = CONFIG.get() {
            icon.as_ref()
                .and_then(|icon| icon.upgrade())
                .map(|icon| icon.set_pixel_size(c.appearance.icon_size));
        }
        drop(builder);
        TileBuilder {
            object,
            icon,
            icon_holder,
            title,
            category,
            tag_start,
            tag_end,
            shortcut_holder,

            content_body,
            content_title,

            equation_holder,
            result_holder,
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
        if let Some(c) = CONFIG.get() {
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

pub trait SherlockSearch {
    fn fuzzy_match<T: AsRef<str> + Debug>(&self, substring: T) -> bool;
}

impl SherlockSearch for String {
    fn fuzzy_match<T>(&self, substring: T) -> bool
    where
        Self: AsRef<str>,
        T: AsRef<str> + Debug,
    {
        let lowercase = substring.as_ref().to_lowercase();
        let char_pattern: HashSet<char> = lowercase.chars().collect();
        let concat_str: String = self
            .to_lowercase()
            .chars()
            .filter(|s| char_pattern.contains(s))
            .collect();
        concat_str.contains(&lowercase)
    }
}
impl SherlockSearch for PipeData {
    fn fuzzy_match<T>(&self, substring: T) -> bool
    where
        T: AsRef<str>,
    {
        // check which value to use
        let search_in = match self.title {
            Some(_) => &self.title,
            None => &self.description,
        };
        if let Some(search_in) = search_in {
            let lowercase = substring.as_ref().to_lowercase();
            let char_pattern: HashSet<char> = lowercase.chars().collect();
            let concat_str: String = search_in
                .to_lowercase()
                .chars()
                .filter(|s| char_pattern.contains(s))
                .collect();
            return concat_str.contains(&lowercase);
        }
        return false;
    }
}

/// Used to update tag_start or tag_end
/// * **label**: The UI label holding the result
/// * **content**: The content for the label, as specified by the user
/// * **keyword**: The current keyword of the search
pub fn update_tag(label: &WeakRef<Label>, content: &Option<String>, keyword: &str) {
    if let Some(content) = &content {
        let content = content.replace("{keyword}", keyword);
        label.upgrade().map(|label| {
            label.set_text(&content);
            label.set_visible(true);
        });
    }
}
