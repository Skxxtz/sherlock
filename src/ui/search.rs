use std::{cell::RefCell, collections::HashMap, rc::Rc};

use async_std::task::block_on;
use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::{
    glib::{
        object::{Cast, CastNone, ObjectExt},
        GString,
    },
    ListStore,
};
use gtk4::{
    prelude::{BoxExt, EditableExt, FilterExt, ListBoxRowExt, ListItemExt, SorterExt, WidgetExt},
    ApplicationWindow, Box, Builder, CustomFilter, CustomSorter, Entry, FilterChange,
    FilterListModel, Image, Label, ListItem, ListView, Ordering, Overlay, ScrolledWindow,
    SignalListItemFactory, SingleSelection, SortListModel, SorterChange, Spinner, Widget,
};
use levenshtein::levenshtein;

use crate::{g_subclasses::sherlock_row::SherlockRow, launcher::Launcher, CONFIG};

use super::{
    entry::AppEntryObject,
    tiles::util::{SherlockSearch, Tile, TileBuilder},
};

struct SearchInterface {
    result_viewport: ScrolledWindow,
    // will be later used for split view to display information about apps/commands
    preview_box: Box,
    search_bar: Entry,
    search_icon_holder: Box,
    mode_title: Label,
    spinner: Spinner,
    results_view: Rc<ListView>,
    search_mode: Rc<RefCell<GString>>,
    search_text: Rc<RefCell<GString>>,
    sorter: CustomSorter,
    filter: CustomFilter,
    list_store: ListStore,
    vbox: Box,
}

