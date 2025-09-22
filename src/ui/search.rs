use gio::{
    glib::{SignalHandlerId, WeakRef},
    ActionEntry, ListStore,
};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{
    self,
    gdk::{Key, ModifierType},
    prelude::*,
    CustomFilter, CustomSorter, EventControllerKey, FilterListModel, ListView, Overlay,
    SignalListItemFactory, SingleSelection, SortListModel, Widget,
};
use gtk4::{glib, ApplicationWindow, Entry};
use levenshtein::levenshtein;
use simd_json::prelude::ArrayTrait;
use std::collections::HashMap;
use std::rc::Rc;
use std::{cell::RefCell, f32};

use super::context::make_context;
use super::util::*;
use crate::{
    api::{api::SherlockAPI, call::ApiCall, server::SherlockServer},
    g_subclasses::{action_entry::ContextAction, sherlock_row::SherlockRow, tile_item::TileItem},
    launcher::{utils::HomeType, Launcher},
    prelude::{IconComp, SherlockNav, SherlockSearch, ShortCut},
    ui::{g_templates::SearchUiObj, key_actions::KeyActions},
    utils::config::OtherDefaults,
};
use crate::{
    g_subclasses::sherlock_row::SherlockRowBind,
    utils::{config::ConfigGuard, errors::SherlockError},
};

