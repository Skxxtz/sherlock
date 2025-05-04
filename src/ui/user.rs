use std::{cell::RefCell, rc::Rc};

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::{glib::WeakRef, ListStore};
use gtk4::{
    self,
    gdk::{self, Key, ModifierType},
    prelude::*,
    ApplicationWindow, Box as GtkBox, Builder, CustomFilter, CustomSorter, Entry,
    EventControllerKey, FilterListModel, Image, Justification, ListScrollFlags, ListView, Overlay,
    SignalListItemFactory, SingleSelection, SortListModel,
};
use levenshtein::levenshtein;

use super::{search::SearchHandler, tiles::util::SherlockSearch, util::*};
use crate::{g_subclasses::sherlock_row::SherlockRow, CONFIG};
use gtk4::{Box as HVBox, ScrolledWindow};

use super::tiles::{util::TextViewTileBuilder, Tile};
use crate::loader::pipe_loader::PipeData;

struct PipeUI {
    result_viewport: WeakRef<ScrolledWindow>,
    results: WeakRef<ListView>,
    search_bar: WeakRef<Entry>,
    search_icon_holder: WeakRef<GtkBox>,
    selection: WeakRef<SingleSelection>,
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    binds: ConfKeys,
}

pub fn display_pipe(
    _window: &ApplicationWindow,
    pipe_content: Vec<PipeData>,
    method: &str,
    error_model: WeakRef<ListStore>,
) -> (HVBox, SearchHandler) {
    let (search_text, stack_page, ui) = construct(pipe_content, method);

    if let Some(viewport) = ui.result_viewport.upgrade() {
        viewport.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    }
    // results.focus_first();

    stack_page.connect_realize({
        let search_bar = ui.search_bar.clone();
        move |_| {
            if let Some(entry) = search_bar.upgrade() {
                entry.grab_focus();
            }
        }
    });

    change_event(
        ui.search_bar.clone(),
        ui.results.clone(),
        ui.filter.clone(),
        ui.sorter.clone(),
        ui.selection.clone(),
        &search_text,
    );
    nav_event(
        ui.selection.clone(),
        ui.results.clone(),
        ui.search_bar.clone(),
        ui.binds,
    );

    let handler = SearchHandler::empty(error_model);
    return (stack_page, handler);
}
pub fn display_raw<T: AsRef<str>>(
    content: T,
    center: bool,
    error_model: WeakRef<ListStore>,
) -> (HVBox, SearchHandler) {
    let builder = TextViewTileBuilder::new("/dev/skxxtz/sherlock/ui/text_view_tile.ui");
    builder
        .content
        .as_ref()
        .and_then(|tmp| tmp.upgrade())
        .map(|ctx| {
            let buffer = ctx.buffer();
            ctx.add_css_class("raw_text");
            ctx.set_monospace(true);
            let sanitized: String = content.as_ref().chars().filter(|&c| c != '\0').collect();
            buffer.set_text(&sanitized);
            if center {
                ctx.set_justification(Justification::Center);
            }
        });
    let handler = SearchHandler::empty(error_model);
    let row = builder
        .object
        .as_ref()
        .and_then(|tmp| tmp.upgrade())
        .unwrap_or_default();

    (row, handler)
}
fn construct(pipe_content: Vec<PipeData>, method: &str) -> (Rc<RefCell<String>>, GtkBox, PipeUI) {
    // Collect Modes
    let custom_binds = ConfKeys::new();
    let search_text = Rc::new(RefCell::new(String::new()));

    // Initialize the builder with the correct path
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/search.ui");

    // Get the required object references
    let vbox: GtkBox = builder.object("vbox").unwrap();
    let results: ListView = builder.object("result-frame").unwrap();
    results.set_focusable(false);

    let search_icon_holder: GtkBox = builder.object("search-icon-holder").unwrap_or_default();
    search_icon_holder.add_css_class("search");
    // Create the search icon
    let search_icon = Image::new();
    search_icon.set_icon_name(Some("system-search-symbolic"));
    search_icon.set_widget_name("search-icon");
    search_icon.set_halign(gtk4::Align::End);
    // Create the back arrow
    let search_icon_back = Image::new();
    search_icon_back.set_icon_name(Some("go-previous-symbolic"));
    search_icon_back.set_widget_name("search-icon-back");
    search_icon_back.set_halign(gtk4::Align::End);
    // Set search icons
    let overlay = Overlay::new();
    overlay.set_child(Some(&search_icon));
    overlay.add_overlay(&search_icon_back);

    // Setup model and factory
    let model = ListStore::new::<SherlockRow>();
    let factory = make_factory();
    results.set_factory(Some(&factory));

    // Setup selection
    let sorter = make_sorter(&search_text);
    let filter = make_filter(&search_text);
    let filter_model = FilterListModel::new(Some(model.clone()), Some(filter.clone()));
    let sorted_model = SortListModel::new(Some(filter_model), Some(sorter.clone()));

    // Set and update `modkey + num` shortcut ui
    sorted_model.connect_items_changed({
        let mod_str = custom_binds.shortcut_modifier_str.clone();
        move |myself, _, removed, added| {
            // Early exit if nothing changed
            if added == 0 && removed == 0 {
                return;
            }
            let mut added_index = 0;
            for i in 0..myself.n_items() {
                if let Some(item) = myself.item(i).and_downcast::<SherlockRow>() {
                    if item.imp().shortcut.get() {
                        if let Some(shortcut_holder) = item.shortcut_holder() {
                            if added_index < 5 {
                                added_index +=
                                    shortcut_holder.apply_shortcut(added_index + 1, &mod_str);
                            } else {
                                shortcut_holder.remove_shortcut();
                            }
                        }
                    }
                }
            }
        }
    });

    let selection = SingleSelection::new(Some(sorted_model));
    results.set_model(Some(&selection));

    let tiles = Tile::pipe_data(&pipe_content, &method);

    for item in tiles.iter() {
        model.append(item);
    }
    results.set_model(Some(&selection));
    results.set_factory(Some(&factory));

    let (_, n_items) = selection.focus_first();
    if n_items > 0 {
        results.scroll_to(0, ListScrollFlags::NONE, None);
    }

    // Disable status-bar
    CONFIG.get().map(|c| {
        if !c.appearance.status_bar {
            let n: Option<GtkBox> = builder.object("status-bar");
            n.map(|n| n.set_visible(false));
        }
    });

    search_icon_holder.append(&overlay);

    let search_bar: Entry = builder.object("search-bar").unwrap_or_default();
    let result_viewport: ScrolledWindow = builder.object("scrolled-window").unwrap_or_default();
    let ui = PipeUI {
        result_viewport: result_viewport.downgrade(),
        results: results.downgrade(),
        search_bar: search_bar.downgrade(),
        search_icon_holder: search_icon_holder.downgrade(),
        selection: selection.downgrade(),
        filter: filter.downgrade(),
        sorter: sorter.downgrade(),
        binds: custom_binds,
    };
    CONFIG.get().map(|c| {
        ui.result_viewport.upgrade().map(|viewport| {
            viewport.set_size_request((c.appearance.width as f32 * 0.4) as i32, 10);
        });
        ui.search_icon_holder
            .upgrade()
            .map(|holder| holder.set_visible(c.appearance.search_icon));
        search_icon.set_pixel_size(c.appearance.icon_size);
        search_icon_back.set_pixel_size(c.appearance.icon_size);
    });

    (search_text, vbox, ui)
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
fn make_filter(search_text: &Rc<RefCell<String>>) -> CustomFilter {
    CustomFilter::new({
        let search_text = Rc::clone(search_text);
        move |entry| {
            let item = entry.downcast_ref::<SherlockRow>().unwrap();
            let current_text = search_text.borrow().clone();
            item.search().fuzzy_match(&current_text)
        }
    })
}
fn make_sorter(search_text: &Rc<RefCell<String>>) -> CustomSorter {
    CustomSorter::new({
        let search_text = Rc::clone(search_text);
        move |item_a, item_b| {
            let search_text = search_text.borrow();

            let item_a = item_a.downcast_ref::<SherlockRow>().unwrap();
            let item_b = item_b.downcast_ref::<SherlockRow>().unwrap();

            let (prio_a, prio_b) = if !search_text.is_empty() {
                (
                    levenshtein(&search_text, &item_a.search()) as f32,
                    levenshtein(&search_text, &item_b.search()) as f32,
                )
            } else {
                (0.0, 0.0)
            };

            prio_a.total_cmp(&prio_b).into()
        }
    })
}
fn nav_event(
    selection: WeakRef<SingleSelection>,
    results: WeakRef<ListView>,
    search_bar: WeakRef<Entry>,
    custom_binds: ConfKeys,
) {
    let event_controller = EventControllerKey::new();
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed({
        let search_bar = search_bar.clone();
        fn move_prev(selection: &WeakRef<SingleSelection>, results: &WeakRef<ListView>) {
            let (new_index, n_items) = selection
                .upgrade()
                .map_or((0, 0), |results| results.focus_prev());
            results.upgrade().map(|results| {
                if n_items > 0 {
                    results.scroll_to(new_index, ListScrollFlags::NONE, None)
                }
            });
        }
        fn move_next(selection: &WeakRef<SingleSelection>, results: &WeakRef<ListView>) {
            let (new_index, n_items) = selection
                .upgrade()
                .map_or((0, 0), |results| results.focus_next());
            results.upgrade().map(|results| {
                if new_index < n_items {
                    results.scroll_to(new_index, ListScrollFlags::NONE, None)
                }
            });
        }
        move |_, key, i, modifiers| {
            match key {
                k if Some(k) == custom_binds.prev
                    && custom_binds
                        .prev_mod
                        .map_or(true, |m| modifiers.contains(m)) =>
                {
                    move_prev(&selection, &results);
                    return true.into();
                }
                k if Some(k) == custom_binds.next
                    && custom_binds
                        .next_mod
                        .map_or(true, |m| modifiers.contains(m)) =>
                {
                    move_next(&selection, &results);
                    return true.into();
                }
                gdk::Key::Up => {
                    move_prev(&selection, &results);
                    return true.into();
                }
                gdk::Key::Down => {
                    move_next(&selection, &results);
                    return true.into();
                }
                gdk::Key::BackSpace => {
                    if custom_binds
                        .shortcut_modifier
                        .map_or(false, |modifier| modifiers.contains(modifier))
                    {
                        search_bar.upgrade().map(|entry| entry.set_text(""));
                        // Focus first item and check for overflow
                        if let Some((_, n_items)) =
                            selection.upgrade().map(|results| results.focus_first())
                        {
                            if n_items > 0 {
                                results.upgrade().map(|results| {
                                    results.scroll_to(0, ListScrollFlags::NONE, None)
                                });
                            }
                        }
                    }
                }
                gdk::Key::Return => {
                    if let Some(upgr) = selection.upgrade() {
                        if let Some(row) = upgr.selected_item().and_downcast::<SherlockRow>() {
                            row.emit_by_name::<()>("row-should-activate", &[]);
                        }
                    }
                }
                _ if key.to_unicode().and_then(|c| c.to_digit(10)).is_some() => {
                    if custom_binds
                        .shortcut_modifier
                        .map_or(false, |modifier| modifiers.contains(modifier))
                    {
                        if let Some(index) = key
                            .name()
                            .and_then(|name| name.parse::<u32>().ok().map(|v| v - 1))
                        {
                            selection.upgrade().map(|r| r.execute_by_index(index));
                            return true.into();
                        }
                    }
                }
                // Pain - solution for shift-tab since gtk handles it as an individual event
                _ if i == 23 && modifiers.contains(ModifierType::SHIFT_MASK) => {
                    let shift = Some(ModifierType::SHIFT_MASK);
                    let tab = Some(Key::Tab);
                    if custom_binds.prev_mod == shift && custom_binds.prev == tab {
                        move_prev(&selection, &results);
                        return true.into();
                    } else if custom_binds.next_mod == shift && custom_binds.next == tab {
                        move_next(&selection, &results);
                        return true.into();
                    }
                }
                _ => (),
            }
            false.into()
        }
    });

    search_bar
        .upgrade()
        .map(|entry| entry.add_controller(event_controller));
}

fn change_event(
    search_bar: WeakRef<Entry>,
    results: WeakRef<ListView>,
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    selection: WeakRef<SingleSelection>,
    search_query: &Rc<RefCell<String>>,
) -> Option<()> {
    let search_bar = search_bar.upgrade()?;
    search_bar.connect_changed({
        let search_query_clone = Rc::clone(search_query);

        move |search_bar| {
            let current_text = search_bar.text().to_string();
            // logic to switch to search mode with respective icons
            if current_text.len() == 1 {
                let _ = search_bar.activate_action("win.switch-mode", Some(&"search".to_variant()));
            } else if current_text.len() == 0 {
                let _ = search_bar.activate_action("win.switch-mode", Some(&"all".to_variant()));
            }

            // filter and sort
            *search_query_clone.borrow_mut() = current_text;
            filter
                .upgrade()
                .map(|filter| filter.changed(gtk4::FilterChange::Different));
            sorter
                .upgrade()
                .map(|sorter| sorter.changed(gtk4::SorterChange::Different));
            // focus first item
            if let Some((_, n_items)) = selection.upgrade().map(|results| results.focus_first()) {
                results.upgrade().map(|results| {
                    if n_items > 0 {
                        results.scroll_to(0, ListScrollFlags::NONE, None);
                    }
                });
            }
        }
    });
    Some(())
}
