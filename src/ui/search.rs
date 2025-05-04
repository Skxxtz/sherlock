use futures::future::join_all;
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::{glib::WeakRef, ActionEntry, ListStore};
use gtk4::{
    self,
    gdk::{self, Key, ModifierType},
    prelude::*,
    Builder, CustomFilter, CustomSorter, EventControllerKey, FilterListModel, Image,
    ListScrollFlags, ListView, Overlay, SignalListItemFactory, SingleSelection, SortListModel,
    Spinner,
};
use gtk4::{glib, ApplicationWindow, Entry};
use gtk4::{Box as GtkBox, Label, ScrolledWindow};
use levenshtein::levenshtein;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::tiles::util::SherlockSearch;
use super::util::*;
use crate::CONFIG;
use crate::{g_subclasses::sherlock_row::SherlockRow, loader::Loader};

#[derive(Clone, Debug)]
pub struct SearchHandler {
    pub model: Option<WeakRef<ListStore>>,
    pub modes: Rc<RefCell<HashMap<String, Option<String>>>>,
    pub task: Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    pub error_model: WeakRef<ListStore>,
}
impl SearchHandler {
    pub fn new(model: WeakRef<ListStore>, error_model: WeakRef<ListStore>) -> Self {
        Self {
            model: Some(model),
            modes: Rc::new(RefCell::new(HashMap::new())),
            task: Rc::new(RefCell::new(None)),
            error_model,
        }
    }
    pub fn empty(error_model: WeakRef<ListStore>) -> Self {
        Self {
            model: None,
            modes: Rc::new(RefCell::new(HashMap::new())),
            task: Rc::new(RefCell::new(None)),
            error_model,
        }
    }
    pub fn clear(&self) {
        if let Some(model) = self.model.as_ref().and_then(|m| m.upgrade()) {
            model.remove_all();
        }
    }
    pub fn populate(&self) {
        // clear potentially stuck rows
        self.clear();

        // load launchers
        let (launchers, n) = match Loader::load_launchers().map_err(|e| e.tile("ERROR")) {
            Ok(r) => r,
            Err(e) => {
                if let Some(model) = self.error_model.upgrade() {
                    model.append(&e);
                }
                return;
            }
        };
        if let Some(model) = self.error_model.upgrade() {
            n.into_iter()
                .map(|n| n.tile("WARNING"))
                .for_each(|row| model.append(&row));
        }

        if let Some(model) = self.model.as_ref().and_then(|m| m.upgrade()) {
            let mut holder: HashMap<String, Option<String>> = HashMap::new();
            let rows: Vec<WeakRef<SherlockRow>> = launchers
                .into_iter()
                .map(|launcher| {
                    let patch = launcher.get_patch();
                    if let Some(alias) = &launcher.alias {
                        holder.insert(format!("{} ", alias), launcher.name);
                    }
                    patch
                })
                .flatten()
                .map(|row| {
                    model.append(&row);
                    row.downgrade()
                })
                .collect();
            update_async(rows, &self.task, String::new());
            *self.modes.borrow_mut() = holder;
        }
    }
}

