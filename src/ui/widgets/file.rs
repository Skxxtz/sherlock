use crate::{
    app::theme::{ActiveTheme, ThemeData},
    launcher::{ExecMode, Launcher},
    loader::resolve_icon_path,
    ui::{
        launcher::views::NavigationViewType,
        widgets::{RenderableChildImpl, Selection},
    },
};
use gpui::{
    AnyElement, App, Image, ImageSource, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, StyledImage, div, img, prelude::FluentBuilder, px,
};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    sync::Arc,
    time::UNIX_EPOCH,
};

#[derive(Clone, Default, Debug)]
pub struct FileData {
    loc: SharedString,
    name: SharedString,
    icon: Option<Arc<Path>>,
}

impl FileData {
    pub fn new(loc: Arc<str>) -> Self {
        let name: Arc<str> = loc
            .trim_end_matches('/')
            .rsplit_once('/')
            .map(|(_, name)| name)
            .unwrap_or(&loc)
            .into();
        Self {
            loc: loc.clone().into(),
            name: name.into(),
            icon: None,
        }
    }

    pub fn with_icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }

    fn fetch_meta(&self) -> Option<FileMeta> {
        let path = std::path::Path::new(self.loc.as_ref());
        let meta = std::fs::symlink_metadata(path).ok()?;
        let is_dir = meta.is_dir();
        let is_symlink = meta.file_type().is_symlink();

        let symlink_target = if is_symlink {
            std::fs::read_link(path)
                .ok()
                .map(|t| t.to_string_lossy().into_owned())
        } else {
            None
        };

        let extension = if is_symlink || is_dir {
            None
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_uppercase())
                .map(|e| e.to_owned())
        };

        let kind = identify_file_type(extension.as_deref(), is_dir, is_symlink);
        let is_image = extension.map_or(false, |ext| {
            matches!(
                ext.as_str(),
                "JPG"
                    | "JPEG"
                    | "PNG"
                    | "GIF"
                    | "WEBP"
                    | "AVIF"
                    | "SVG"
                    | "ICO"
                    | "BMP"
                    | "TIFF"
                    | "HEIC"
                    | "RAW"
            )
        });

        let size = if is_dir {
            std::fs::read_dir(path)
                .map(|e| format!("{} items", e.filter_map(|e| e.ok()).count()))
                .unwrap_or_else(|_| "—".into())
        } else {
            format_size(meta.len())
        };

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| format_timestamp(d.as_secs()))
            .unwrap_or_else(|| "—".into());

        let created = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| format_timestamp(d.as_secs()))
            .unwrap_or_else(|| "—".into());

        let (permissions, executable) = {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();
            (format_permissions(mode), !is_dir && (mode & 0o111 != 0))
        };

        Some(FileMeta {
            kind,
            size: size.into(),
            modified,
            created,
            permissions,
            executable,
            symlink_target,
            is_image,
        })
    }
}

struct FileMeta {
    kind: &'static str,
    size: SharedString,
    modified: String,
    created: String,
    permissions: String,
    executable: bool,
    symlink_target: Option<String>,
    is_image: bool,
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{bytes} B")
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

fn format_timestamp(secs: u64) -> String {
    // Simple formatting without chrono — compute y/m/d from epoch seconds
    let days_since_epoch = secs / 86400;
    let seconds_in_day = secs % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;

    // Rata Die algorithm for Gregorian calendar
    let z = days_since_epoch as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:02}.{:02}.{:04} {:02}:{:02}", d, m, y, hours, minutes)
}

fn format_permissions(mode: u32) -> String {
    let chars = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let s: String = chars
        .iter()
        .map(|&(bit, ch)| if mode & bit != 0 { ch } else { '-' })
        .collect();
    format!("{s} ({:04o})", mode & 0o777)
}

