use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ffi::OsStr, path::PathBuf};

use crate::ui::model::file::backends::command::CommandFactory;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RgFactory;

impl CommandFactory for RgFactory {
    fn binary_name(&self) -> &'static str {
        "rg"
    }
    fn args<'a>(&self, paths: &'a [PathBuf]) -> impl Iterator<Item = Cow<'a, OsStr>> {
        const FLAGS: &[&str] = &["--files"];
        FLAGS
            .iter()
            .map(|&s| Cow::Borrowed(OsStr::new(s)))
            .chain(paths.iter().map(|p| Cow::Borrowed(p.as_os_str())))
    }
}
