use crate::{
    g_subclasses::sherlock_row::SherlockRow, launcher::Launcher, loader::pipe_loader::PipeData,
    CONFIG,
};
use gio::glib::WeakRef;
use gtk4::{prelude::*, Box, Builder, Image, Label, Overlay, Spinner, TextView};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct AsyncLauncherTile {
    pub launcher: Launcher,
    pub row: WeakRef<SherlockRow>,
    pub text_tile: Option<TextTileElements>,
    pub image_replacement: Option<ImageReplacementElements>,
    pub weather_tile: Option<WeatherTileElements>,
    pub attrs: HashMap<String, String>,
}

#[derive(Debug)]
pub struct TextTileElements {
    pub title: WeakRef<Label>,
    pub body: WeakRef<Label>,
}
#[derive(Debug)]
pub struct ImageReplacementElements {
    pub icon_holder_overlay: Option<WeakRef<Overlay>>,
}
impl ImageReplacementElements {
    pub fn new() -> Self {
        ImageReplacementElements {
            icon_holder_overlay: None,
        }
    }
}
#[derive(Debug)]
pub struct WeatherTileElements {
    pub temperature: WeakRef<Label>,
    pub location: WeakRef<Label>,
    pub icon: WeakRef<Image>,
    pub spinner: WeakRef<Spinner>,
}

#[derive(Default)]
pub struct TextViewTileBuilder {
    pub object: Box,
    pub content: TextView,
}
impl TextViewTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        TextViewTileBuilder {
            object: builder.object("next_tile").unwrap_or_default(),
            content: builder.object("content").unwrap_or_default(),
        }
    }
}

#[derive(Default)]
pub struct EventTileBuilder {
    pub object: SherlockRow,
    pub title: WeakRef<Label>,
    pub icon: WeakRef<Image>,
    pub start_time: WeakRef<Label>,
    pub end_time: WeakRef<Label>,
    pub shortcut_holder: Option<WeakRef<Box>>,
}
impl EventTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let holder: Box = builder.object("holder").unwrap_or_default();

        // Append content to the sherlock row
        let object = SherlockRow::new();
        object.set_child(Some(&holder));
        object.set_css_classes(&["tile"]);

        let title: Label = builder.object("title-label").unwrap_or_default();
        let start_time: Label = builder.object("time-label").unwrap_or_default();
        let end_time: Label = builder.object("end-time-label").unwrap_or_default();
        let icon: Image = builder.object("icon-name").unwrap_or_default();
        let shortcut_option: Option<Box> = builder.object("shortcut-holder");
        let shortcut_holder: Option<WeakRef<Box>> =
            shortcut_option.and_then(|h| Some(h.downgrade()));

        EventTileBuilder {
            object,
            title: title.downgrade(),
            start_time: start_time.downgrade(),
            end_time: end_time.downgrade(),
            icon: icon.downgrade(),
            shortcut_holder,
        }
    }
}

#[derive(Clone, Default)]
pub struct TileBuilder {
    pub object: SherlockRow,
    pub icon: WeakRef<Image>,
    pub icon_holder: WeakRef<Box>,
    pub title: WeakRef<Label>,
    pub category: WeakRef<Label>,
    pub tag_start: WeakRef<Label>,
    pub tag_end: WeakRef<Label>,
    pub shortcut_holder: Option<WeakRef<Box>>,

    // Specific to 'bulk_text_tile'
    pub content_title: WeakRef<Label>,
    pub content_body: WeakRef<Label>,
    // Specific to 'calc_tile'
    pub equation_holder: WeakRef<Label>,
    pub result_holder: WeakRef<Label>,
}