fn identify_file_type(extension: Option<&str>, is_dir: bool, is_symlink: bool) -> &'static str {
    if is_symlink {
        return "Symbolic Link";
    }
    if is_dir {
        return "Folder";
    }

    match extension.unwrap_or("") {
        // Systems & compiled
        "rs" => "Rust",
        "c" => "C",
        "h" => "C Header",
        "cpp" | "cc" | "cxx" => "C++",
        "hpp" | "hxx" => "C++ Header",
        "go" => "Go",
        "zig" => "Zig",
        "s" | "asm" => "Assembly",
        // JVM
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "scala" => "Scala",
        "class" => "Java Bytecode",
        "jar" => "Java Archive",
        // Scripting
        "py" => "Python",
        "rb" => "Ruby",
        "lua" => "Lua",
        "pl" | "pm" => "Perl",
        "sh" => "Shell Script",
        "bash" => "Bash Script",
        "zsh" => "Zsh Script",
        "fish" => "Fish Script",
        "ps1" => "PowerShell Script",
        // Web
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" | "mts" => "TypeScript",
        "jsx" => "React JSX",
        "tsx" => "React TSX",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "vue" => "Vue Component",
        "svelte" => "Svelte Component",
        "wasm" => "WebAssembly",
        // Data & config
        "json" | "jsonc" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "ini" | "cfg" | "conf" => "Config",
        "env" => "Environment Config",
        "xml" => "XML",
        "csv" => "CSV",
        "tsv" => "TSV",
        "sql" => "SQL",
        "db" | "sqlite" | "sqlite3" => "SQLite Database",
        // Documents
        "md" | "markdown" => "Markdown",
        "rst" => "reStructuredText",
        "tex" => "LaTeX",
        "pdf" => "PDF",
        "txt" => "Plain Text",
        "doc" | "docx" => "Word Document",
        "xls" | "xlsx" => "Excel Spreadsheet",
        "ppt" | "pptx" => "PowerPoint",
        "odt" => "OpenDocument Text",
        "ods" => "OpenDocument Spreadsheet",
        "epub" => "eBook",
        // Images
        "jpg" | "jpeg" => "JPEG Image",
        "png" => "PNG Image",
        "gif" => "GIF Image",
        "webp" => "WebP Image",
        "avif" => "AVIF Image",
        "svg" => "SVG",
        "ico" => "Icon",
        "bmp" => "Bitmap",
        "tiff" | "tif" => "TIFF Image",
        "heic" | "heif" => "HEIF Image",
        "raw" | "cr2" | "nef" => "RAW Image",
        // Audio
        "mp3" => "MP3",
        "flac" => "FLAC",
        "wav" => "WAV",
        "ogg" => "OGG",
        "aac" => "AAC",
        "m4a" => "M4A",
        "opus" => "Opus",
        // Video
        "mp4" | "m4v" => "MP4",
        "mkv" => "MKV",
        "webm" => "WebM",
        "avi" => "AVI",
        "mov" => "QuickTime",
        "wmv" => "WMV",
        // Archives
        "zip" => "ZIP Archive",
        "tar" => "Tar Archive",
        "gz" | "tgz" => "Gzip Archive",
        "xz" => "XZ Archive",
        "bz2" => "Bzip2 Archive",
        "zst" => "Zstandard Archive",
        "7z" => "7-Zip Archive",
        "rar" => "RAR Archive",
        "deb" => "Debian Package",
        "rpm" => "RPM Package",
        "appimage" => "AppImage",
        "flatpak" => "Flatpak",
        // Fonts
        "ttf" => "TrueType Font",
        "otf" => "OpenType Font",
        "woff" | "woff2" => "Web Font",
        // Binary / system
        "so" => "Shared Library",
        "a" => "Static Library",
        "o" => "Object File",
        "exe" => "Executable",
        "dll" => "DLL",
        "lock" => "Lock File",
        "log" => "Log",
        "pid" => "PID File",
        _ => "File",
    }
}

impl<'a> RenderableChildImpl<'a> for FileData {
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(if let Some(icon) = self.icon.as_ref() {
                img(Arc::clone(&icon))
                    .size(px(24.))
                    .flex_shrink_0()
                    .into_any_element()
            } else {
                img(ImageSource::Image(Arc::new(Image::empty())))
                    .size(px(24.))
                    .flex_shrink_0()
                    .into_any_element()
            })
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .min_w_0()
                    .w_full()
                    .child(
                        div()
                            .font_family(theme.font_family.clone())
                            .text_sm()
                            .w_full()
                            .text_color(theme.secondary_text)
                            .when(selection.is_selected, |this| {
                                this.text_color(theme.primary_text)
                            })
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .font_family(theme.font_family.clone())
                            .text_xs()
                            .w_full()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_color(theme.secondary_text)
                            .child(self.loc.clone()),
                    ),
            )
            .into_any_element()
    }

    fn sidebar(&self, _cx: &mut App) -> Option<AnyElement> {
        let meta = self.fetch_meta()?;
        Some(
            FileSidebar {
                meta,
                icon: self.icon.clone(),
                name: self.name.clone(),
                loc: self.loc.clone(),
            }
            .into_any_element(),
        )
    }

    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<Launcher>) -> Option<ExecMode> {
        if self.loc.ends_with('/') {
            return Some(ExecMode::CreateView {
                mode: NavigationViewType::Files {
                    dir: Some(self.loc.clone()),
                },
                launcher: Arc::clone(launcher),
            });
        }
        None
    }

    #[inline(always)]
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }

    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<Launcher>) -> &'a str {
        &self.loc
    }
}

