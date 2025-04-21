use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gtk4::glib;

use crate::loader::util::AppData;

mod imp;

glib::wrapper! {
    pub struct AppEntryObject(ObjectSubclass<imp::AppEntryObject>);
}

impl AppEntryObject {
    pub fn new(app_name: &str, mode: &str, launcher_name: &str, app_data: AppData) -> Self {
        let app_entry: Self = glib::Object::builder()
            .property("name", app_name)
            .property("mode", mode)
            .property("launcher_name", launcher_name)
            .property("priority", app_data.priority)
            .build();

        app_entry.set_app_data(app_data);

        app_entry
    }

    pub fn app_data(&self) -> &AppData {
        self.imp().data.get().unwrap()
    }

    pub fn set_app_data(&self, app_data: AppData) {
        self.imp().data.set(app_data).unwrap()
    }
}
