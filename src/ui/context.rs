use std::{cell::Cell, rc::Rc};

use gio::{glib::object::ObjectExt, prelude::ListModelExt, ListStore};
use gtk4::{ListView, Revealer, ScrolledWindow, SignalListItemFactory, SingleSelection};

use crate::{
    g_subclasses::{action_entry::ContextAction, emoji_action_entry::EmojiContextAction},
    utils::config::ConfigGuard,
};

use super::util::ContextUI;

pub fn make_context() -> (ContextUI<ContextAction>, Revealer) {
    let max_heigth = ConfigGuard::read()
        .map_or(60, |c| c.appearance.height - 200)
        .max(0);
    let model = ListStore::new::<ContextAction>();
    let factory = SignalListItemFactory::new();
    let selection = SingleSelection::new(Some(model.clone()));
    let context_open = Rc::new(Cell::new(false));
    let context = ListView::builder()
        .name("context-menu")
        .model(&selection)
        .factory(&factory)
        .focusable(false)
        .build();

    let viewport = ScrolledWindow::builder()
        .child(&context)
        .propagate_natural_height(true)
        .max_content_height(max_heigth)
        .vexpand(true)
        .width_request(300)
        .build();

    let revealer = Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::Crossfade)
        .transition_duration(100)
        .valign(gtk4::Align::End)
        .halign(gtk4::Align::End)
        .child(&viewport)
        .build();

    if !ConfigGuard::read().map_or(false, |c| c.behavior.animate) {
        revealer.set_transition_duration(0);
    }

    model.connect_items_changed({
        let revealer = revealer.downgrade();
        let context_open = Rc::clone(&context_open);
        move |model, _, _, _| {
            let n_items = model.n_items();
            if let Some(revealer) = revealer.upgrade() {
                if n_items != 0 {
                    revealer.set_reveal_child(true);
                    context_open.set(true);
                } else {
                    let tmp = revealer.transition_duration();
                    revealer.set_transition_duration(0);
                    revealer.set_reveal_child(false);
                    revealer.set_transition_duration(tmp);
                    context_open.set(false);
                }
            }
        }
    });

    let ui = ContextUI::<ContextAction>::new(model.downgrade(), context.downgrade(), context_open);
    ui.make_factory();
    (ui, revealer)
}
pub fn make_emoji_context() -> (ContextUI<EmojiContextAction>, Revealer) {
    let max_heigth = ConfigGuard::read()
        .map_or(60, |c| c.appearance.height - 200)
        .max(0);
    let model = ListStore::new::<EmojiContextAction>();
    let factory = SignalListItemFactory::new();
    let selection = SingleSelection::new(Some(model.clone()));
    let context_open = Rc::new(Cell::new(false));
    let context = ListView::builder()
        .name("context-menu")
        .css_classes(["emoji"])
        .model(&selection)
        .factory(&factory)
        .focusable(false)
        .build();

    let viewport = ScrolledWindow::builder()
        .child(&context)
        .propagate_natural_height(true)
        .max_content_height(max_heigth)
        .vexpand(true)
        .width_request(300)
        .build();

    let revealer = Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::Crossfade)
        .transition_duration(100)
        .valign(gtk4::Align::End)
        .halign(gtk4::Align::End)
        .child(&viewport)
        .build();

    if !ConfigGuard::read().map_or(false, |c| c.behavior.animate) {
        revealer.set_transition_duration(0);
    }

    model.connect_items_changed({
        let revealer = revealer.downgrade();
        let context_open = Rc::clone(&context_open);
        move |model, _, _, _| {
            let n_items = model.n_items();
            if let Some(revealer) = revealer.upgrade() {
                if n_items != 0 {
                    revealer.set_reveal_child(true);
                    context_open.set(true);
                } else {
                    let tmp = revealer.transition_duration();
                    revealer.set_transition_duration(0);
                    revealer.set_reveal_child(false);
                    revealer.set_transition_duration(tmp);
                    context_open.set(false);
                }
            }
        }
    });

    let ui =
        ContextUI::<EmojiContextAction>::new(model.downgrade(), context.downgrade(), context_open);
    ui.make_factory();
    (ui, revealer)
}
