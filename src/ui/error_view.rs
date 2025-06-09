use gio::glib::WeakRef;
use gio::ListStore;
use gtk4::{self, gdk::Key, prelude::*, Builder, EventControllerKey};
use gtk4::{Box as GtkBox, ListView, SignalListItemFactory, SingleSelection};
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::api::SherlockAPI;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::ui::tiles::Tile;
use crate::utils::errors::SherlockError;

pub fn errors(
    errors: &Vec<SherlockError>,
    non_breaking: &Vec<SherlockError>,
    stack_page: &Rc<RefCell<String>>,
    sherlock: Rc<RefCell<SherlockAPI>>,
) -> (GtkBox, WeakRef<ListStore>) {
    let (stack, ui) = construct(errors, non_breaking);

    {
        let mut sherlock = sherlock.borrow_mut();
        sherlock.errors = Some(ui.model.clone());
    }

    nav_event(&stack, ui.results.clone(), stack_page);
    return (stack, ui.model.clone());
}
fn nav_event(stack: &GtkBox, result_holder: WeakRef<ListView>, stack_page: &Rc<RefCell<String>>) {
    // Wrap the event controller in an Rc<RefCell> for shared mutability
    let event_controller = EventControllerKey::new();
    let stack_page = Rc::clone(&stack_page);

    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed(move |_, key, _, _| {
        if stack_page.borrow().as_str() != "error-page" {
            return false.into();
        }
        match key {
            Key::Return => {
                let _ = result_holder.upgrade().map(|widget| {
                    widget.activate_action(
                        "win.switch-page",
                        Some(&String::from("error-page->search-page").to_variant()),
                    )
                });
                true.into()
            }
            _ => false.into(),
        }
    });
    stack.add_controller(event_controller);
}

fn construct(errors: &Vec<SherlockError>, non_breaking: &Vec<SherlockError>) -> (GtkBox, ErrorUI) {
    // Initialize the builder with the correct path
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/error_view.ui");

    // Get the required object references
    let vbox: GtkBox = builder.object("vbox").unwrap();
    vbox.set_visible(true);
    vbox.set_can_focus(true);
    vbox.set_focus_on_click(true);
    vbox.set_focusable(true);
    vbox.set_sensitive(true);
    vbox.connect_map(move |myself| {
        myself.grab_focus();
    });

    let results: ListView = builder.object("result-frame").unwrap();

    // Setup model and factory
    let model = ListStore::new::<SherlockRow>();
    let factory = make_factory();
    results.set_factory(Some(&factory));

    // Start first update cycle to update async tiles
    let (_, breaking_error_tiles) = Tile::error_tile(0, errors, "🚨", "ERROR");
    let (_, error_tiles) = Tile::error_tile(0, non_breaking, "⚠️", "WARNING");

    breaking_error_tiles
        .into_iter()
        .for_each(|tile| model.append(&tile));
    error_tiles.into_iter().for_each(|tile| model.append(&tile));
    results.set_factory(Some(&factory));
    let selection = SingleSelection::new(Some(model.clone()));
    results.set_model(Some(&selection));

    let ui = ErrorUI {
        model: model.downgrade(),
        results: results.downgrade(),
    };

    (vbox, ui)
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
            .and_downcast::<SherlockRow>()
            .expect("Row should be SherlockRow");
        item.set_child(Some(&row));
    });
    factory
}
struct ErrorUI {
    model: WeakRef<ListStore>,
    results: WeakRef<ListView>,
}