impl SearchInterface {
    pub fn construct(launchers: &Vec<Launcher>) -> Self {
        let original_mode = CONFIG
            .get()
            .and_then(|c| c.behavior.sub_menu.as_deref())
            .unwrap_or("all");

        let mode = Rc::new(RefCell::new(original_mode.to_string()));
        let modes: HashMap<String, Option<String>> = launchers
            .iter()
            .filter_map(|item| item.alias.as_ref().map(|alias| (alias, &item.name)))
            .map(|(alias, name)| (format!("{} ", alias), name.clone()))
            .collect();

        let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/search.ui");

        let vbox: Box = builder.object("vbox").unwrap();
        let results: Rc<ListView> = Rc::new(builder.object("result-frame").unwrap());

        results.add_css_class("result-frame");

        let search_icon_holder: Box = builder.object("search-icon-holder").unwrap_or_default();

        search_icon_holder.add_css_class("search");

        let search_icon = Image::new();

        search_icon.set_icon_name(Some("search"));
        search_icon.set_widget_name("search-icon");
        search_icon.set_halign(gtk4::Align::End);

        let search_icon_back = Image::new();

        search_icon_back.set_icon_name(Some("go-previous"));
        search_icon_back.set_widget_name("search-icon-back");
        search_icon_back.set_halign(gtk4::Align::End);

        let overlay = Overlay::new();

        overlay.set_child(Some(&search_icon));
        overlay.add_overlay(&search_icon_back);

        CONFIG.get().map(|c| {
            if !c.appearance.status_bar {
                let n: Option<Box> = builder.object("status-bar");

                n.map(|n| n.set_visible(false));
            }
        });

        search_icon_holder.append(&overlay);

        let search_text = Rc::new(RefCell::new(GString::new()));
        let search_mode = Rc::new(RefCell::new(GString::new()));

        let sorter = CustomSorter::new({
            let search_text = search_text.clone();

            move |item_a, item_b| {
                let search_text = search_text.borrow();

                let item_a = item_a.downcast_ref::<AppEntryObject>().unwrap();
                let item_b = item_b.downcast_ref::<AppEntryObject>().unwrap();

                let mut priority_a = item_a.priority();
                let mut priority_b = item_b.priority();

                if !search_text.is_empty() {
                    priority_a += levenshtein(&search_text, &item_a.name()) as f32;
                    priority_b += levenshtein(&search_text, &item_b.name()) as f32;
                }

                priority_a.total_cmp(&priority_b).into()
            }
        });

        let filter = CustomFilter::new({
            let search_text = search_text.clone();
            let search_mode = search_mode.clone();

            move |entry| {
                let item = entry.downcast_ref::<AppEntryObject>().unwrap();

                /* item.mode() == *search_mode.borrow()
                && */
                item.name()
                    .to_ascii_lowercase()
                    .fuzzy_match(&*search_text.borrow().to_ascii_lowercase())
            }
        });

        let list_store = ListStore::new::<AppEntryObject>();

        for launcher in launchers {
            if let Some(apps) = launcher.get_apps() {
                let launcher_name = launcher.name.as_ref().unwrap();

                for (name, data) in apps {
                    list_store.append(&AppEntryObject::new(name, "", &launcher_name, data.clone()));
                }
            }
        }

        let filter_model = FilterListModel::new(Some(list_store.clone()), Some(filter.clone()));
        let sorted_model = SortListModel::new(Some(filter_model), Some(sorter.clone()));

        let selection = SingleSelection::new(Some(sorted_model));
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let row = SherlockRow::new();

            list_item.set_child(Some(&row));
        });

        factory.connect_unbind(|_, list_item| {
            let row = list_item
                .child()
                .unwrap()
                .downcast::<SherlockRow>()
                .unwrap();

            row.set_child(Option::<&Widget>::None);

            unsafe {
                list_item.steal_data::<Tile>("tile");
            }
        });

        factory.connect_bind(move |_, list_item| {
            let row = list_item.child().unwrap();
            let row = row.downcast_ref::<SherlockRow>().unwrap();
            let tile = Tile::from_resource("/dev/skxxtz/sherlock/ui/tile.ui");

            row.set_widgets(tile.widgets.clone());
            row.set_child(Some(&tile.root));
            row.add_css_class("tile");

            unsafe {
                list_item.set_data("tile", tile);
            }

            let item = list_item.item();
            let item = item
                .and_downcast_ref::<AppEntryObject>()
                .expect("Item should be ItemObject");

            let widgets = row.widgets();

            let (title, category, icon) = (
                widgets.title.upgrade().unwrap(),
                widgets.category.upgrade().unwrap(),
                widgets.icon.upgrade().unwrap(),
            );

            let app_data = item.app_data();

            row.set_priority(item.priority());

            title.set_text(&item.name());
            category.set_text(&item.launcher_name());

            if let Some(class) = &app_data.icon_class {
                icon.add_css_class(class);
            }

            if app_data.icon.starts_with("/") {
                icon.set_from_file(Some(&app_data.icon));
            } else {
                icon.set_icon_name(Some(&app_data.icon));
            }
        });

        results.set_factory(Some(&factory));
        results.set_model(Some(&selection));

        let ui = Self {
            result_viewport: builder.object("scrolled-window").unwrap_or_default(),
            preview_box: builder.object("preview_box").unwrap_or_default(),
            search_bar: builder.object("search-bar").unwrap_or_default(),
            search_icon_holder,
            mode_title: builder.object("category-type-label").unwrap_or_default(),
            spinner: builder.object("status-bar-spinner").unwrap_or_default(),
            vbox,
            results_view: results,
            search_text,
            search_mode,
            list_store,
            filter,
            sorter,
        };

        CONFIG.get().map(|c| {
            ui.result_viewport
                .set_size_request((c.appearance.width as f32 * 0.4) as i32, 10);

            ui.search_icon_holder.set_visible(c.appearance.search_icon);

            search_icon.set_pixel_size(c.appearance.icon_size);
            search_icon_back.set_pixel_size(c.appearance.icon_size);
        });

        ui
    }

    pub fn connect_change_event(&self) {
        let search_bar = &self.search_bar;

        let search_text = self.search_text.clone();
        let sorter = self.sorter.clone();
        let filter = self.filter.clone();

        search_bar.connect_changed(move |search_bar| {
            *search_text.borrow_mut() = search_bar.text();

            sorter.changed(SorterChange::Different);
            filter.changed(FilterChange::Different);
        });
    }
}

pub fn search(
    launchers: &Vec<Launcher>,
    window: &ApplicationWindow,
    stack_page_ref: &Rc<RefCell<String>>,
) -> Box {
    let search_interface = SearchInterface::construct(launchers);
    let search_bar = search_interface.search_bar.clone();

    search_interface
        .result_viewport
        .set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

    search_interface.vbox.connect_realize(move |_| {
        search_bar.grab_focus();
    });

    search_interface.connect_change_event();

    search_interface.vbox
}

