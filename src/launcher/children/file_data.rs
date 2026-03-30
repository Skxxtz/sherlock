use crate::{
    app::ThemeData,
    launcher::{
        ExecMode, Launcher,
        children::{RenderableChildImpl, Selection},
    },
    loader::resolve_icon_path,
    ui::launcher::views::NavigationViewType,
};
use gpui::{
    AnyElement, Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div, img,
    prelude::FluentBuilder, px,
};
use std::{path::Path, sync::Arc, time::UNIX_EPOCH};

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

        let kind = if is_symlink {
            "Symlink".into()
        } else if is_dir {
            "Directory".into()
        } else {
            extension
                .as_deref()
                .map(|e| format!("{e} File"))
                .unwrap_or_else(|| "File".into())
        };

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
            size,
            modified,
            created,
            permissions,
            executable,
            symlink_target,
            extension,
        })
    }
}

struct FileMeta {
    kind: String,
    size: String,
    modified: String,
    created: String,
    permissions: String,
    executable: bool,
    symlink_target: Option<String>,
    extension: Option<String>,
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

    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hours, minutes)
}

#[cfg(unix)]
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

impl<'a> RenderableChildImpl<'a> for FileData {
    fn render(
        &self,
        _launcher: &Arc<Launcher>,
        selection: Selection,
        theme: Arc<ThemeData>,
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

    fn sidebar(&self, theme: Arc<ThemeData>) -> Option<AnyElement> {
        let meta = self.fetch_meta()?;

        // Compact label/value row
        let row = |label: &'static str, value: String| {
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
                        .child(SharedString::from(label)),
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

        Some(
            div()
                .w(px(400.))
                .p(px(16.))
                .mb(px(10.))
                .rounded_lg()
                .bg(theme.bg_selected)
                .border_1()
                .border_color(theme.border_selected)
                .flex_col()
                .overflow_hidden()
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
                                .bg(theme.mantle)
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
                                                .bg(theme.mantle)
                                                .text_xs()
                                                .font_family(theme.font_family.clone())
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(theme.secondary_text)
                                                .child(SharedString::from(meta.kind.clone())),
                                        )
                                        .when(meta.executable, |this| {
                                            this.child(
                                                div()
                                                    .px(px(6.))
                                                    .py(px(1.))
                                                    .rounded(px(4.))
                                                    .bg(theme.mantle)
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
                        .child(row("Size", meta.size))
                        .child(row("Kind", meta.kind))
                        .when_some(meta.symlink_target, |this, target| {
                            this.child(row("Points to", target))
                        }),
                )
                .child(separator())
                .child(
                    div()
                        .flex_col()
                        .child(section_label("Dates"))
                        .child(row("Modified", meta.modified))
                        .child(row("Created", meta.created)),
                )
                .child(separator())
                .child(
                    div()
                        .flex_col()
                        .child(section_label("Permissions"))
                        .child(row("Mode", meta.permissions)),
                )
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