impl TileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let holder: Box = builder.object("holder").unwrap_or_default();
        let icon: Image = builder.object("icon-name").unwrap_or_default();
        let title: Label = builder.object("app-name").unwrap_or_default();
        let category: Label = builder.object("launcher-type").unwrap_or_default();
        let icon_holder: Box = builder.object("app-icon-holder").unwrap_or_default();
        let tag_start: Label = builder.object("app-name-tag-start").unwrap_or_default();
        let tag_end: Label = builder.object("app-name-tag-end").unwrap_or_default();

        // Append content to the sherlock row
        let object = SherlockRow::new();
        object.set_child(Some(&holder));
        object.set_css_classes(&["tile"]);

        // Specific to 'bulk_text_tile' and 'error_tile'
        let content_title: Label = builder.object("content-title").unwrap_or_default();
        let content_body: Label = builder.object("content-body").unwrap_or_default();

        // Specific to 'calc_tile'
        let equation_holder: Label = builder.object("equation-holder").unwrap_or_default();
        let result_holder: Label = builder.object("result-holder").unwrap_or_default();

        let shortcut_option: Option<Box> = builder.object("shortcut_holder");
        let shortcut_holder: Option<WeakRef<Box>> =
            shortcut_option.and_then(|s| Some(s.downgrade()));

        // Set the icon size to the user-specified one
        if let Some(c) = CONFIG.get() {
            icon.set_pixel_size(c.appearance.icon_size);
        }
        drop(builder);
        TileBuilder {
            object,
            icon: icon.downgrade(),
            icon_holder: icon_holder.downgrade(),
            title: title.downgrade(),
            category: category.downgrade(),
            tag_start: tag_start.downgrade(),
            tag_end: tag_end.downgrade(),
            shortcut_holder,

            content_body: content_body.downgrade(),
            content_title: content_title.downgrade(),

            equation_holder: equation_holder.downgrade(),
            result_holder: result_holder.downgrade(),
        }
    }
    pub fn display_tag_start<T>(&self, content: &Option<String>, keyword: T)
    where
        T: AsRef<str>,
    {
        if let Some(start_tag) = content {
            let text = start_tag.replace("{keyword}", keyword.as_ref());
            if !text.is_empty() {
                self.tag_start.upgrade().map(|t| {
                    t.set_text(&text);
                    t.set_visible(true);
                });
            }
        }
    }
    pub fn display_tag_end<T>(&self, content: &Option<String>, keyword: T)
    where
        T: AsRef<str>,
    {
        if let Some(start_tag) = content {
            let text = start_tag.replace("{keyword}", keyword.as_ref());
            if !text.is_empty() {
                self.tag_end.upgrade().map(|t| {
                    t.set_text(&text);
                    t.set_visible(true);
                });
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct WeatherTileBuilder {
    pub object: SherlockRow,
    pub icon: WeakRef<Image>,
    pub location: WeakRef<Label>,
    pub temperature: WeakRef<Label>,
    pub spinner: WeakRef<Spinner>,
}

impl WeatherTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let body: Box = builder.object("holder").unwrap_or_default();
        let icon: Image = builder.object("icon-name").unwrap_or_default();
        let location: Label = builder.object("location").unwrap_or_default();
        let temperature: Label = builder.object("temperature").unwrap_or_default();

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

        object.set_child(Some(&overlay));

        // Set the icon size to the user-specified one
        if let Some(c) = CONFIG.get() {
            icon.set_pixel_size(c.appearance.icon_size);
        }

        WeatherTileBuilder {
            object,
            icon: icon.downgrade(),
            location: location.downgrade(),
            temperature: temperature.downgrade(),
            spinner: spinner.downgrade(),
        }
    }
}

pub trait SherlockSearch {
    fn fuzzy_match<T: AsRef<str>>(&self, substring: T) -> bool;
}

impl SherlockSearch for String {
    fn fuzzy_match<T>(&self, substring: T) -> bool
    where
        Self: AsRef<str>,
        T: AsRef<str>,
    {
        let char_pattern: HashSet<char> = substring.as_ref().chars().collect();
        let concat_str: String = self.chars().filter(|s| char_pattern.contains(s)).collect();
        concat_str.contains(substring.as_ref())
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
            let char_pattern: HashSet<char> = substring.as_ref().chars().collect();
            let concat_str: String = search_in
                .chars()
                .filter(|s| char_pattern.contains(s))
                .collect();
            return concat_str.contains(substring.as_ref());
        }
        return false;
    }
}
