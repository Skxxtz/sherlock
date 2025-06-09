use std::{cell::Cell, rc::Rc};

use gio::{
    glib::object::{Cast, CastNone, ObjectExt},
    prelude::ListModelExt,
    ListStore,
};
use gtk4::{
    prelude::ListItemExt, ListView, Revealer, ScrolledWindow, SignalListItemFactory,
    SingleSelection,
};

use crate::{g_subclasses::action_entry::ContextAction, CONFIG};

use super::util::ContextUI;

pub fn make_context() -> (ContextUI, Revealer) {
    let max_heigth = CONFIG.get().map_or(60, |c| c.appearance.height - 200);
    let factory = make_factory();
    let model = ListStore::new::<ContextAction>();
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

    if !CONFIG.get().map_or(false, |c| c.behavior.animate) {
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

    let ui = ContextUI {
        model: model.downgrade(),
        view: context.downgrade(),
        open: context_open,
    };
    (ui, revealer)
}
fn make_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();
    factory.connect_bind(|_, item| {
        let item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Item mut be a ListItem");
        let row = item
            .item()
            .clone()
            .and_downcast::<ContextAction>()
            .expect("Row should be ContextAction");
        item.set_child(Some(&row));
    });
    factory
}