#[derive(IntoElement)]
struct FileSidebar {
    meta: FileMeta,
    loc: SharedString,
    name: SharedString,
    icon: Option<Arc<Path>>,
}
impl RenderOnce for FileSidebar {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        // Compact label/value row
        let row = |label: &'static str, value: SharedString| {
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_4()
                .py(px(4.))
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .flex_shrink_0()
                        .child(label),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.primary_text)
                        .text_right()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(SharedString::from(value)),
                )
        };

        let section_label = |text: &'static str| {
            div()
                .text_xs()
                .font_family(theme.font_family.clone())
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.secondary_text)
                .pt_3()
                .pb_1()
                .child(SharedString::from(text))
        };

        let separator = || div().h(px(1.)).w_full().bg(theme.border_selected).my_1();

        div()
            .min_h_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pb(px(12.))
                    .child(
                        // Icon box
                        div()
                            .size(px(48.))
                            .rounded_lg()
                            .bg(theme.bg_muted)
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .child(if let Some(icon) = &self.icon {
                                img(Arc::clone(icon)).size(px(32.)).into_any_element()
                            } else {
                                div().into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_family(theme.font_family.clone())
                                    .px(px(3.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.primary_text)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(self.name.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(4.))
                                            .bg(theme.bg_muted)
                                            .text_xs()
                                            .font_family(theme.font_family.clone())
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.secondary_text)
                                            .child(self.meta.kind),
                                    )
                                    .when(self.meta.executable, |this| {
                                        this.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(4.))
                                                .bg(theme.bg_muted)
                                                .text_xs()
                                                .font_family(theme.font_family.clone())
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(theme.secondary_text)
                                                .child(SharedString::from("Executable")),
                                        )
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .mt_auto()
                    .pt(px(5.))
                    .text_xs()
                    .text_color(theme.secondary_text)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(self.loc.clone()),
            )
            .child(separator())
            .child(
                div()
                    .flex_col()
                    .child(section_label("Info"))
                    .child(row("Size", self.meta.size.clone()))
                    .child(row("Kind", self.meta.kind.into()))
                    .when_some(self.meta.symlink_target, |this, target| {
                        this.child(row("Points to", target.into()))
                    }),
            )
            .child(separator())
            .child(
                div()
                    .flex_col()
                    .child(section_label("Dates"))
                    .child(row("Modified", self.meta.modified.into()))
                    .child(row("Created", self.meta.created.into())),
            )
            .child(separator())
            .child(
                div()
                    .flex_col()
                    .child(section_label("Permissions"))
                    .child(row("Mode", self.meta.permissions.into())),
            )
            .when(self.meta.is_image, |this| {
                this.child(separator())
                    .child(section_label("Preview"))
                    .child(
                        if let Some(image) = load_thumbnail(self.loc.as_str(), 150) {
                            div()
                                .w_full()
                                .mt(px(3.))
                                .mb(px(12.))
                                .rounded_md()
                                .overflow_hidden()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    img(image)
                                        .w_full()
                                        .h(px(200.))
                                        .object_fit(gpui::ObjectFit::Contain),
                                )
                        } else {
                            div()
                                .w_full()
                                .aspect_square()
                                .bg(theme.bg_muted)
                                .rounded_md()
                                .flex()
                                .justify_center()
                                .items_center()
                                .child(
                                    div()
                                        .px_3()
                                        .py_1()
                                        .rounded_sm()
                                        .bg(theme.bg_selected)
                                        .font_family(theme.font_family.clone())
                                        .text_size(px(11.))
                                        .text_color(theme.secondary_text)
                                        .child(self.meta.size),
                                )
                        },
                    )
            })
    }
}

fn load_thumbnail(path: &str, max_px: u32) -> Option<Arc<gpui::Image>> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let id = hasher.finish();

    if ext.as_deref() == Some("svg") {
        return load_svg_thumbnail(path, max_px);
    }

    let file_size = std::fs::metadata(path).ok()?.len();
    if file_size > 1 * 1024 * 1024 {
        return None;
    }

    let img = image::open(path).ok()?;
    let thumb = img.thumbnail(max_px, max_px);
    let mut bytes = Vec::new();
    thumb
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .ok()?;

    Some(Arc::new(gpui::Image {
        format: gpui::ImageFormat::Png,
        bytes: bytes.into(),
        id,
    }))
}

fn load_svg_thumbnail(path: &str, max_px: u32) -> Option<Arc<gpui::Image>> {
    let bytes = std::fs::read(path).ok()?;
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&bytes, &options).ok()?;

    let size = tree.size();
    let scale = (max_px as f32 / size.width().max(size.height())).min(1.0);
    let w = (size.width() * scale) as u32;
    let h = (size.height() * scale) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let id = hasher.finish();

    Some(Arc::new(gpui::Image {
        format: gpui::ImageFormat::Png,
        bytes: pixmap.encode_png().ok()?.into(),
        id,
    }))
}