#[allow(dead_code)]
struct SearchUI {
    result_viewport: WeakRef<ScrolledWindow>,
    results: WeakRef<ListView>,
    // will be later used for split view to display information about apps/commands
    preview_box: WeakRef<GtkBox>,
    search_bar: WeakRef<Entry>,
    search_icon_holder: WeakRef<GtkBox>,
    mode_title: WeakRef<Label>,
    spinner: WeakRef<Spinner>,
    selection: WeakRef<SingleSelection>,
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    binds: ConfKeys,
}
fn update_async(
    update_tiles: Vec<WeakRef<SherlockRow>>,
    current_task: &Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    keyword: String,
) {
    let current_task_clone = Rc::clone(current_task);
    if let Some(t) = current_task.borrow_mut().take() {
        t.abort();
    };
    let task = glib::MainContext::default().spawn_local({
        async move {
            // Set spinner active
            let spinner_row = update_tiles.get(0).cloned();
            if let Some(row) = spinner_row.as_ref().and_then(|row| row.upgrade()) {
                let _ = row.activate_action("win.spinner-mode", Some(&true.to_variant()));
            }
            // Make async tiles update concurrently
            let futures: Vec<_> = update_tiles
                .into_iter()
                .map(|row| {
                    let current_text = keyword.clone();
                    async move {
                        // Process text tile
                        if let Some(row) = row.upgrade() {
                            row.async_update(&current_text).await
                        }
                    }
                })
                .collect();

            let _ = join_all(futures).await;
            // Set spinner inactive
            if let Some(row) = spinner_row.as_ref().and_then(|row| row.upgrade()) {
                let _ = row.activate_action("win.spinner-mode", Some(&false.to_variant()));
            }
            *current_task_clone.borrow_mut() = None;
        }
    });
    *current_task.borrow_mut() = Some(task);
}
pub fn search(
    window: &ApplicationWindow,
    stack_page_ref: &Rc<RefCell<String>>,
    error_model: WeakRef<ListStore>,
) -> (GtkBox, SearchHandler) {
    // Initialize the view to show all apps
    let (search_query, mode, stack_page, ui, handler) = construct_window(error_model);
    ui.result_viewport
        .upgrade()
        .map(|view| view.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic));

    let initial_mode = mode.borrow().clone();
    let modes_clone = Rc::clone(&handler.modes);
    let mode_clone = Rc::clone(&mode);

    let search_bar = ui.search_bar.clone();
    stack_page.connect_realize(move |_| {
        search_bar
            .upgrade()
            .map(|search_bar| search_bar.grab_focus());
    });

    nav_event(
        ui.selection.clone(),
        ui.results.clone(),
        ui.search_bar.clone(),
        ui.filter.clone(),
        ui.sorter.clone(),
        ui.binds,
        stack_page_ref,
        &mode,
    );
    change_event(
        ui.search_bar.clone(),
        ui.results,
        Rc::clone(&handler.modes),
        &mode,
        ui.filter,
        ui.sorter,
        ui.selection,
        &search_query,
        &handler.task,
    );

    // Improved mode selection
    let search_bar = ui.search_bar.clone();
    let mode_action = ActionEntry::builder("switch-mode")
        .parameter_type(Some(&String::static_variant_type()))
        .state(initial_mode.to_variant())
        .activate(move |_, action, parameter| {
            let state = action.state().and_then(|s| s.get::<String>());
            let parameter = parameter.and_then(|p| p.get::<String>());

            if let (Some(mut state), Some(mut parameter)) = (state, parameter) {
                match parameter.as_str() {
                    "search" => {
                        ui.search_icon_holder
                            .upgrade()
                            .map(|holder| holder.set_css_classes(&["back"]));
                        ui.mode_title
                            .upgrade()
                            .map(|title| title.set_text("Search"));
                    }
                    _ => {
                        parameter.push_str(" ");
                        let mode_name = modes_clone.borrow().get(&parameter).cloned();
                        match mode_name {
                            Some(name) => {
                                ui.search_icon_holder
                                    .upgrade()
                                    .map(|holder| holder.set_css_classes(&["back"]));
                                ui.mode_title.upgrade().map(|title| {
                                    title.set_text(name.as_deref().unwrap_or_default())
                                });
                                *mode_clone.borrow_mut() = parameter.clone();
                                state = parameter;
                            }
                            _ => {
                                ui.search_icon_holder
                                    .upgrade()
                                    .map(|holder| holder.set_css_classes(&["search"]));
                                ui.mode_title.upgrade().map(|title| title.set_text("All"));
                                parameter = String::from("all ");
                                *mode_clone.borrow_mut() = parameter.clone();
                                state = parameter;
                            }
                        }
                        let search_bar = search_bar.clone();
                        glib::idle_add_local(move || {
                            // to trigger homescreen rebuild
                            search_bar.upgrade().map(|entry| {
                                entry.set_text("");
                            });
                            glib::ControlFlow::Break
                        });
                        action.set_state(&state.to_variant());
                    }
                }
            }
        })
        .build();

    // Spinner action
    let spinner_clone = ui.spinner;
    let action_spinner = ActionEntry::builder("spinner-mode")
        .parameter_type(Some(&bool::static_variant_type()))
        .activate(move |_, _, parameter| {
            let parameter = parameter.and_then(|p| p.get::<bool>());
            parameter.map(|p| {
                if p {
                    spinner_clone
                        .upgrade()
                        .map(|spinner| spinner.set_css_classes(&["spinner-appear"]));
                } else {
                    spinner_clone
                        .upgrade()
                        .map(|spinner| spinner.set_css_classes(&["spinner-disappear"]));
                };
                spinner_clone
                    .upgrade()
                    .map(|spinner| spinner.set_spinning(p));
            });
        })
        .build();

    let search_bar = ui.search_bar.clone();
    let action_clear_win = ActionEntry::builder("clear-search")
        .activate(move |_: &ApplicationWindow, _, _| {
            let search_bar = search_bar.clone();
            glib::idle_add_local(move || {
                search_bar.upgrade().map(|entry| entry.set_text(""));
                glib::ControlFlow::Break
            });
        })
        .build();
    window.add_action_entries([mode_action, action_clear_win, action_spinner]);

    return (stack_page, handler);
}