#[sherlock_macro::timing(name = "Search Window Creation")]
pub fn search(
    window: &ApplicationWindow,
    stack_page_ref: &Rc<RefCell<String>>,
    sherlock: Rc<RefCell<SherlockAPI>>,
) -> Result<Overlay, SherlockError> {
    let error_model = sherlock.borrow().errors.clone().unwrap_or_default();

    // Initialize the view to show all apps
    let (search_query, stack_page, ui, handler, context) = construct_window(error_model)?;
    let imp = ui.imp();
    imp.result_viewport
        .set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

    // Add support for custom binds
    let custom_controller = EventControllerKey::new();
    custom_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let custom_handler = Rc::new(RefCell::new(UserBindHandler::new(
        custom_controller.downgrade(),
    )));

    {
        let mut sherlock = sherlock.borrow_mut();
        sherlock.search_ui = Some(ui.downgrade());
        sherlock.search_handler = Some(handler.clone());
    }

    // Mode setup - used to decide which tiles should be shown
    let initial_mode = handler.mode.borrow().clone();

    // Initial setup on show
    stack_page.connect_map({
        let search_bar = imp.search_bar.downgrade();
        move |_| {
            // Focus search bar as soon as it's visible
            if let Some(search_bar) = search_bar.upgrade() {
                search_bar.grab_focus();
            }
        }
    });
    if let Some(model) = handler.model.as_ref().and_then(|tmp| tmp.upgrade()) {
        model.connect_items_changed({
            let results = imp.results.downgrade();
            let context_model = context.model.clone();
            let current_mode = Rc::clone(&handler.mode);
            let custom_handler = Rc::clone(&custom_handler);
            let modstr = handler.binds.shortcut_modifier_str.clone();
            let (apply_animation, num_shortcuts) = ConfigGuard::read()
                .map(|c| {
                    (
                        c.behavior.animate,
                        c.appearance.num_shortcuts.clamp(0, 10) as i32,
                    )
                })
                .unwrap_or((false, 5));
            move |_myself, _position, _removed, added| {
                if added == 0 {
                    return;
                }
                // Show or hide context menu shortcuts whenever stack shows
                if let Some(results) = results.upgrade() {
                    results.focus_first(
                        Some(&context_model),
                        Some(current_mode.clone()),
                        Some(custom_handler.clone()),
                    );
                    if let Some(selection) = results.model().and_downcast::<SingleSelection>() {
                        let mut current = 1;
                        for i in 0..selection.n_items() {
                            if let Some(item) = selection.item(i).and_downcast::<TileItem>() {
                                if apply_animation {
                                    if let Some(row) = item.parent().upgrade() {
                                        row.add_css_class("animate");
                                    }
                                }
                                if let Some(shortcut) = item.shortcut() {
                                    if current < num_shortcuts + 1 {
                                        current += shortcut.apply_shortcut(current, &modstr);
                                    } else {
                                        shortcut.remove_shortcut();
                                    }
                                }
                            }
                        }
                    };
                }
            }
        });
    }

    nav_event(
        imp.results.downgrade(),
        imp.search_bar.downgrade(),
        handler.filter.clone(),
        handler.sorter.clone(),
        handler.binds.clone(),
        stack_page_ref,
        &Rc::clone(&handler.mode),
        context.clone(),
        Rc::clone(&custom_handler),
    );
    change_event(
        imp.search_bar.downgrade(),
        imp.results.downgrade(),
        Rc::clone(&handler.modes),
        &Rc::clone(&handler.mode),
        &search_query,
    );

    // Improved mode selection
    let mode_action = ActionEntry::builder("switch-mode")
        .parameter_type(Some(&String::static_variant_type()))
        .state(initial_mode.to_variant())
        .activate({
            let mode_clone = Rc::clone(&handler.mode);
            let modes_clone = Rc::clone(&handler.modes);
            let ui = ui.downgrade();
            move |_, action, parameter| {
                let state = action.state().and_then(|s| s.get::<String>());
                let parameter = parameter.and_then(|p| p.get::<String>());
                let ui = match ui.upgrade() {
                    Some(ui) => ui,
                    _ => return,
                };
                let imp = ui.imp();

                if let (Some(mut state), Some(mut parameter)) = (state, parameter) {
                    match parameter.as_str() {
                        "search" => {
                            imp.search_icon_holder.set_css_classes(&["back"]);
                            imp.mode_title.set_text("Search");
                        }
                        _ => {
                            parameter.push_str(" ");
                            match modes_clone.borrow().get(&parameter) {
                                Some(launchers) if launchers.len() > 0 => {
                                    if let Some(launcher) = launchers.get(0) {
                                        imp.search_icon_holder.set_css_classes(&["back"]);
                                        if let Some(name) = &launcher.name {
                                            imp.mode_title.set_text(name);
                                        }

                                        *mode_clone.borrow_mut() = parameter.clone();
                                        state = parameter;
                                    }
                                }
                                _ => {
                                    imp.search_icon_holder.set_css_classes(&["search"]);
                                    imp.mode_title.set_text("All");

                                    parameter = String::from("all ");
                                    *mode_clone.borrow_mut() = parameter.clone();
                                    state = parameter;
                                }
                            }
                            action.set_state(&state.to_variant());
                        }
                    }
                }
            }
        })
        .build();

    // Action to update filter and sorter
    let sorter_actions = ActionEntry::builder("update-items")
        .parameter_type(Some(&bool::static_variant_type()))
        .activate({
            let filter = handler.filter.clone();
            let sorter = handler.sorter.clone();
            let results = imp.results.clone();
            let current_task = handler.task.clone();
            let current_text = search_query.clone();
            let context_model = context.model.clone();
            let current_mode = Rc::clone(&handler.mode);
            let custom_handler = Rc::clone(&custom_handler);
            let modstr = handler.binds.shortcut_modifier_str.clone();
            let num_shortcuts = ConfigGuard::read()
                .map_or(5, |c| c.appearance.num_shortcuts)
                .clamp(0, 10) as i32;
            move |_: &ApplicationWindow, _, parameter| {
                if let Some(focus_first) = parameter.and_then(|p| p.get::<bool>()) {
                    filter
                        .upgrade()
                        .map(|filter| filter.changed(gtk4::FilterChange::Different));
                    sorter
                        .upgrade()
                        .map(|sorter| sorter.changed(gtk4::SorterChange::Different));
                    let weaks = results.get_weaks().unwrap_or(vec![]);
                    if focus_first {
                        results.focus_first(
                            Some(&context_model),
                            Some(current_mode.clone()),
                            Some(custom_handler.clone()),
                        );
                        if let Some(selection) = results.model().and_downcast::<SingleSelection>() {
                            let mut current = 1;
                            for i in 0..selection.n_items() {
                                if let Some(item) = selection.item(i).and_downcast::<TileItem>() {
                                    if let Some(shortcut) = item.shortcut() {
                                        if current < num_shortcuts + 1 {
                                            current += shortcut.apply_shortcut(current, &modstr);
                                        } else {
                                            shortcut.remove_shortcut();
                                        }
                                    }
                                }
                            }
                        };
                    }
                    if !weaks.is_empty() {
                        update_async(weaks, &current_task, current_text.borrow().clone());
                    }
                }
            }
        })
        .build();

    let action_clear_win = ActionEntry::builder("clear-search")
        .parameter_type(Some(&bool::static_variant_type()))
        .activate({
            let search_bar = imp.search_bar.downgrade();
            let mode = Rc::clone(&handler.mode);
            move |_: &ApplicationWindow, _, parameter| {
                let clear_mode = parameter.and_then(|p| p.get::<bool>()).unwrap_or_default();
                if clear_mode {
                    *mode.borrow_mut() = "all".to_string();
                }
                let search_bar = search_bar.clone();
                glib::idle_add_local(move || {
                    if let Some(entry) = search_bar.upgrade() {
                        entry.set_text("");
                    }
                    glib::ControlFlow::Break
                });
            }
        })
        .build();
    window.add_action_entries([mode_action, action_clear_win, sorter_actions]);

    return Ok(stack_page);
}

