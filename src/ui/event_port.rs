use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::CastNone, variant::ToVariant, WeakRef};
use gtk4::{
    gdk::{Key, ModifierType},
    prelude::{EditableExt, FilterExt, SorterExt, WidgetExt},
    CustomFilter, CustomSorter,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    api::{call::ApiCall, server::SherlockServer},
    g_subclasses::{action_entry::ContextAction, tile_item::TileItem},
    loader::util::ExecVariable,
    prelude::SherlockNav,
    ui::{
        g_templates::SearchUiObj, key_actions::KeyActions, search::UserBindHandler, util::ContextUI,
    },
    utils::config::ConfigGuard,
};

/// The event port is a centralized point where events get handled. It solves two main issues:
///     - Shared behavior on multiple widgets
///     - User binds defined by assigning actions to keys {key: action}
///
/// Main Flow:
/// 1. Get event as string (e.g, "tab") from widget
/// 2. Get corresponding event from binds mapping
/// 3. Execute action
pub struct EventPort {
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    binds: HashMap<String, UIFunction>,
    current_mode: Rc<RefCell<String>>,
    key_actions: KeyActions,
}
impl EventPort {
    // TODO:
    pub fn new(
        ui: WeakRef<SearchUiObj>,
        filter: WeakRef<CustomFilter>,
        sorter: WeakRef<CustomSorter>,
        binds: HashMap<String, UIFunction>,
        current_mode: Rc<RefCell<String>>,
        context: ContextUI<ContextAction>,
        custom_handler: Rc<RefCell<UserBindHandler>>,
    ) -> Self {
        let key_actions = KeyActions::new(ui, context, custom_handler);
        Self {
            filter,
            sorter,
            binds,
            current_mode,
            key_actions,
        }
    }

    pub fn handle_key_event(&self, key: &str, reference: Rc<RefCell<Self>>) -> bool {
        // Check if shortcut bind
        let mut key = key.to_string();
        let mut shortcut_index = 0;
        if let Some(pos) = key.rfind('-') {
            let (before, last) = key.split_at(pos + 1);
            if let Ok(num) = last.parse::<u8>() {
                shortcut_index = num;
                key = format!("{}-<digit>", before);
            }
        }

        // Check if context open escape
        let action = if key == "escape" && self.key_actions.context.open.get() {
            Some(UIFunction::CloseContext).as_ref()
        } else {
            self.binds.get(&key)
        };

        if let Some(action) = action {
            match action {
                UIFunction::ItemDown => {
                    self.key_actions.on_next();
                    self.update_var_ui(reference);
                }
                UIFunction::ItemUp => {
                    self.key_actions.on_prev();
                    self.update_var_ui(reference);
                }
                UIFunction::ItemLeft => {
                    if !ConfigGuard::read().map_or(false, |c| c.behavior.use_lr_nav) {
                        self.key_actions.on_prev();
                        self.update_var_ui(reference);
                    }
                }
                UIFunction::ItemRight => {
                    if !ConfigGuard::read().map_or(false, |c| c.behavior.use_lr_nav) {
                        self.key_actions.on_next();
                        self.update_var_ui(reference);
                    }
                }

                UIFunction::ArgNext => {
                    self.key_actions.arg_next();
                }
                UIFunction::ArgPrev => {
                    self.key_actions.arg_prev();
                }

                UIFunction::Exec => {
                    if !ConfigGuard::read().map_or(false, |c| c.runtime.multi) {
                        self.key_actions.on_return(Some(true));
                    } else {
                        self.key_actions.on_multi_return(Some(true));
                    }
                }
                UIFunction::ExecInplace => {
                    if !ConfigGuard::read().map_or(false, |c| c.runtime.multi) {
                        self.key_actions.on_return(Some(false));
                    } else {
                        self.key_actions.on_multi_return(Some(false));
                    }
                }

                UIFunction::MultiSelect => {
                    self.key_actions.mark_active();
                }

                UIFunction::ToggleContext => {
                    self.key_actions.open_context();
                }
                UIFunction::CloseContext => {
                    self.key_actions.close_context();
                }

                UIFunction::ClearBar => {
                    if let Some(ui) = self.key_actions.ui.upgrade() {
                        if let Some(bar) = ui.current_bar() {
                            bar.set_text("");
                        }
                    }
                }

                UIFunction::Backspace => {
                    if let Some(ui) = self.key_actions.ui.upgrade() {
                        if let Some(bar) = ui.current_bar() {
                            let ctext = bar.text().to_string();
                            if ctext.is_empty() && self.current_mode.borrow().as_str() != "all" {
                                let _ = self.key_actions.search_bar.upgrade().map(|entry| {
                                    let _ = entry.activate_action(
                                        "win.switch-mode",
                                        Some(&"all".to_variant()),
                                    );
                                    // apply filter and sorter
                                    self.filter.upgrade().map(|filter| {
                                        filter.changed(gtk4::FilterChange::Different)
                                    });
                                    self.sorter.upgrade().map(|sorter| {
                                        sorter.changed(gtk4::SorterChange::Different)
                                    });
                                });
                            }
                            // Focus first item and check for overflow
                            self.key_actions.focus_first(self.current_mode.clone());
                        }
                    }
                }

                UIFunction::ErrorPage => {
                    let api_call = ApiCall::SwitchMode(crate::api::api::SherlockModes::Error);
                    let _ = SherlockServer::send_action(api_call);
                }

                UIFunction::Shortcut => {
                    let internal_index = if shortcut_index == 0 {
                        9
                    } else {
                        shortcut_index - 1
                    };
                    self.key_actions
                        .results
                        .upgrade()
                        .map(|r| r.execute_by_index(internal_index as u32));
                }
            }
            // If some action exists, capture event, else let pass
            true
        } else {
            false
        }
    }

