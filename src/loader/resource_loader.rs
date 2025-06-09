use gio::glib;

use super::Loader;
use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
};

impl Loader {
    #[sherlock_macro::timing("Loading resources")]
    pub fn load_resources() -> Result<(), SherlockError> {
        let res_bytes = include_bytes!("../../resources.gresources");
        let resource = gio::Resource::from_data(&glib::Bytes::from_static(res_bytes))
            .map_err(|e| sherlock_error!(SherlockErrorType::ResourceParseError, e.to_string()))?;
        gio::resources_register(&resource);
        Ok(())
    }
}