fn construct_window(
    error_model: WeakRef<ListStore>,
) -> Result<
    (
        Rc<RefCell<String>>,
        Overlay,
        SearchUiObj,
        SearchHandler,
        ContextUI<ContextAction>,
    ),
    SherlockError,
> {
    // Collect Modes
    let custom_binds = ConfKeys::new();
    let config = ConfigGuard::read()?;
    let original_mode = config.runtime.sub_menu.as_deref().unwrap_or("all");
    let mode = Rc::new(RefCell::new(original_mode.to_string()));
    let search_text = Rc::new(RefCell::new(String::from("")));

    // Initialize the builder with the correct path
    let ui = SearchUiObj::new();
    let imp = ui.imp();

    let (context, revealer) = make_context();
    let main_overlay = Overlay::new();
    main_overlay.set_child(Some(&ui));
    main_overlay.add_overlay(&revealer);

    imp.search_bar
        .set_placeholder_text(Some(&config.appearance.placeholder));

    // Update the search icon
    imp.search_icon.set_icon(
        Some(&config.search_bar_icon.icon),
        None,
        Some(&OtherDefaults::search_icon()),
    );
    imp.search_icon.set_pixel_size(config.search_bar_icon.size);

    // Create the back arrow
    imp.search_icon_back.set_icon(
        Some(&config.search_bar_icon.icon_back),
        None,
        Some(&OtherDefaults::search_icon_back()),
    );
    imp.search_icon_back
        .set_pixel_size(config.search_bar_icon.size);

    // Setup model and factory
    let model = ListStore::new::<TileItem>();
    let factory = make_factory(search_text.clone());
    imp.results.set_factory(Some(&factory));

    // Setup selection
    let sorter = make_sorter(&search_text);
    let filter = make_filter(&search_text, &mode);
    let filter_model = FilterListModel::new(Some(model.clone()), Some(filter.clone()));
    let sorted_model = SortListModel::new(Some(filter_model), Some(sorter.clone()));

    let selection = SingleSelection::new(Some(sorted_model));
    imp.results.set_model(Some(&selection));

    imp.results.set_model(Some(&selection));
    imp.results.set_factory(Some(&factory));

    let handler = SearchHandler::new(
        model.downgrade(),
        mode,
        error_model,
        filter.downgrade(),
        sorter.downgrade(),
        imp.results.get().upcast::<Widget>().downgrade(),
        custom_binds,
    );

    if config.expand.enable {
        imp.result_viewport
            .set_max_content_height(config.appearance.height);
        imp.result_viewport.set_propagate_natural_height(true);
    }

    // enable or disable search bar icons
    imp.search_icon_holder
        .set_visible(config.search_bar_icon.enable);

    Ok((search_text, main_overlay, ui, handler, context))
}
fn make_factory(search_text: Rc<RefCell<String>>) -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Item must be a ListItem");
        let container = SherlockRow::new();
        item.set_child(Some(&container));
    });
    factory.connect_bind(move |_, item| {
        let item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Item mut be a ListItem");

        let row = item
            .child()
            .and_downcast::<SherlockRow>()
            .expect("Child must be a SherlockRow");

        let tile_item = item
            .item()
            .and_downcast::<TileItem>()
            .expect("Row should be TileItem");

        if let Some(patch) = tile_item.get_patch() {
            tile_item.replace_tile(&patch);
            tile_item.set_parent(Some(&row));
            tile_item.based_show(&search_text.borrow());
            tile_item.update(&search_text.borrow());
            tile_item.bind_signal(&row);
            row.append(&patch);
        }
    });
    factory.connect_unbind(|_, item| {
        let item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Item mut be a ListItem");
        let tile_item = item
            .item()
            .clone()
            .and_downcast::<TileItem>()
            .expect("Row should be TileItem");
        let row = item
            .child()
            .and_downcast::<SherlockRow>()
            .expect("Child must be a SherlockRow");

        // Remove any pending animations
        row.remove_css_class("animate");

        while let Some(child) = row.first_child() {
            row.remove(&child);
        }

        if let Some(shortcut) = tile_item.shortcut() {
            shortcut.remove_shortcut();
        }
        tile_item.set_parent(None);
    });
    factory
}
fn make_filter(search_text: &Rc<RefCell<String>>, mode: &Rc<RefCell<String>>) -> CustomFilter {
    CustomFilter::new({
        let search_text = Rc::clone(search_text);
        let search_mode = Rc::clone(mode);
        move |entry| {
            let item = entry.downcast_ref::<TileItem>().unwrap();
            let imp = item.imp();
            let launcher = imp.launcher.borrow();
            let home = launcher.home;

            // Remove any pending animations
            if let Some(p) = item.parent().upgrade() {
                p.remove_css_class("animate");
            }

            let mode = search_mode.borrow().trim().to_string();
            let current_text = search_text.borrow().clone();
            let is_home = current_text.is_empty() && mode == "all";

            let update_res = item.based_show(&search_text.borrow());
            item.update(&search_text.borrow());

            if home == HomeType::Persist {
                return true;
            }

            if is_home {
                if home != HomeType::Search {
                    return true;
                }
                false
            } else {
                let priority = item.priority();
                if mode != "all" {
                    if home == HomeType::OnlyHome || Some(mode) != launcher.alias {
                        return false;
                    }
                    if current_text.is_empty() {
                        return true;
                    }
                } else if priority < 1.0 {
                    return false;
                }
                let search = match item.search() {
                    Some(s) => s,
                    _ => return update_res,
                };

                search.fuzzy_match(&current_text)
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
            let (distance, element) = match_in
                .split(';')
                .map(|elem| (levenshtein(query, elem), elem))
                .min_by_key(|(dist, _)| *dist)
                .unwrap_or((usize::MAX, ""));

            let normed = (distance as f32 / element.len() as f32).clamp(0.2, 1.0);
            let normed = (normed * 100.0).round() / 100.0;
            let starts_with = if element.starts_with(query) {
                -0.2
            } else {
                0.0
            };
            if let Ok(var) = std::env::var("DEBUG_SEARCH") {
                if var == "true" {
                    println!(
                        "Candidate: {}\nFor Query: {}\nDistance {:?}\nNormed: {:?}\nTotal: {:?}",
                        element,
                        query,
                        distance,
                        normed,
                        normed + starts_with
                    );
                }
            }
            normed + starts_with
        }

        fn make_prio(prio: f32, query: &str, match_in: &str) -> f32 {
            let score = search_score(query, match_in);
            // shift counts 3 to right; 1.34 → 1.0034 to make room for levenshtein (2 spaces for
            // max .99)
            let counters = prio.fract() / 100.0;
            if let Ok(var) = std::env::var("DEBUG_SEARCH") {
                if var == "true" {
                    println!("Base Prio: {}", prio);
                    println!(
                        "Resulting Prio: {}\n",
                        prio.trunc() + (counters + score).min(0.99)
                    );
                }
            }
            prio.trunc() + (counters + score).min(0.99)
        }
        move |item_a, item_b| {
            let search_text = search_text.borrow().to_ascii_lowercase();

            let item_a = item_a.downcast_ref::<TileItem>().unwrap();
            let item_b = item_b.downcast_ref::<TileItem>().unwrap();

            let mut priority_a = item_a.priority();
            let mut priority_b = item_b.priority();

            if !search_text.is_empty() {
                if let Some(search) = item_a.search() {
                    priority_a = make_prio(item_a.priority(), &search_text, &search.to_lowercase());
                }
                if let Some(search) = item_b.search() {
                    priority_b = make_prio(item_b.priority(), &search_text, &search.to_lowercase());
                }
            }

            priority_a.total_cmp(&priority_b).into()
        }
    })
}

fn nav_event(
    results: WeakRef<ListView>,
    search_bar: WeakRef<Entry>,
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    binds: ConfKeys,
    stack_page: &Rc<RefCell<String>>,
    current_mode: &Rc<RefCell<String>>,
    context: ContextUI<ContextAction>,
    custom_handler: Rc<RefCell<UserBindHandler>>,
) {
    let event_controller = EventControllerKey::new();
    let custom_controller = custom_handler.borrow().get_controller();
    let stack_page = Rc::clone(stack_page);
    let multi = ConfigGuard::read().map_or(false, |c| c.runtime.multi);
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed({
        let search_bar = search_bar.clone();
        let current_mode = Rc::clone(current_mode);
        let stack_page = Rc::clone(&stack_page);
        let key_actions = KeyActions::new(results, search_bar, context, custom_handler);
        move |_, key, i, mods| {
            if stack_page.borrow().as_str() != "search-page" {
                return false.into();
            };
            let matches = |comp: Option<Key>, comp_mod: Option<ModifierType>| {
                let key_matches = Some(key) == comp;
                let mod_matches = comp_mod.map_or(false, |m| mods.contains(m));
                key_matches && mod_matches
            };

            match key {
                // Inplace execution of commands
                _ if matches(binds.exec_inplace, binds.exec_inplace_mod) => {
                    key_actions.on_return(Some(false))
                }

                // Context menu opening
                _ if matches(binds.context, binds.context_mod) => {
                    key_actions.open_context();
                }

                // Custom previous key
                Key::Up => key_actions.on_prev(),
                Key::Left if binds.use_lr_nav => key_actions.on_prev(),
                _ if matches(binds.up, binds.up_mod) => {
                    key_actions.on_prev();
                }
                _ if matches(binds.left, binds.left_mod) => {
                    key_actions.on_prev();
                }

                // Custom next key
                Key::Down => key_actions.on_next(),
                Key::Right if binds.use_lr_nav => key_actions.on_next(),
                _ if matches(binds.down, binds.down_mod) => {
                    key_actions.on_next();
                }
                _ if matches(binds.right, binds.right_mod) => {
                    key_actions.on_next();
                }

                Key::BackSpace => {
                    let mut ctext = key_actions
                        .search_bar
                        .upgrade()
                        .map_or(String::new(), |entry| entry.text().to_string());
                    if binds
                        .shortcut_modifier
                        .map_or(false, |modifier| mods.contains(modifier))
                    {
                        key_actions
                            .search_bar
                            .upgrade()
                            .map(|entry| entry.set_text(""));
                        ctext.clear();
                    }
                    if ctext.is_empty() && current_mode.borrow().as_str() != "all" {
                        let _ = key_actions.search_bar.upgrade().map(|entry| {
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
                    key_actions.focus_first(current_mode.clone());
                    return false.into();
                }
                Key::Return if multi => {
                    key_actions.on_multi_return(None);
                }
                Key::Return | Key::KP_Enter => {
                    key_actions.on_return(None);
                }
                Key::Escape if key_actions.context.open.get() => {
                    key_actions.close_context();
                }
                Key::Tab if multi && mods.is_empty() => {
                    key_actions.mark_active();
                }
                Key::Tab => {
                    return true.into();
                }
                Key::F11 => {
                    let api_call = ApiCall::SwitchMode(crate::api::api::SherlockModes::Error);
                    let _ = SherlockServer::send_action(api_call);
                }
                _ if key.to_unicode().and_then(|c| c.to_digit(10)).is_some() => {
                    if binds
                        .shortcut_modifier
                        .map_or(false, |modifier| mods.contains(modifier))
                    {
                        if let Some(index) = key.name().and_then(|name| name.parse::<u32>().ok()) {
                            let internal_index = if index == 0 { 9 } else { index - 1 };
                            println!("index: {} - {}", index, internal_index);
                            key_actions
                                .results
                                .upgrade()
                                .map(|r| r.execute_by_index(internal_index));
                        }
                    } else {
                        return false.into();
                    }
                }
                // Pain - solution for shift-tab since gtk handles it as an individual event
                _ if i == 23 && mods.contains(ModifierType::SHIFT_MASK) => {
                    let shift = Some(ModifierType::SHIFT_MASK);
                    let tab = Some(Key::Tab);
                    if binds.left_mod == shift && binds.left == tab {
                        key_actions.on_prev();
                    } else if binds.right_mod == shift && binds.right == tab {
                        key_actions.on_next();
                    }
                }
                _ => return false.into(),
            }
            true.into()
        }
    });

    if let Some(entry) = search_bar.upgrade() {
        entry.add_controller(event_controller);
        if let Some(custom_controller) = custom_controller {
            entry.add_controller(custom_controller);
        }
    }
}

fn change_event(
    search_bar: WeakRef<Entry>,
    results: WeakRef<ListView>,
    modes: Rc<RefCell<HashMap<String, Vec<Rc<Launcher>>>>>,
    mode: &Rc<RefCell<String>>,
    search_query: &Rc<RefCell<String>>,
) -> Option<()> {
    let search_bar = search_bar.upgrade()?;
    search_bar.connect_changed({
        let mode_clone = Rc::clone(mode);
        let search_query_clone = Rc::clone(search_query);

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
                let _ = search_bar.activate_action("win.clear-search", Some(&false.to_variant()));
                current_text.clear();
            }
            *search_query_clone.borrow_mut() = current_text.clone();
            // filter and sort
            if let Some(res) = results.upgrade() {
                // To reload ui according to mode
                let _ = res.activate_action("win.update-items", Some(&true.to_variant()));
            }
        }
    });
    Some(())
}

pub struct UserBindHandler {
    signal_id: Option<SignalHandlerId>,
    inner: WeakRef<EventControllerKey>,
}
impl UserBindHandler {
    pub fn new(inner: WeakRef<EventControllerKey>) -> Self {
        Self {
            signal_id: None,
            inner,
        }
    }
    pub fn set_handler(&mut self, id: SignalHandlerId) {
        if let Some(inner) = self.inner.upgrade() {
            if let Some(id) = self.signal_id.replace(id) {
                inner.disconnect(id);
            }
        }
    }
    pub fn set_binds(
        &mut self,
        binds: Option<Vec<SherlockRowBind>>,
        widget: WeakRef<TileItem>,
    ) -> Option<SignalHandlerId> {
        let binds = binds?;
        if binds.is_empty() {
            return None;
        }
        let inner = self.inner.upgrade()?;

        inner.connect_key_pressed({
            move |_, key, _, mods| {
                if let Some(bind) = binds
                    .iter()
                    .find(|s| s.key == Some(key) && mods.contains(s.modifier))
                {
                    let exit: u8 = match bind.exit {
                        Some(false) => 1,
                        Some(true) => 2,
                        _ => 0,
                    };
                    if let Some(row) = widget.upgrade().and_then(|tile| tile.parent().upgrade()) {
                        row.emit_by_name::<()>("row-should-activate", &[&exit, &bind.callback]);
                        return true.into();
                    }
                };
                false.into()
            }
        });
        None
    }
    pub fn get_controller(&self) -> Option<EventControllerKey> {
        self.inner.upgrade()
    }
}
