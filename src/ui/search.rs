use gio::{
    glib::{SignalHandlerId, WeakRef},
    ActionEntry, ListStore,
};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{
    self, prelude::*, CustomFilter, CustomSorter, EventControllerKey, FilterListModel, Overlay,
    SignalListItemFactory, SingleSelection, SortListModel, Widget,
};
use gtk4::{glib, ApplicationWindow};
use levenshtein::levenshtein;
use simd_json::prelude::ArrayTrait;
use std::collections::HashMap;
use std::rc::Rc;
use std::{cell::RefCell, f32};

use super::context::make_context;
use super::util::*;
use crate::{
    api::api::SherlockAPI,
    g_subclasses::{action_entry::ContextAction, sherlock_row::SherlockRow, tile_item::TileItem},
    launcher::{utils::HomeType, Launcher},
    loader::util::ExecVariable,
    prelude::{IconComp, SherlockNav, SherlockSearch, ShortCut},
    ui::{event_port::EventPort, g_templates::SearchUiObj},
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

    let binds = ConfigGuard::read()
        .map(|c| c.keybinds.clone())
        .unwrap_or_default();
    let event_port = EventPort::new(
        ui.downgrade(),
        handler.filter.clone(),
        handler.sorter.clone(),
        binds,
        Rc::clone(&handler.mode),
        context.clone(),
        Rc::clone(&custom_handler),
    );
    let event_port = Rc::new(RefCell::new(event_port));

    nav_event(
        ui.downgrade(),
        Rc::clone(&event_port),
        stack_page_ref,
        Rc::clone(&custom_handler),
    );
    change_event(
        ui.downgrade(),
        Rc::clone(&event_port),
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
    ui_weak: WeakRef<SearchUiObj>,
    event_port: Rc<RefCell<EventPort>>,
    stack_page: &Rc<RefCell<String>>,
    custom_handler: Rc<RefCell<UserBindHandler>>,
) {
    let ui = ui_weak.upgrade().unwrap();
    let imp = ui.imp();

    let search_bar = imp.search_bar.downgrade();

    let event_controller = EventControllerKey::new();
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed({
        let stack_page = Rc::clone(&stack_page);
        move |_, key, _, mods| {
            if stack_page.borrow().as_str() != "search-page" {
                return false.into();
            };

            // Construct event string
            let event_string = EventPort::key_event_string(key, mods);
            if event_string.is_empty() {
                return false.into();
            }

            let port_clone = event_port.clone();
            event_port
                .borrow()
                .handle_key_event(&event_string, port_clone)
                .into()
        }
    });

    if let Some(entry) = search_bar.upgrade() {
        entry.add_controller(event_controller);
        if let Some(custom_controller) = custom_handler.borrow().get_controller() {
            entry.add_controller(custom_controller);
        }
    }
}

fn change_event(
    ui_weak: WeakRef<SearchUiObj>,
    event_port: Rc<RefCell<EventPort>>,
    modes: Rc<RefCell<HashMap<String, Vec<Rc<Launcher>>>>>,
    mode: &Rc<RefCell<String>>,
    search_query: &Rc<RefCell<String>>,
) -> Option<()> {
    let ui = ui_weak.upgrade()?;
    let imp = ui.imp();

    let search_bar = imp.search_bar.get();
    let results = imp.results.downgrade();

    search_bar.connect_changed({
        let mode_clone = Rc::clone(mode);
        let search_query_clone = Rc::clone(search_query);
        let ui_clone = ui_weak.clone();

        move |search_bar| {
            let mut current_text = search_bar.text().to_string();

            // Make search bar auto resize
            let layout = search_bar.create_pango_layout(Some(&current_text));
            let (w, h) = layout.size();
            let hpx = w / gtk4::pango::SCALE;
            let vpx = h / gtk4::pango::SCALE;

            search_bar.set_size_request(hpx + 1, vpx + 1);

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

                // Show arg bars
                if let Some(sel) = res.selected_item().and_downcast::<TileItem>() {
                    let vars = sel.variables();
                    if let Some(ui) = ui_clone.upgrade() {
                        if vars.len() > 0 {
                            // add variable fields
                            ui.remove_arg_bars();
                            for (i, var) in vars.into_iter().enumerate() {
                                match var {
                                    ExecVariable::StringInput(placeholder) => {
                                        ui.add_arg_bar(i as u8, &placeholder, event_port.clone());
                                    }
                                    ExecVariable::PasswordInput(placeholder) => {
                                        ui.add_pwd_bar(i as u8, &placeholder, event_port.clone());
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
