use std::sync::Arc;

use serde::de::IntoDeserializer;

use crate::launcher::children::RenderableChild;
use crate::launcher::{LauncherProvider, LauncherType};
use crate::loader::application_loader::parse_priority;
use crate::loader::resolve_icon_path;
use crate::loader::utils::{ApplicationAction, RawLauncher, deserialize_named_appdata};
use crate::sherlock_msg;
use crate::ui::launcher::context_menu::ContextMenuAction;
use crate::utils::errors::types::SherlockErrorType;

#[derive(Clone, Debug)]
pub struct CategoryLauncher {}

impl LauncherProvider for CategoryLauncher {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Categories(CategoryLauncher {})
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::Launcher>,
        ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<super::children::RenderableChild>, crate::utils::errors::SherlockMessage> {
        let cmds = opts.get("categories").ok_or_else(|| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Invalid launcher configuration.".into()),
                "Category launcher does not contain any categories."
            )
        })?;
        let app_data =
            deserialize_named_appdata(cmds.clone().into_deserializer()).unwrap_or_default();

        let children: Vec<RenderableChild> = app_data
            .into_iter()
            .map(|mut inner| {
                let count = inner
                    .exec
                    .as_deref()
                    .and_then(|exec| ctx.counts.get(exec))
                    .copied()
                    .unwrap_or(0u32);
                inner.icon = inner
                    .icon
                    .and_then(|i| i.to_str().and_then(resolve_icon_path));
                inner.priority = Some(parse_priority(
                    launcher.priority as f32,
                    count,
                    ctx.max_decimals,
                ));
                inner.actions = inner
                    .actions
                    .iter()
                    .map(|action_arc| match action_arc.as_ref() {
                        ContextMenuAction::App(app_action) => {
                            let resolved_icon = app_action
                                .icon
                                .as_deref()
                                .and_then(|p| p.to_str())
                                .and_then(|s| resolve_icon_path(s));

                            Arc::new(ContextMenuAction::App(ApplicationAction {
                                icon: resolved_icon,
                                ..app_action.clone()
                            }))
                        }
                        ContextMenuAction::Fn(_) => Arc::clone(action_arc),
                        ContextMenuAction::Emoji(_) => Arc::clone(action_arc),
                    })
                    .collect();

                RenderableChild::AppLike {
                    launcher: Arc::clone(&launcher),
                    inner,
                }
            })
            .collect();

        Ok(children)
    }
}
