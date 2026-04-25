use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui::{App, Context, WeakEntity};

use crate::{
    launcher::{Launcher, message_launcher::MessageLauncher, variant_type::LauncherType},
    ui::{
        model::Model,
        widgets::{RenderableChild, message::MessageChild},
    },
    utils::{config::HomeType, errors::SherlockMessage},
};

pub struct MessageView {
    pub launcher: Arc<Launcher>,
    pub count: Cell<usize>,
    pub model: Model,
}

impl MessageView {
    pub fn new(data: Vec<SherlockMessage>, cx: &mut Context<Self>) -> Self {
        let launcher = Arc::new(Launcher {
            name: None,
            display_name: Some("Errors".into()),
            icon: None,
            alias: None,
            on_return: None,
            exit: false,
            priority: 1,
            r#async: false,
            home: HomeType::Home,
            launcher_type: LauncherType::Message(MessageLauncher {}),
            shortcut: false,
            spawn_focus: true,
            actions: None,
            add_actions: None,
        });
        let messages: Vec<_> = data
            .into_iter()
            .map(|message| {
                let weak = cx.entity().downgrade();
                let inner = MessageChild::new(message).on_dismiss(move |cx, idx| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(cx, |message_view, cx| {
                            message_view.remove_message(idx, cx);
                        });
                    }
                });
                RenderableChild::Message {
                    launcher: Arc::clone(&launcher),
                    inner,
                }
            })
            .collect();

        Self {
            launcher,
            count: Cell::new(messages.len()),
            model: Model::standard(messages, cx),
        }
    }
    /// This adds a message from the Model. It requires a filter and sort afterwards
    pub fn push_message(
        &self,
        message: SherlockMessage,
        weak: WeakEntity<MessageView>,
        cx: &mut App,
    ) {
        self.model.data().update(cx, |this, _| {
            let data = Rc::make_mut(this);
            data.push(RenderableChild::Message {
                launcher: self.launcher.clone(),
                inner: MessageChild::new(message).on_dismiss(move |cx, idx| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(cx, |message_view, cx| {
                            message_view.remove_message(idx, cx);
                        });
                    }
                }),
            });
        });
        self.count.update(|i| i + 1);
    }
    /// This removes a message from the Model. It requires a filter and sort afterwards
    pub fn remove_message(&mut self, idx: usize, cx: &mut App) {
        let Model::Standard {
            data,
            filtered_indices,
            ..
        } = &mut self.model
        else {
            return;
        };

        let removed = data.update(cx, |this, _| {
            if idx < this.len() {
                let data = Rc::make_mut(this);
                data.remove(idx);
                true
            } else {
                false
            }
        });

        if removed {
            let mut vec = filtered_indices.to_vec();
            if let Some(pos) = vec.iter().position(|&x| x == idx) {
                vec.remove(pos);
            }

            for val in vec.iter_mut() {
                if *val > idx {
                    *val -= 1;
                }
            }

            *filtered_indices = Arc::from(vec);
            self.count.update(|i| i.saturating_sub(1));
        }
    }
}
