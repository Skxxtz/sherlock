use std::cell::{Cell, RefCell};

use gio::glib::Properties;
use gtk4::{
    glib::{
        self as glib, derived_properties, object_subclass,
        subclass::{object::ObjectImpl, types::ObjectSubclass},
    },
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::OnceCell;

use crate::loader::util::AppData;

#[derive(Default, Properties)]
#[properties(wrapper_type = super::AppEntryObject)]
pub struct AppEntryObject {
    #[property(get, set)]
    name: RefCell<String>,

    #[property(get, set)]
    mode: RefCell<String>,

    #[property(get, set)]
    priority: Cell<f32>,

    #[property(get, set)]
    launcher_name: RefCell<String>,

    pub data: OnceCell<AppData>,
}

#[object_subclass]
impl ObjectSubclass for AppEntryObject {
    const NAME: &'static str = "AppEntryObject";
    type Type = super::AppEntryObject;
}

#[derived_properties]
impl ObjectImpl for AppEntryObject {}