    pub fn key_event_string(key: Key, mods: ModifierType) -> String {
        let modifiers = mods
            .iter_names()
            .map(|(n, _)| {
                let s = match n {
                    "CONTROL_MASK" => "ctrl",
                    "SHIFT_MASK" => "shift",
                    "LOCK_MASK" => "caps",
                    "ALT_MASK" | "MOD1_MASK" => "alt",
                    "SUPER_MASK" | "MOD4_MASK" => "meta",
                    "MOD5_MASK" => "mod5",
                    _ => n,
                };
                s.to_string()
            })
            .join("-");

        let key_name = key.name().unwrap_or_default().to_lowercase();

        let normalized = match key_name.as_str() {
            "iso_left_tab" => "tab",
            "iso_level3_shift" => "alt_gr",
            "control_l" => "ctrl_l",
            "control_r" => "ctrl_r",
            other => other,
        }
        .to_string();

        if normalized.is_empty() {
            return String::new();
        } else if modifiers.is_empty() {
            normalized
        } else {
            format!("{}-{}", modifiers, normalized)
        }
    }

    fn update_var_ui(&self, reference: Rc<RefCell<Self>>) {
        if let Some(ui) = self.key_actions.ui.upgrade() {
            let res = ui.imp().results.get();

            // Show arg bars
            if let Some(sel) = res.selected_item().and_downcast::<TileItem>() {
                let vars = sel.variables();
                if vars.len() > 0 {
                    // add variable fields
                    ui.remove_arg_bars();
                    for (i, var) in vars.into_iter().enumerate() {
                        match var {
                            ExecVariable::StringInput(placeholder) => {
                                ui.add_arg_bar(i as u8, &placeholder, reference.clone());
                            }
                            ExecVariable::PasswordInput(placeholder) => {
                                ui.add_pwd_bar(i as u8, &placeholder, reference.clone());
                            }
                        }
                    }
                } else {
                    // remove all variable fields
                    ui.remove_arg_bars();
                }
            }
        }
    }

    // Getters
    pub fn ui(&self) -> WeakRef<SearchUiObj> {
        self.key_actions.ui.clone()
    }
}

#[derive(Deserialize, Serialize, Hash, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UIFunction {
    ItemDown,
    ItemUp,
    ItemLeft,
    ItemRight,

    ArgNext,
    ArgPrev,

    Exec,
    ExecInplace,

    MultiSelect,

    ToggleContext,
    CloseContext,

    ClearBar,
    Backspace,

    ErrorPage,

    Shortcut,
}
