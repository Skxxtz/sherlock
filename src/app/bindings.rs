use std::collections::HashMap;

use gpui::{App, KeyBinding};

use crate::{
    ui::{
        UIFunction,
        launcher::{
            Execute, NextVar, OpenContext, PrevVar, Quit, SelectionDown, SelectionLeft,
            SelectionRight, SelectionUp,
        },
        search_bar::actions::{
            Backspace, Copy, Cut, Delete, DeleteAll, End, Home, Paste, SelectAll, ShortcutAction,
        },
    },
    utils::config::ConfigGuard,
};

pub(super) fn register_bindings(cx: &mut App) {
    let mut bindings: HashMap<String, KeyBinding> = HashMap::new();

    let mut add = |key: &str, binding: KeyBinding| {
        bindings.insert(key.to_string(), binding);
    };

    // default binds
    add("backspace", KeyBinding::new("backspace", Backspace, None));
    add("delete", KeyBinding::new("delete", Delete, None));
    add(
        "ctrl-backspace",
        KeyBinding::new("ctrl-backspace", DeleteAll, None),
    );
    add("ctrl-a", KeyBinding::new("ctrl-a", SelectAll, None));
    add("ctrl-v", KeyBinding::new("ctrl-v", Paste, None));
    add("ctrl-c", KeyBinding::new("ctrl-c", Copy, None));
    add("ctrl-x", KeyBinding::new("ctrl-x", Cut, None));
    add("escape", KeyBinding::new("escape", Quit, None));

    add("home", KeyBinding::new("home", Home, None));
    add("end", KeyBinding::new("end", End, None));
    // add("left", KeyBinding::new("left", Left, None));
    // add("right", KeyBinding::new("right", Right, None));
    add("down", KeyBinding::new("down", SelectionDown, None));
    add("up", KeyBinding::new("up", SelectionUp, None));
    add("left", KeyBinding::new("left", SelectionLeft, None));
    add("right", KeyBinding::new("right", SelectionRight, None));
    add(
        "variable.tab",
        UIFunction::Complete.into_bind("variable.tab").unwrap(),
    );
    add("enter", KeyBinding::new("enter", Execute, None));
    add("tab", KeyBinding::new("tab", NextVar, None));
    add("shift-tab", KeyBinding::new("shift-tab", PrevVar, None));
    add("ctrl-l", KeyBinding::new("ctrl-l", OpenContext, None));

    if let Ok(config) = ConfigGuard::read() {
        for (key, action_type) in &config.keybinds {
            if *action_type == UIFunction::Shortcut && key.contains("<digit>") {
                for i in 0..=9 {
                    let actual_key = key.replace("<digit>", &i.to_string());
                    add(
                        &actual_key,
                        KeyBinding::new(&actual_key, ShortcutAction { index: i }, None),
                    );
                }
            } else if let Some(binding) = action_type.into_bind(key) {
                add(key, binding);
            }
        }
    }

    cx.bind_keys(bindings.into_values().collect::<Vec<_>>());
}