fn construct_window(
    error_model: WeakRef<ListStore>,
) -> (
    Rc<RefCell<String>>,
    Rc<RefCell<String>>,
    GtkBox,
    SearchUI,
    SearchHandler,
) {
    // Collect Modes
    let custom_binds = ConfKeys::new();
    let original_mode = CONFIG
        .get()
        .and_then(|c| c.behavior.sub_menu.as_deref())
        .unwrap_or("all");
    let mode = Rc::new(RefCell::new(original_mode.to_string()));
    let search_text = Rc::new(RefCell::new(String::from("")));

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
    let filter = make_filter(&search_text, &mode);
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

    // Add tiles to the view and create modes
    let handler = SearchHandler::new(model.downgrade(), error_model);
    handler.populate();

    results.set_model(Some(&selection));
    results.set_factory(Some(&factory));

    let (_, n_items) = selection.focus_first();
    if n_items > 0 {
        results.scroll_to(0, ListScrollFlags::NONE, None);
    }

    search_icon_holder.append(&overlay);

    let spinner: Spinner = builder.object("status-bar-spinner").unwrap_or_default();
    let preview_box: GtkBox = builder.object("preview_box").unwrap_or_default();
    let search_bar: Entry = builder.object("search-bar").unwrap_or_default();
    let result_viewport: ScrolledWindow = builder.object("scrolled-window").unwrap_or_default();
    let mode_title: Label = builder.object("category-type-label").unwrap_or_default();
    let ui = SearchUI {
        result_viewport: result_viewport.downgrade(),
        results: results.downgrade(),
        preview_box: preview_box.downgrade(),
        search_bar: search_bar.downgrade(),
        search_icon_holder: search_icon_holder.downgrade(),
        mode_title: mode_title.downgrade(),
        spinner: spinner.downgrade(),
        selection: selection.downgrade(),
        filter: filter.downgrade(),
        sorter: sorter.downgrade(),
        binds: custom_binds,
    };
    CONFIG.get().map(|c| {
        // disable status bar
        if !c.appearance.status_bar {
            let n: Option<GtkBox> = builder.object("status-bar");
            n.map(|n| n.set_visible(false));
        }
        // set sizes
        ui.result_viewport.upgrade().map(|viewport| {
            viewport.set_size_request((c.appearance.width as f32 * 0.4) as i32, 10);
        });
        ui.search_icon_holder
            .upgrade()
            .map(|holder| holder.set_visible(c.appearance.search_icon));
        search_icon.set_pixel_size(c.appearance.icon_size);
        search_icon_back.set_pixel_size(c.appearance.icon_size);
    });

    (search_text, mode, vbox, ui, handler)
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
fn make_filter(search_text: &Rc<RefCell<String>>, mode: &Rc<RefCell<String>>) -> CustomFilter {
    CustomFilter::new({
        let search_text = Rc::clone(search_text);
        let search_mode = Rc::clone(mode);
        move |entry| {
            let item = entry.downcast_ref::<SherlockRow>().unwrap();
            let (home, only_home) = item.home();

            let mode = search_mode.borrow().trim().to_string();
            let current_text = search_text.borrow().clone();
            let is_home = current_text.is_empty() && mode == "all";

            let update_res = item.update(&current_text);

            if is_home {
                if home || only_home {
                    return true;
                }
                return false;
            } else {
                let alias = item.alias();
                let priority = item.priority();
                if mode != "all" {
                    if only_home || mode != alias {
                        return false;
                    }
                    if current_text.is_empty() {
                        return true;
                    }
                } else if priority <= 1.0 {
                    return false;
                }
                if item.is_keyword_aware() {
                    return true;
                }

                if update_res {
                    return true;
                }
                item.search().fuzzy_match(&current_text)
            }
        }
    })
}
fn make_sorter(search_text: &Rc<RefCell<String>>) -> CustomSorter {
    CustomSorter::new({
        let search_text = Rc::clone(search_text);
        fn search_score(query: &str, match_in: &str) -> f32 {
            if match_in.len() == 0 {
                return 0.0;
            }
            let distance = levenshtein(query, match_in) as f32;
            let normed = (distance / match_in.len() as f32).clamp(0.2, 1.0);
            let starts_with = if match_in.starts_with(query) {
                -0.2
            } else {
                0.0
            };
            normed + starts_with
        }

        fn make_prio(prio: f32, query: &str, match_in: &str) -> f32 {
            let score = search_score(query, match_in);
            // shift counts 3 to right; 1.34 → 1.00034 to make room for levenshtein
            let counters = prio.fract() / 1000.0;
            prio.trunc() + (counters + score).min(0.99)
        }
        move |item_a, item_b| {
            let search_text = search_text.borrow();

            let item_a = item_a.downcast_ref::<SherlockRow>().unwrap();
            let item_b = item_b.downcast_ref::<SherlockRow>().unwrap();

            let mut priority_a = item_a.priority();
            let mut priority_b = item_b.priority();

            if !search_text.is_empty() {
                priority_a = make_prio(item_a.priority(), &search_text, &item_a.search());
                priority_b = make_prio(item_b.priority(), &search_text, &item_b.search());
            }

            priority_a.total_cmp(&priority_b).into()
        }
    })
}

fn nav_event(
    selection: WeakRef<SingleSelection>,
    results: WeakRef<ListView>,
    search_bar: WeakRef<Entry>,
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    custom_binds: ConfKeys,
    stack_page: &Rc<RefCell<String>>,
    current_mode: &Rc<RefCell<String>>,
) {
    let stack_page = Rc::clone(stack_page);
    let event_controller = EventControllerKey::new();
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed({
        let search_bar = search_bar.clone();
        let current_mode = Rc::clone(current_mode);
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
            if stack_page.borrow().as_str() != "search-page" {
                return false.into();
            };
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
                    let mut ctext = search_bar
                        .upgrade()
                        .map_or(String::new(), |entry| entry.text().to_string());
                    if custom_binds
                        .shortcut_modifier
                        .map_or(false, |modifier| modifiers.contains(modifier))
                    {
                        search_bar.upgrade().map(|entry| entry.set_text(""));
                        ctext.clear();
                    }
                    if ctext.is_empty() && current_mode.borrow().as_str() != "all" {
                        let _ = search_bar.upgrade().map(|entry| {
                            let _ =
                                entry.activate_action("win.switch-mode", Some(&"all".to_variant()));
                            // apply filter and sorter
                            filter
                                .upgrade()
                                .map(|filter| filter.changed(gtk4::FilterChange::Different));
                            sorter
                                .upgrade()
                                .map(|sorter| sorter.changed(gtk4::SorterChange::Different));
                        });
                    }
                    // Focus first item and check for overflow
                    if let Some((_, n_items)) =
                        selection.upgrade().map(|results| results.focus_first())
                    {
                        if n_items > 0 {
                            results
                                .upgrade()
                                .map(|results| results.scroll_to(0, ListScrollFlags::NONE, None));
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
    modes: Rc<RefCell<HashMap<String, Option<String>>>>,
    mode: &Rc<RefCell<String>>,
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    selection: WeakRef<SingleSelection>,
    search_query: &Rc<RefCell<String>>,
    current_task: &Rc<RefCell<Option<glib::JoinHandle<()>>>>,
) -> Option<()> {
    let search_bar = search_bar.upgrade()?;
    search_bar.connect_changed({
        let mode_clone = Rc::clone(mode);
        let search_query_clone = Rc::clone(search_query);
        let current_task = Rc::clone(current_task);

        move |search_bar| {
            let mut current_text = search_bar.text().to_string();
            // logic to switch to search mode with respective icons
            if current_text.len() == 1 {
                let _ = search_bar.activate_action("win.switch-mode", Some(&"search".to_variant()));
            } else if current_text.len() == 0 && mode_clone.borrow().as_str().trim() == "all" {
                let _ = search_bar.activate_action("win.switch-mode", Some(&"all".to_variant()));
            }
            let trimmed = current_text.trim();
            if !trimmed.is_empty() && modes.borrow().contains_key(&current_text) {
                // Logic to apply modes
                let _ = search_bar.activate_action("win.switch-mode", Some(&trimmed.to_variant()));
                current_text.clear();
            }
            *search_query_clone.borrow_mut() = current_text.clone();
            // filter and sort
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
                let weaks: Vec<WeakRef<SherlockRow>> = if let Some(selection) = selection.upgrade()
                {
                    (0..n_items)
                        .filter_map(|i| {
                            selection
                                .item(i)
                                .and_downcast::<SherlockRow>()
                                .map(|row| row.downgrade())
                        })
                        .collect()
                } else {
                    vec![]
                };
                update_async(weaks, &current_task, current_text);
            }
        }
    });
    Some(())
}
