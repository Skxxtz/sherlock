use std::sync::Arc;

use gpui::{
    ImageSource, InteractiveElement, IntoElement, ParentElement, Styled, div, hsla, img,
    prelude::FluentBuilder, px, relative,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    launcher::{
        children::emoji_data::{EmojiAction, apply_skin_tones, get_selected_skin_tones},
        emoji_launcher::ALL_SKIN_TONES,
    },
    loader::utils::ApplicationAction,
};

#[derive(Debug, PartialEq)]
pub enum ContextMenuAction {
    App(ApplicationAction),
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
    pub fn render_row(&self, is_selected: bool) -> impl IntoElement {
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
                hsla(0.0, 0.0, 0.8, 1.0)
            } else {
                hsla(0.6, 0.0217, 0.3608, 1.0)
            })
            .text_size(px(13.))
            .line_height(relative(1.0))
            .items_center()
            .bg(if is_selected {
                hsla(0., 0., 0.149, 1.0)
            } else {
                hsla(0., 0., 0., 0.)
            })
            .hover(|s| {
                if is_selected {
                    s
                } else {
                    s.bg(hsla(0., 0., 0.12, 1.0))
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
    pub fn render_col(&self, row_is_selected: bool) -> impl IntoElement {
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
                hsla(0.0, 0.0, 0.8, 1.0)
            } else {
                hsla(0.6, 0.0217, 0.3608, 1.0)
            })
            .text_size(px(13.))
            .line_height(relative(1.0))
            .items_center()
            .bg(if row_is_selected {
                hsla(0., 0., 0.149, 1.0)
            } else {
                hsla(0., 0., 0., 0.)
            })
            .hover(|s| {
                if row_is_selected {
                    s
                } else {
                    s.bg(hsla(0., 0., 0.12, 1.0))
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
                    .when(col_idx == i, |this| this.bg(hsla(0.0, 0.0, 1.0, 0.15)))
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
