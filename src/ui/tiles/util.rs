use crate::{
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{Launcher, ResultItem},
    loader::pipe_loader::PipeData,
    CONFIG,
};
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{self, clone::Downgrade, RustClosure, WeakRef};
use gtk4::{prelude::*, Box, Builder, Image, Label, Overlay, Spinner, TextView};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

#[derive(Debug)]
pub struct AsyncLauncherTile {
    pub launcher: Launcher,
    pub result_item: ResultItem,
    pub text_tile: Option<TextTileElements>,
    pub image_replacement: Option<ImageReplacementElements>,
    pub weather_tile: Option<WeatherTileElements>,
    pub attrs: HashMap<String, String>,
}

#[derive(Debug)]
pub struct TextTileElements {
    pub title: Label,
    pub body: Label,
}
#[derive(Debug)]
pub struct ImageReplacementElements {
    pub _icon: Option<Image>,
    pub icon_holder_overlay: Option<Overlay>,
}
impl ImageReplacementElements {
    pub fn new() -> Self {
        ImageReplacementElements {
            _icon: None,
            icon_holder_overlay: None,
        }
    }
}
#[derive(Debug)]
pub struct WeatherTileElements {
    pub temperature: Label,
    pub location: Label,
    pub icon: Image,
    pub spinner: Spinner,
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
    pub title: Label,
    pub icon: Image,
    pub start_time: Label,
    pub end_time: Label,
    pub shortcut_holder: Option<Box>,
}
impl EventTileBuilder {
    pub fn new(resource: &str) -> Self {
        let builder = Builder::from_resource(resource);
        let holder: Box = builder.object("holder").unwrap_or_default();

        // Append content to the sherlock row
        let object = SherlockRow::new();
        object.set_child(Some(&holder));
        object.set_css_classes(&vec!["tile"]);

        EventTileBuilder {
            object,
            title: builder.object("title-label").unwrap_or_default(),
            start_time: builder.object("time-label").unwrap_or_default(),
            end_time: builder.object("end-time-label").unwrap_or_default(),
            icon: builder.object("icon-name").unwrap_or_default(),
            shortcut_holder: builder.object("shortcut-holder"),
        }
    }
}

#[derive(Clone)]
pub struct TileWidgets {
    pub icon: WeakRef<Image>,
    pub icon_holder: WeakRef<Box>,

    pub title: WeakRef<Label>,
    pub category: WeakRef<Label>,
}

pub struct Tile {
    pub root: Box,
    pub widgets: TileWidgets,
}

impl Tile {
    pub fn from_resource<T>(resource: T) -> Self
    where
        T: AsRef<str>,
    {
        let builder = Builder::from_resource(resource.as_ref());

        let holder: Box = builder.object("holder").unwrap();
        let icon: Image = builder.object("icon-name").unwrap();
        let title: Label = builder.object("app-name").unwrap();
        let category: Label = builder.object("launcher-type").unwrap();
        let icon_holder: Box = builder.object("app-icon-holder").unwrap();

        Self {
            root: holder,

            widgets: TileWidgets {
                icon: Downgrade::downgrade(&icon),
                icon_holder: Downgrade::downgrade(&icon_holder),

                title: Downgrade::downgrade(&title),
                category: Downgrade::downgrade(&category),
            },
        }
    }
}

#[derive(Clone, Default)]
pub struct TileBuilder {
    pub object: SherlockRow,
    pub icon: Image,
    pub icon_holder: Box,
    pub title: Label,
    pub category: Label,
    pub tag_start: Label,
    pub tag_end: Label,
    pub shortcut_holder: Option<Box>,

    // Specific to 'bulk_text_tile'
    pub content_title: Label,
    pub content_body: Label,
    // Specific to 'calc_tile'
    pub equation_holder: Label,
    pub result_holder: Label,
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
        object.set_css_classes(&vec!["tile"]);

        // Specific to 'bulk_text_tile' and 'error_tile'
        let content_title: Label = builder.object("content-title").unwrap_or_default();
        let content_body: Label = builder.object("content-body").unwrap_or_default();

        // Specific to 'calc_tile'
        let equation_holder: Label = builder.object("equation-holder").unwrap_or_default();
        let result_holder: Label = builder.object("result-holder").unwrap_or_default();

        // Set the icon size to the user-specified one
        if let Some(c) = CONFIG.get() {
            icon.set_pixel_size(c.appearance.icon_size);
        }

        TileBuilder {
            object,
            icon,
            icon_holder,
            title,
            category,
            tag_start,
            tag_end,
            shortcut_holder: builder.object("shortcut-holder"),

            content_body,
            content_title,

            equation_holder,
            result_holder,
        }
    }
    pub fn display_tag_start<T>(&self, content: &Option<String>, keyword: T)
    where
        T: AsRef<str>,
    {
        if let Some(start_tag) = content {
            let text = start_tag.replace("{keyword}", keyword.as_ref());
            if !text.is_empty() {
                self.tag_start.set_text(&text);
                self.tag_start.set_visible(true);
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
                self.tag_end.set_text(&text);
                self.tag_end.set_visible(true);
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct WeatherTileBuilder {
    pub object: SherlockRow,
    pub icon: Image,
    pub location: Label,
    pub temperature: Label,
    pub spinner: Spinner,
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
        object.set_css_classes(&vec!["tile"]);

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
            icon,
            location,
            temperature,
            spinner,
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
