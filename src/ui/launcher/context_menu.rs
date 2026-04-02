use std::{fmt::Debug, path::Path, sync::Arc};

use gpui::{
    App, ImageSource, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div,
    img, prelude::FluentBuilder, px, relative,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    app::theme::ThemeData,
    launcher::emoji_launcher::ALL_SKIN_TONES,
    loader::{resolve_icon_path, utils::ApplicationAction},
    ui::widgets::emoji::{EmojiAction, apply_skin_tones, get_selected_skin_tones},
};

#[derive(Debug, PartialEq)]
pub enum ContextMenuAction {
    App(ApplicationAction),
    Fn(DynamicFunctionAction),
    Emoji(EmojiAction),
}
impl From<ApplicationAction> for Arc<ContextMenuAction> {
    fn from(value: ApplicationAction) -> Self {
        Arc::new(ContextMenuAction::App(value))
    }
}
impl Serialize for ContextMenuAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ContextMenuAction::App(app) => serializer.serialize_some(app),

            _ => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for ContextMenuAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<ApplicationAction>::deserialize(deserializer)?;

        match opt {
            Some(app_action) => Ok(ContextMenuAction::App(app_action)),
            None => Err(serde::de::Error::custom(
                "Found None where App was expected",
            )),
        }
    }
}

impl ContextMenuAction {
    pub fn render_row(&self, is_selected: bool, theme: Arc<ThemeData>) -> impl IntoElement {
        let Self::App(this) = self else { return div() };

        div()
            .group("")
            .rounded_md()
            .relative()
            .flex_1()
            .flex()
            .gap(px(10.))
            .p(px(10.))
            .cursor_pointer()
            .text_color(if is_selected {
                theme.primary_text
            } else {
                theme.secondary_text
            })
            .text_size(px(13.))
            .font_family(theme.font_family.clone())
            .line_height(relative(1.0))
            .items_center()
            .bg(if is_selected {
                theme.bg_selected
            } else {
                theme.bg_idle
            })
            .hover(|s| {
                if is_selected {
                    s
                } else {
                    s.bg(theme.bg_selected)
                }
            })
            .child(if let Some(icon) = this.icon.as_ref() {
                img(Arc::clone(icon)).size(px(16.)).into_any_element()
            } else {
                img(ImageSource::Image(Arc::new(gpui::Image::empty())))
                    .size(px(16.))
                    .into_any_element()
            })
            .child(this.name.as_ref().unwrap().clone())
    }
    pub fn render_emoji_col(
        &self,
        row_is_selected: bool,
        theme: Arc<ThemeData>,
    ) -> impl IntoElement {
        let Self::Emoji(this) = self else {
            return div();
        };

        let emoji = this.emoji();
        let mut tones = get_selected_skin_tones();
        let col_idx = this.get_index() as usize;

        div()
            .group("skin-tone-container")
            .flex()
            .flex_row()
            .w_full()
            .justify_between()
            .items_center()
            .rounded_md()
            .relative()
            .gap(px(10.))
            .p(px(4.))
            .cursor_pointer()
            .text_color(if row_is_selected {
                theme.primary_text
            } else {
                theme.secondary_text
            })
            .text_size(px(13.))
            .line_height(relative(1.0))
            .items_center()
            .bg(if row_is_selected {
                theme.bg_selected
            } else {
                theme.bg_idle
            })
            .hover(|s| {
                if row_is_selected {
                    s
                } else {
                    if row_is_selected {
                        s
                    } else {
                        s.bg(theme.bg_selected)
                    }
                }
            })
            .children(ALL_SKIN_TONES.iter().enumerate().map(|(i, tone)| {
                tones[this.for_tone as usize] = *tone;
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .items_center()
                    .rounded_sm()
                    .p(px(8.))
                    .when(col_idx == i, |this| this.bg(theme.border_selected))
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .items_center()
                            .w(px(24.))
                            .child(apply_skin_tones(emoji, &tones).as_str().to_string()),
                    )
            }))
    }
}

pub struct DynamicFunctionAction {
    pub name: SharedString,
    pub icon: Option<Arc<Path>>,
    pub exit: bool,
    pub func: Option<Box<dyn Fn(&mut App) + Send + Sync + 'static>>,
}
impl Debug for DynamicFunctionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}
impl PartialEq for DynamicFunctionAction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.icon == other.icon && self.exit == other.exit
    }
    fn ne(&self, other: &Self) -> bool {
        !(self.name == other.name && self.icon == other.icon && self.exit == other.exit)
    }
}

impl DynamicFunctionAction {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            icon: None,
            exit: true,
            func: None,
        }
    }

    pub fn on_exec<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut App) + Send + Sync + 'static,
    {
        self.func = Some(Box::new(f));
        self
    }

    pub fn icon(mut self, icon: Arc<Path>) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }

    pub fn exit(mut self, should_exit: bool) -> Self {
        self.exit = should_exit;
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::loader::utils::AppData;

    use super::*;
    use serde_json;

    #[test]
    fn test_context_menu_action_round_trip() {
        // 1. Setup mock actions
        let app_action = ContextMenuAction::App(ApplicationAction::new("test"));

        // 2. Serialize to JSON
        let serialized = serde_json::to_string(&app_action).expect("Failed to serialize");

        // 3. Deserialize back
        let deserialized: ContextMenuAction =
            serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(app_action, deserialized)
    }

    #[test]
    fn test_appdata_round_trip() {
        let app_action = ContextMenuAction::App(ApplicationAction::new("test"));
        let mut app_data = AppData::new();
        app_data.actions = Arc::new([Arc::new(app_action)]);

        let serialized = serde_json::to_string(&app_data).expect("Failed to serialize");

        let deserialized: AppData =
            serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(app_data, deserialized)
    }
}
