pub mod application_loader;
pub mod assets;
mod flag_loader;
mod icon;
mod launcher_loader;
pub mod utils;

pub struct Loader;
pub use icon::{CustomIconTheme, IconThemeGuard, resolve_icon_path};
