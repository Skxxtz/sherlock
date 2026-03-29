use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ffi::OsStr, path::PathBuf};

use crate::ui::model::file::backends::command::CommandFactory;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FdFactory;

impl CommandFactory for FdFactory {
    fn binary_name(&self) -> &'static str {
        "fd"
    }

    fn args<'a>(&self, paths: &'a [PathBuf]) -> impl Iterator<Item = Cow<'a, OsStr>> {
        const FLAGS: &[&str] = &[".", "--color", "never", "--type", "f", "--type", "d"];
        FLAGS
            .iter()
            .map(|&s| Cow::Borrowed(OsStr::new(s)))
            .chain(paths.iter().flat_map(|p| {
                [
                    Cow::Borrowed(OsStr::new("--search-path")),
                    Cow::Borrowed(p.as_os_str()),
                ]
            }))
    }
}