/*
use futures::future::join_all;
use gio::ActionEntry;
use gtk4::{
    self, gdk::{self, Key, ModifierType}, prelude::*, Builder, EventControllerKey, Image, ListView, Overlay, SingleSelection, Spinner
};
use gtk4::{glib, ApplicationWindow, Entry};
use gtk4::{Box as HVBox, Label, ListBox, ScrolledWindow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::tiles::util::AsyncLauncherTile;
use super::util::*;
use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::{construct_tiles, Launcher, ResultItem};
use crate::CONFIG;

#[allow(dead_code)]
struct SearchUI {
    result_viewport: ScrolledWindow,
    // will be later used for split view to display information about apps/commands
    preview_box: HVBox,
    search_bar: Entry,
    search_icon_holder: HVBox,
    mode_title: Label,
    spinner: Spinner,
}

pub fn search(
    launchers: &Vec<Launcher>,
    window: &ApplicationWindow,
    stack_page_ref: &Rc<RefCell<String>>,
) -> HVBox {
    // Initialize the view to show all apps
    let (mode, modes, stack_page, ui, results) = construct_window(&launchers);
    ui.result_viewport
        .set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

    let initial_mode = mode.borrow().clone();
    let search_bar_clone = ui.search_bar.clone();
    let search_bar_clone2 = ui.search_bar.clone();
    let modes_clone = modes.clone();
    let mode_clone = Rc::clone(&mode);

    let search_bar = ui.search_bar.clone();
    stack_page.connect_realize(move |_| {
        search_bar.grab_focus();
    });

    let custom_binds = ConfKeys::new();

    change_event(
        &ui.search_bar,
        modes,
        &mode,
        &launchers,
        &results,
        &custom_binds,
    );
    nav_event(
        &stack_page,
        results,
        ui.search_bar,
        ui.result_viewport,
        custom_binds,
        stack_page_ref,
    );

    // Improved mode selection
    let mode_action = ActionEntry::builder("switch-mode")
        .parameter_type(Some(&String::static_variant_type()))
        .state(initial_mode.to_variant())
        .activate(move |_, action, parameter| {
            let state = action.state().and_then(|s| s.get::<String>());
            let parameter = parameter.and_then(|p| p.get::<String>());

            if let (Some(mut state), Some(mut parameter)) = (state, parameter) {
                match parameter.as_str() {
                    "search" => {
                        ui.search_icon_holder.set_css_classes(&["back"]);
                        ui.mode_title.set_text("Search");
                    }
                    _ => {
                        parameter.push_str(" ");
                        let mode_name = modes_clone.get(&parameter);
                        match mode_name {
                            Some(name) => {
                                ui.search_icon_holder.set_css_classes(&["back"]);
                                *mode_clone.borrow_mut() = parameter.clone();
                                ui.mode_title.set_text(name.as_deref().unwrap_or_default());
                                state = parameter;
                            }
                            _ => {
                                ui.search_icon_holder.set_css_classes(&["search"]);
                                ui.mode_title.set_text("All");
                                parameter = String::from("all ");
                                *mode_clone.borrow_mut() = parameter.clone();
                                state = parameter;
                            }
                        }
                        let search_bar_clone = search_bar_clone.clone();
                        glib::idle_add_local(move || {
                            // to trigger homescreen rebuild
                            search_bar_clone.set_text("\n");
                            search_bar_clone.set_text("");
                            glib::ControlFlow::Break
                        });
                        action.set_state(&state.to_variant());
                    }
                }
            }
        })
        .build();

    // Spinner action
    let action_spinner = ActionEntry::builder("spinner-mode")
        .parameter_type(Some(&bool::static_variant_type()))
        .activate(move |_, _, parameter| {
            let parameter = parameter.and_then(|p| p.get::<bool>());
            parameter.map(|p| {
                if p {
                    ui.spinner.set_css_classes(&["spinner-appear"]);
                } else {
                    ui.spinner.set_css_classes(&["spinner-disappear"]);
                };
                ui.spinner.set_spinning(p);
            });
        })
        .build();

    let action_clear_win = ActionEntry::builder("clear-search")
        .activate(move |_: &ApplicationWindow, _, _| {
            let search_bar_clone = search_bar_clone2.clone();
            glib::idle_add_local(move || {
                search_bar_clone.set_text("");
                glib::ControlFlow::Break
            });
        })
        .build();
    window.add_action_entries([mode_action, action_clear_win, action_spinner]);

    return stack_page;
}

fn construct_window(
    launchers: &Vec<Launcher>,
) -> (
    Rc<RefCell<String>>,
    HashMap<String, Option<String>>,
    HVBox,
    SearchUI,
    Rc<ListBox>,
) {
    // Collect Modes
    let original_mode = CONFIG
        .get()
        .and_then(|c| c.behavior.sub_menu.as_deref())
        .unwrap_or("all");
    let mode = Rc::new(RefCell::new(original_mode.to_string()));
    let modes: HashMap<String, Option<String>> = launchers
        .iter()
        .filter_map(|item| item.alias.as_ref().map(|alias| (alias, &item.name)))
        .map(|(alias, name)| (format!("{} ", alias), name.clone()))
        .collect();

    // Initialize the builder with the correct path
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/search.ui");

    // Get the required object references
    let vbox: HVBox = builder.object("vbox").unwrap();
    let results: Rc<ListView> = Rc::new(builder.object("result-frame").unwrap());
    let results: Rc<ListBox> = Rc::new(builder.object("result-frame").unwrap());

    let search_icon_holder: HVBox = builder.object("search-icon-holder").unwrap_or_default();
    search_icon_holder.add_css_class("search");
    // Create the search icon
    let search_icon = Image::new();
    search_icon.set_icon_name(Some("search"));
    search_icon.set_widget_name("search-icon");
    search_icon.set_halign(gtk4::Align::End);
    // Create the back arrow
    let search_icon_back = Image::new();
    search_icon_back.set_icon_name(Some("go-previous"));
    search_icon_back.set_widget_name("search-icon-back");
    search_icon_back.set_halign(gtk4::Align::End);

    let overlay = Overlay::new();
    overlay.set_child(Some(&search_icon));
    overlay.add_overlay(&search_icon_back);

    // Show notification-bar
    CONFIG.get().map(|c| {
        if !c.appearance.status_bar {
            let n: Option<HVBox> = builder.object("status-bar");
            n.map(|n| n.set_visible(false));
        }
    });

    search_icon_holder.append(&overlay);

    let ui = SearchUI {
        result_viewport: builder.object("scrolled-window").unwrap_or_default(),
        preview_box: builder.object("preview_box").unwrap_or_default(),
        search_bar: builder.object("search-bar").unwrap_or_default(),
        search_icon_holder,
        mode_title: builder.object("category-type-label").unwrap_or_default(),
        spinner: builder.object("status-bar-spinner").unwrap_or_default(),
    };
    CONFIG.get().map(|c| {
        ui.result_viewport
            .set_size_request((c.appearance.width as f32 * 0.4) as i32, 10);
        ui.search_icon_holder.set_visible(c.appearance.search_icon);
        search_icon.set_pixel_size(c.appearance.icon_size);
        search_icon_back.set_pixel_size(c.appearance.icon_size);
    });

    (mode, modes, vbox, ui, results)
}

fn nav_event(
    stack: &HVBox,
    results: Rc<ListBox>,
    search_bar: Entry,
    result_viewport: ScrolledWindow,
    custom_binds: ConfKeys,
    stack_page: &Rc<RefCell<String>>,
) {
    let stack_page = Rc::clone(stack_page);
    let event_controller = EventControllerKey::new();
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed(move |_, key, i, modifiers| {
        if stack_page.borrow().as_str() != "search-page" {
            return false.into();
        };
        match key {
            k if Some(k) == custom_binds.prev
                && custom_binds
                    .prev_mod
                    .map_or(true, |m| modifiers.contains(m)) =>
            {
                results.focus_prev(&result_viewport);
                return true.into();
            }
            k if Some(k) == custom_binds.next
                && custom_binds
                    .next_mod
                    .map_or(true, |m| modifiers.contains(m)) =>
            {
                results.focus_next(&result_viewport);
                return true.into();
            }
            gdk::Key::Up => {
                results.focus_prev(&result_viewport);
            }
            gdk::Key::Down => {
                results.focus_next(&result_viewport);
                return true.into();
            }
            gdk::Key::BackSpace => {
                let mut ctext = search_bar.text().to_string();
                if custom_binds
                    .shortcut_modifier
                    .map_or(false, |modifier| modifiers.contains(modifier))
                {
                    let _ = search_bar.set_text("");
                    ctext.clear();
                }
                if ctext.is_empty() {
                    let _ =
                        search_bar.activate_action("win.switch-mode", Some(&"all".to_variant()));
                }
                results.focus_first();
            }
            gdk::Key::Return => {
                if let Some(row) = results.selected_row().and_downcast_ref::<SherlockRow>() {
                    row.emit_by_name::<()>("row-should-activate", &[]);
                }
            }
            Key::_1 | Key::_2 | Key::_3 | Key::_4 | Key::_5 => {
                if custom_binds
                    .shortcut_modifier
                    .map_or(false, |modifier| modifiers.contains(modifier))
                {
                    let key_index = match key {
                        Key::_1 => 1,
                        Key::_2 => 2,
                        Key::_3 => 3,
                        Key::_4 => 4,
                        Key::_5 => 5,
                        _ => return false.into(),
                    };
                    execute_by_index(&*results, key_index);
                    return true.into();
                }
            }
            // Pain - solution for shift-tab since gtk handles it as an individual event
            _ if i == 23 && modifiers.contains(ModifierType::SHIFT_MASK) => {
                let shift = Some(ModifierType::SHIFT_MASK);
                let tab = Some(Key::Tab);
                if custom_binds.prev_mod == shift && custom_binds.prev == tab {
                    results.focus_prev(&result_viewport);
                    return true.into();
                } else if custom_binds.next_mod == shift && custom_binds.next == tab {
                    results.focus_next(&result_viewport);
                    return true.into();
                }
            }
            _ => (),
        }
        false.into()
    });

    stack.add_controller(event_controller);
}

fn change_event(
    search_bar: &Entry,
    modes: HashMap<String, Option<String>>,
    mode: &Rc<RefCell<String>>,
    launchers: &Vec<Launcher>,
    results: &Rc<ListBox>,
    custom_binds: &ConfKeys,
) {
    // Setting up async capabilities
    let current_task: Rc<RefCell<Option<glib::JoinHandle<()>>>> = Rc::new(RefCell::new(None));
    let cancel_flag = Rc::new(RefCell::new(false));

    // vars
    let mod_str = custom_binds.shortcut_modifier_str.clone();

    // Setting home screen
    async_calc(
        &cancel_flag,
        &current_task,
        &launchers,
        &mode,
        String::new(),
        &results,
        &mod_str,
        true,
    );

    search_bar.connect_changed({
        let launchers_clone = launchers.clone();
        let mode_clone = Rc::clone(mode);
        let results_clone = Rc::clone(results);

        move |search_bar| {
            let mut current_text = search_bar.text().to_string();
            if current_text.len() == 1 && current_text != "\n" {
                let _ = search_bar.activate_action("win.switch-mode", Some(&"search".to_variant()));
            } else if current_text.len() == 0 && mode_clone.borrow().as_str() == "all" {
                let _ = search_bar.activate_action("win.switch-mode", Some(&"all".to_variant()));
            }
            if let Some(task) = current_task.borrow_mut().take() {
                task.abort();
            };
            *cancel_flag.borrow_mut() = true;
            let trimmed = current_text.trim();
            if !trimmed.is_empty() && modes.contains_key(&current_text) {
                // Logic to apply modes
                let _ = search_bar.activate_action("win.switch-mode", Some(&trimmed.to_variant()));
                current_text.clear();
            }
            async_calc(
                &cancel_flag,
                &current_task,
                &launchers_clone,
                &mode_clone,
                current_text,
                &results_clone,
                &mod_str,
                false,
            );
        }
    });
}

pub fn async_calc(
    cancel_flag: &Rc<RefCell<bool>>,
    current_task: &Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    launchers: &[Launcher],
    mode: &Rc<RefCell<String>>,
    current_text: String,
    results: &Rc<ListBox>,
    mod_str: &str,
    animate: bool,
) {
    *cancel_flag.borrow_mut() = false;
    // If task is currently running, abort it
    if let Some(t) = current_task.borrow_mut().take() {
        t.abort();
    };
    let is_home = current_text.is_empty() && mode.borrow().as_str().trim() == "all";
    let cancel_flag = Rc::clone(&cancel_flag);
    let filtered_launchers: Vec<Launcher> = launchers
        .iter()
        .filter(|launcher| (is_home && launcher.home) || (!is_home && !launcher.only_home))
        .cloned()
        .collect();
    let (async_launchers, non_async_launchers): (Vec<Launcher>, Vec<Launcher>) = filtered_launchers
        .into_iter()
        .partition(|launcher| launcher.r#async);

    // Create loader widgets
    // TODO
    let current_mode_ref = mode.borrow();
    let current_mode = current_mode_ref.trim();

    // extract result items to reduce cloning
    let mut async_widgets: Vec<ResultItem> = Vec::with_capacity(async_launchers.capacity());
    let async_launchers: Vec<AsyncLauncherTile> = async_launchers
        .into_iter()
        .filter_map(|launcher| {
            if (launcher.priority == 0 && current_mode == launcher.alias.as_deref().unwrap_or(""))
                || (current_mode == "all" && launcher.priority > 0)
            {
                launcher.get_loader_widget(&current_text).map(|tile| {
                    async_widgets.push(tile.result_item.clone());
                    tile
                })
            } else {
                None
            }
        })
        .collect();
    populate(
        &current_text,
        &mode.borrow(),
        &*results,
        &non_async_launchers,
        Some(async_widgets),
        animate,
        mod_str,
    );

    // Gather results for asynchronous widgets
    let task = glib::MainContext::default().spawn_local({
        let current_task_clone = Rc::clone(current_task);
        async move {
            if *cancel_flag.borrow() {
                return;
            }
            if let Some(row) = async_launchers.get(0) {
                let _ = row
                    .result_item
                    .row_item
                    .activate_action("win.spinner-mode", Some(&true.to_variant()));
            }
            // Make them update concurrently
            let futures: Vec<_> = async_launchers
                .iter()
                .map(|widget| {
                    let widget_clone = widget;
                    let current_text = current_text.clone();
                    async move {
                        let mut attrs = widget_clone.attrs.clone();

                        // Process text tile
                        if let Some(opts) = &widget_clone.text_tile {
                            if let Some((title, body, next_content)) =
                                widget_clone.launcher.get_result(&current_text).await
                            {
                                opts.title.set_text(&title);
                                opts.body.set_text(&body);
                                if let Some(next_content) = next_content {
                                    attrs.insert(
                                        String::from("next_content"),
                                        next_content.to_string(),
                                    );
                                }
                            }
                        }

                        // Process image replacement
                        if let Some(opts) = &widget_clone.image_replacement {
                            if let Some(overlay) = &opts.icon_holder_overlay {
                                if let Some((image, was_cached)) =
                                    widget_clone.launcher.get_image().await
                                {
                                    if !was_cached {
                                        overlay.add_css_class("image-replace-overlay");
                                    }
                                    let gtk_image = Image::from_pixbuf(Some(&image));
                                    gtk_image.set_widget_name("album-cover");
                                    gtk_image.set_pixel_size(50);
                                    overlay.add_overlay(&gtk_image);
                                }
                            }
                        }

                        // Process weather tile
                        if let Some(wtr) = &widget_clone.weather_tile {
                            if let Some((data, was_changed)) =
                                widget_clone.launcher.get_weather().await
                            {
                                let css_class = if was_changed {
                                    "weather-animate"
                                } else {
                                    "weather-no-animate"
                                };
                                widget_clone.result_item.row_item.add_css_class(css_class);
                                widget_clone.result_item.row_item.add_css_class(&data.icon);
                                wtr.temperature.set_text(&data.temperature);
                                wtr.spinner.set_spinning(false);
                                wtr.icon.set_icon_name(Some(&data.icon));
                                wtr.location.set_text(&data.format_str);
                            } else {
                                widget_clone.result_item.row_item.set_visible(false);
                            }
                        }

                        // Connect row-should-activate signal
                        widget_clone.result_item.row_item.connect(
                            "row-should-activate",
                            false,
                            move |row| {
                                let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                                execute_from_attrs(&row, &attrs);
                                None
                            },
                        );
                    }
                })
                .collect();

            let _ = join_all(futures).await;
            // Set spinner inactive
            if let Some(row) = async_launchers.get(0) {
                let _ = row
                    .result_item
                    .row_item
                    .activate_action("win.spinner-mode", Some(&false.to_variant()));
            }
            *current_task_clone.borrow_mut() = None;
        }
    });
    *current_task.borrow_mut() = Some(task);
}

pub fn populate(
    keyword: &str,
    mode: &str,
    results_frame: &ListBox,
    launchers: &Vec<Launcher>,
    async_widgets: Option<Vec<ResultItem>>,
    animate: bool,
    mod_str: &str,
) {
    // Remove all elements inside to avoid duplicates
    while let Some(row) = results_frame.last_child() {
        results_frame.remove(&row);
    }
    let mut launcher_tiles = construct_tiles(&keyword.to_string(), &launchers, &mode.to_string());
    if let Some(widgets) = async_widgets {
        launcher_tiles.extend(widgets);
    }

    launcher_tiles.sort_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap());

    if let Some(c) = CONFIG.get() {
        let mut shortcut_index = 1;
        for widget in launcher_tiles {
            if animate && c.behavior.animate {
                widget.row_item.add_css_class("animate");
            }
            if let Some(shortcut_holder) = widget.shortcut_holder {
                shortcut_index += shortcut_holder.apply_shortcut(shortcut_index, mod_str);
            }
            results_frame.append(&widget.row_item);
        }
    }
    results_frame.focus_first();
}
*/
