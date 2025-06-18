use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::{glib::WeakRef, ActionEntry, ListStore};
use gtk4::{
    self,
    gdk::{Key, ModifierType},
    prelude::*,
    CustomFilter, CustomSorter, EventControllerKey, FilterListModel, ListView, Overlay,
    SignalListItemFactory, SingleSelection, SortListModel,
};
use gtk4::{glib, ApplicationWindow, Entry};
use levenshtein::levenshtein;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::context::make_context;
use super::util::*;
use crate::{
    api::{api::SherlockAPI, call::ApiCall, server::SherlockServer},
    g_subclasses::sherlock_row::SherlockRow,
    launcher::utils::HomeType,
    prelude::{IconComp, SherlockNav, SherlockSearch, ShortCut},
    ui::key_actions::KeyActions,
    utils::config::{default_search_icon, default_search_icon_back},
};
use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
    CONFIG,
};

#[sherlock_macro::timing(name = "Search Window Creation")]
pub fn search(
    window: &ApplicationWindow,
    stack_page_ref: &Rc<RefCell<String>>,
    error_model: WeakRef<ListStore>,
    sherlock: Rc<RefCell<SherlockAPI>>,
) -> Result<(Overlay, SearchHandler), SherlockError> {
    // Initialize the view to show all apps
    let (search_query, mode, stack_page, ui, handler, context) = construct_window(error_model)?;
    let imp = ui.imp();
    imp.result_viewport
        .set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

    {
        let mut sherlock = sherlock.borrow_mut();
        sherlock.search_ui = Some(ui.downgrade());
        sherlock.search_handler = Some(handler.clone());
    }

    // Mode setup - used to decide which tiles should be shown
    let initial_mode = mode.borrow().clone();

    // Initial setup on show
    stack_page.connect_map({
        let search_bar = imp.search_bar.downgrade();
        move |_| {
            // Focus search bar as soon as it's visible
            search_bar
                .upgrade()
                .map(|search_bar| search_bar.grab_focus());
        }
    });
    if let Some(model) = handler.model.as_ref().and_then(|tmp| tmp.upgrade()) {
        model.connect_items_changed({
            let results = imp.results.downgrade();
            let context_model = context.model.clone();
            let current_mode = Rc::clone(&mode);
            move |_myself, _position, _removed, added| {
                if added == 0 {
                    return;
                }
                // Show or hide context menu shortcuts whenever stack shows
                results
                    .upgrade()
                    .map(|r| r.focus_first(Some(&context_model), Some(current_mode.clone())));
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
        &mode,
        context.clone(),
    );
    change_event(
        imp.search_bar.downgrade(),
        imp.results.downgrade(),
        Rc::clone(&handler.modes),
        &mode,
        &search_query,
    );

    // Improved mode selection
    let mode_action = ActionEntry::builder("switch-mode")
        .parameter_type(Some(&String::static_variant_type()))
        .state(initial_mode.to_variant())
        .activate({
            let mode_clone = Rc::clone(&mode);
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
                            let mode_name = modes_clone.borrow().get(&parameter).cloned();
                            match mode_name {
                                Some(name) => {
                                    imp.search_icon_holder.set_css_classes(&["back"]);
                                    imp.mode_title.set_text(name.as_deref().unwrap_or_default());

                                    *mode_clone.borrow_mut() = parameter.clone();
                                    state = parameter;
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
            let current_mode = Rc::clone(&mode);
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
                        if results
                            .focus_first(Some(&context_model), Some(current_mode.clone()))
                            .is_some()
                        {
                            update_async(weaks, &current_task, current_text.borrow().clone());
                        }
                    } else {
                        update_async(weaks, &current_task, current_text.borrow().clone());
                    }
                }
            }
        })
        .build();

    // Spinner action
    let spinner_clone = imp.spinner.downgrade();
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

    // Action to display or hide context menu shortcut
    let context_action = ActionEntry::builder("context-mode")
        .parameter_type(Some(&bool::static_variant_type()))
        .activate({
            let desc = imp.context_action_desc.downgrade();
            let first = imp.context_action_first.downgrade();
            let second = imp.context_action_second.downgrade();
            move |_, _, parameter| {
                let parameter = parameter.and_then(|p| p.get::<bool>());
                parameter.map(|p| {
                    if p {
                        desc.upgrade().map(|tmp| tmp.set_css_classes(&["active"]));
                        first.upgrade().map(|tmp| tmp.set_css_classes(&["active"]));
                        second.upgrade().map(|tmp| tmp.set_css_classes(&["active"]));
                    } else {
                        desc.upgrade().map(|tmp| tmp.set_css_classes(&["inactive"]));
                        first
                            .upgrade()
                            .map(|tmp| tmp.set_css_classes(&["inactive"]));
                        second
                            .upgrade()
                            .map(|tmp| tmp.set_css_classes(&["inactive"]));
                    };
                });
            }
        })
        .build();

    let action_clear_win = ActionEntry::builder("clear-search")
        .parameter_type(Some(&bool::static_variant_type()))
        .activate({
            let search_bar = imp.search_bar.downgrade();
            let mode = Rc::clone(&mode);
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
    window.add_action_entries([
        mode_action,
        action_clear_win,
        action_spinner,
        context_action,
        sorter_actions,
    ]);

    return Ok((stack_page, handler));
}

fn construct_window(
    error_model: WeakRef<ListStore>,
) -> Result<
    (
        Rc<RefCell<String>>,
        Rc<RefCell<String>>,
        Overlay,
        SearchUiObj,
        SearchHandler,
        ContextUI,
    ),
    SherlockError,
> {
    // Collect Modes
    let custom_binds = ConfKeys::new();
    let config = CONFIG
        .get()
        .ok_or_else(|| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;
    let original_mode = config.behavior.sub_menu.as_deref().unwrap_or("all");
    let mode = Rc::new(RefCell::new(original_mode.to_string()));
    let search_text = Rc::new(RefCell::new(String::from("")));

    // Initialize the builder with the correct path
    let ui = SearchUiObj::new();
    let imp = ui.imp();

    let (context, revealer) = make_context();
    let main_overlay = Overlay::new();
    main_overlay.set_child(Some(&ui));
    main_overlay.add_overlay(&revealer);

    // Update the search icon
    imp.search_icon.set_icon(
        Some(&config.appearance.search_bar_icon),
        None,
        Some(&default_search_icon()),
    );
    imp.search_icon
        .set_pixel_size(config.appearance.search_icon_size);

    // Create the back arrow
    imp.search_icon_back.set_icon(
        Some(&config.appearance.search_bar_icon_back),
        None,
        Some(&default_search_icon_back()),
    );
    imp.search_icon_back
        .set_pixel_size(config.appearance.search_icon_size);

    // Setup model and factory
    let model = ListStore::new::<SherlockRow>();
    let factory = make_factory();
    imp.results.set_factory(Some(&factory));

    // Setup selection
    let sorter = make_sorter(&search_text);
    let filter = make_filter(&search_text, &mode);
    let filter_model = FilterListModel::new(Some(model.clone()), Some(filter.clone()));
    let sorted_model = SortListModel::new(Some(filter_model), Some(sorter.clone()));

    // Set and update `modkey + num` shortcut ui
    let first_iter = Cell::new(true);
    sorted_model.connect_items_changed({
        let mod_str = custom_binds.shortcut_modifier_str.clone();
        let search_text = Rc::clone(&search_text);
        let first_iter = Cell::clone(&first_iter);
        move |myself, _, removed, added| {
            // Early exit if nothing changed
            if added == 0 && removed == 0 {
                return;
            }
            let mut added_index = 0;
            let apply_css = search_text.borrow().trim().is_empty()
                && config.behavior.animate
                && first_iter.get();
            for i in 0..myself.n_items() {
                if let Some(item) = myself.item(i).and_downcast::<SherlockRow>() {
                    if apply_css {
                        item.add_css_class("animate");
                    } else {
                        item.remove_css_class("animate");
                    }
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
            first_iter.set(false);
        }
    });

    let selection = SingleSelection::new(Some(sorted_model));
    imp.results.set_model(Some(&selection));

    imp.results.set_model(Some(&selection));
    imp.results.set_factory(Some(&factory));

    if let Some(context_str) = &custom_binds.context_str {
        imp.context_action_first
            .set_text(&custom_binds.context_mod_str);
        imp.context_action_second.set_text(context_str);
    } else {
        imp.context_action_first.set_visible(false);
        imp.context_action_second.set_visible(false);
    }

    let handler = SearchHandler::new(
        model.downgrade(),
        error_model,
        filter.downgrade(),
        sorter.downgrade(),
        custom_binds,
        first_iter,
    );

    if config.expand.enable {
        imp.result_viewport
            .set_max_content_height(config.appearance.height);
        imp.result_viewport.set_propagate_natural_height(true);
    }

    // disable status bar
    if !config.appearance.status_bar {
        imp.status_bar.set_visible(false);
    }
    // enable or disable search bar icons
    imp.search_icon_holder
        .set_visible(config.appearance.search_icon);

    Ok((search_text, mode, main_overlay, ui, handler, context))
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
            let home = item.home();

            let mode = search_mode.borrow().trim().to_string();
            let current_text = search_text.borrow().clone();
            let is_home = current_text.is_empty() && mode == "all";

            let update_res = item.update(&current_text);

            if is_home {
                if home != HomeType::Search {
                    return true;
                }
                false
            } else {
                let alias = item.alias();
                let priority = item.priority();
                if mode != "all" {
                    if home == HomeType::OnlyHome || mode != alias {
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
    results: WeakRef<ListView>,
    search_bar: WeakRef<Entry>,
    filter: WeakRef<CustomFilter>,
    sorter: WeakRef<CustomSorter>,
    binds: ConfKeys,
    stack_page: &Rc<RefCell<String>>,
    current_mode: &Rc<RefCell<String>>,
    context: ContextUI,
) {
    let event_controller = EventControllerKey::new();
    let stack_page = Rc::clone(stack_page);
    let multi = CONFIG.get().map_or(false, |c| c.runtime.multi);
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed({
        let search_bar = search_bar.clone();
        let current_mode = Rc::clone(current_mode);
        let key_actions = KeyActions::new(results, search_bar, context);
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
                    key_actions.on_return(key_actions.context.open.get(), Some(false))
                }

                // Context menu opening
                _ if matches(binds.context, binds.context_mod) => {
                    key_actions.open_context();
                }

                // Custom previous key
                Key::Up => key_actions.on_prev(),
                _ if matches(binds.prev, binds.prev_mod) => {
                    key_actions.on_prev();
                }

                // Custom next key
                Key::Down => key_actions.on_next(),
                _ if matches(binds.next, binds.next_mod) => {
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
                    key_actions.on_multi_return();
                }
                Key::Return | Key::KP_Enter => {
                    key_actions.on_return(key_actions.context.open.get(), None);
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
                        if let Some(index) = key
                            .name()
                            .and_then(|name| name.parse::<u32>().ok().map(|v| v - 1))
                        {
                            key_actions
                                .results
                                .upgrade()
                                .map(|r| r.execute_by_index(index));
                        }
                    } else {
                        return false.into();
                    }
                }
                // Pain - solution for shift-tab since gtk handles it as an individual event
                _ if i == 23 && mods.contains(ModifierType::SHIFT_MASK) => {
                    let shift = Some(ModifierType::SHIFT_MASK);
                    let tab = Some(Key::Tab);
                    if binds.prev_mod == shift && binds.prev == tab {
                        key_actions.on_prev();
                    } else if binds.next_mod == shift && binds.next == tab {
                        key_actions.on_next();
                    }
                }
                _ => return false.into(),
            }
            true.into()
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

mod imp {
    use gtk4::subclass::prelude::*;
    use gtk4::CompositeTemplate;
    use gtk4::{glib, Entry, Image, ListView, ScrolledWindow, Spinner};
    use gtk4::{Box as GtkBox, Label};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/search.ui")]
    pub struct SearchUiObj {
        #[template_child(id = "split-view")]
        pub all: TemplateChild<GtkBox>,

        #[template_child(id = "status-bar-spinner")]
        pub spinner: TemplateChild<Spinner>,

        #[template_child(id = "preview_box")]
        pub preview_box: TemplateChild<GtkBox>,

        #[template_child(id = "search-bar")]
        pub search_bar: TemplateChild<Entry>,

        #[template_child(id = "scrolled-window")]
        pub result_viewport: TemplateChild<ScrolledWindow>,

        #[template_child(id = "category-type-holder")]
        pub mode_title_holder: TemplateChild<GtkBox>,

        #[template_child(id = "category-type-label")]
        pub mode_title: TemplateChild<Label>,

        #[template_child(id = "context-menu-desc")]
        pub context_action_desc: TemplateChild<Label>,

        #[template_child(id = "context-menu-first")]
        pub context_action_first: TemplateChild<Label>,

        #[template_child(id = "context-menu-second")]
        pub context_action_second: TemplateChild<Label>,

        #[template_child(id = "status-bar")]
        pub status_bar: TemplateChild<GtkBox>,

        #[template_child(id = "search-icon-holder")]
        pub search_icon_holder: TemplateChild<GtkBox>,

        #[template_child(id = "search-icon")]
        pub search_icon: TemplateChild<Image>,

        #[template_child(id = "search-icon-back")]
        pub search_icon_back: TemplateChild<Image>,

        #[template_child(id = "result-frame")]
        pub results: TemplateChild<ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchUiObj {
        const NAME: &'static str = "SearchUI";
        type Type = super::SearchUiObj;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchUiObj {}
    impl WidgetImpl for SearchUiObj {}
    impl BoxImpl for SearchUiObj {}
}

glib::wrapper! {
    pub struct SearchUiObj(ObjectSubclass<imp::SearchUiObj>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl SearchUiObj {
    pub fn new() -> Self {
        let ui = glib::Object::new::<Self>();
        let imp = ui.imp();
        imp.search_icon_holder.add_css_class("search");
        imp.results.set_focusable(false);
        ui
    }
}
