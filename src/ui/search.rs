use futures::future::join_all;
use gio::{glib::WeakRef, ActionEntry};
use gtk4::{
    self,
    gdk::{self, Key, ModifierType},
    prelude::*,
    Builder, EventControllerKey, Image, Overlay, Spinner,
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
    result_viewport: WeakRef<ScrolledWindow>,
    // will be later used for split view to display information about apps/commands
    preview_box: WeakRef<HVBox>,
    search_bar: WeakRef<Entry>,
    search_icon_holder: WeakRef<HVBox>,
    mode_title: WeakRef<Label>,
    spinner: WeakRef<Spinner>,
}

pub fn search(
    launchers: &Vec<Launcher>,
    window: &ApplicationWindow,
    stack_page_ref: &Rc<RefCell<String>>,
) -> HVBox {
    // Initialize the view to show all apps
    let (mode, modes, stack_page, ui, results) = construct_window(&launchers);
    ui.result_viewport
        .upgrade()
        .map(|view| view.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic));

    let initial_mode = mode.borrow().clone();
    let search_bar_clone = ui.search_bar.clone();
    let search_bar_clone2 = ui.search_bar.clone();
    let modes_clone = modes.clone();
    let mode_clone = Rc::clone(&mode);

    let search_bar = ui.search_bar.clone();
    stack_page.connect_realize(move |_| {
        search_bar
            .upgrade()
            .map(|search_bar| search_bar.grab_focus());
    });

    let custom_binds = ConfKeys::new();
    let results = results.downgrade();

    change_event(
        ui.search_bar.clone(),
        modes,
        &mode,
        &launchers,
        results.clone(),
        &custom_binds,
    );
    nav_event(
        &stack_page,
        results,
        ui.search_bar.clone(),
        ui.result_viewport.clone(),
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
                        ui.search_icon_holder
                            .upgrade()
                            .map(|holder| holder.set_css_classes(&["back"]));
                        ui.mode_title
                            .upgrade()
                            .map(|title| title.set_text("Search"));
                    }
                    _ => {
                        parameter.push_str(" ");
                        let mode_name = modes_clone.get(&parameter);
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
                        let search_bar_clone = search_bar_clone.clone();
                        glib::idle_add_local(move || {
                            // to trigger homescreen rebuild
                            search_bar_clone.upgrade().map(|entry| {
                                entry.set_text("\n");
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

    let action_clear_win = ActionEntry::builder("clear-search")
        .activate(move |_: &ApplicationWindow, _, _| {
            let search_bar_clone = search_bar_clone2.clone();
            glib::idle_add_local(move || {
                search_bar_clone.upgrade().map(|entry| entry.set_text(""));
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
    ListBox,
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
    let results: ListBox = builder.object("result-frame").unwrap();

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

    let spinner: Spinner = builder.object("status-bar-spinner").unwrap_or_default();
    let preview_box: HVBox = builder.object("preview_box").unwrap_or_default();
    let search_bar: Entry = builder.object("search-bar").unwrap_or_default();
    let result_viewport: ScrolledWindow = builder.object("scrolled-window").unwrap_or_default();
    let mode_title: Label = builder.object("category-type-label").unwrap_or_default();
    let ui = SearchUI {
        result_viewport: result_viewport.downgrade(),
        preview_box: preview_box.downgrade(),
        search_bar: search_bar.downgrade(),
        search_icon_holder: search_icon_holder.downgrade(),
        mode_title: mode_title.downgrade(),
        spinner: spinner.downgrade(),
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

    (mode, modes, vbox, ui, results)
}

fn nav_event(
    stack: &HVBox,
    results: WeakRef<ListBox>,
    search_bar: WeakRef<Entry>,
    result_viewport: WeakRef<ScrolledWindow>,
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
                results
                    .upgrade()
                    .map(|results| results.focus_prev(&result_viewport));
                return true.into();
            }
            k if Some(k) == custom_binds.next
                && custom_binds
                    .next_mod
                    .map_or(true, |m| modifiers.contains(m)) =>
            {
                results
                    .upgrade()
                    .map(|results| results.focus_next(&result_viewport));
                return true.into();
            }
            gdk::Key::Up => {
                results
                    .upgrade()
                    .map(|results| results.focus_prev(&result_viewport));
            }
            gdk::Key::Down => {
                results
                    .upgrade()
                    .map(|results| results.focus_next(&result_viewport));
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
                if ctext.is_empty() {
                    let _ = search_bar.upgrade().map(|entry| {
                        entry.activate_action("win.switch-mode", Some(&"all".to_variant()))
                    });
                }
                results.upgrade().map(|results| results.focus_first());
            }
            gdk::Key::Return => {
                if let Some(upgr) = results.upgrade() {
                    if let Some(row) = upgr.selected_row().and_downcast_ref::<SherlockRow>() {
                        row.emit_by_name::<()>("row-should-activate", &[]);
                    }
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
                    results.upgrade().map(|r| execute_by_index(&r, key_index));
                    return true.into();
                }
            }
            // Pain - solution for shift-tab since gtk handles it as an individual event
            _ if i == 23 && modifiers.contains(ModifierType::SHIFT_MASK) => {
                let shift = Some(ModifierType::SHIFT_MASK);
                let tab = Some(Key::Tab);
                if custom_binds.prev_mod == shift && custom_binds.prev == tab {
                    results
                        .upgrade()
                        .map(|results| results.focus_prev(&result_viewport));
                    return true.into();
                } else if custom_binds.next_mod == shift && custom_binds.next == tab {
                    results
                        .upgrade()
                        .map(|results| results.focus_next(&result_viewport));
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
    search_bar: WeakRef<Entry>,
    modes: HashMap<String, Option<String>>,
    mode: &Rc<RefCell<String>>,
    launchers: &Vec<Launcher>,
    results: WeakRef<ListBox>,
    custom_binds: &ConfKeys,
) -> Option<()> {
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

    let search_bar = search_bar.upgrade()?;
    search_bar.connect_changed({
        let launchers_clone = launchers.clone();
        let mode_clone = Rc::clone(mode);
        let results = results.clone();

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
                &results,
                &mod_str,
                false,
            );
        }
    });
    Some(())
}

pub fn async_calc(
    cancel_flag: &Rc<RefCell<bool>>,
    current_task: &Rc<RefCell<Option<glib::JoinHandle<()>>>>,
    launchers: &[Launcher],
    mode: &Rc<RefCell<String>>,
    current_text: String,
    results: &WeakRef<ListBox>,
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
                launcher
                    .get_loader_widget(&current_text)
                    .map(|(tile, result_item)| {
                        async_widgets.push(result_item);
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
        let results = results.clone();
        async move {
            if *cancel_flag.borrow() {
                return;
            }
            // Set spinner active
            results
                .upgrade()
                .map(|r| r.activate_action("win.spinner-mode", Some(&true.to_variant())));
            // Make async tiles update concurrently
            let futures: Vec<_> = async_launchers
                .into_iter()
                .map(|widget| {
                    let current_text = current_text.clone();
                    async move {
                        let mut attrs = widget.attrs.clone();

                        // Process text tile
                        if let Some(opts) = &widget.text_tile {
                            if let Some((title, body, next_content)) =
                                widget.launcher.get_result(&current_text).await
                            {
                                opts.title.upgrade().map(|t| t.set_text(&title));
                                opts.body.upgrade().map(|b| b.set_text(&body));
                                if let Some(next_content) = next_content {
                                    attrs.insert(
                                        String::from("next_content"),
                                        next_content.to_string(),
                                    );
                                }
                            }
                        }

                        // Process image replacement
                        if let Some(opts) = &widget.image_replacement {
                            if let Some(overlay) = &opts.icon_holder_overlay {
                                if let Some((image, was_cached)) = widget.launcher.get_image().await
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
                        if let Some(wtr) = &widget.weather_tile {
                            if let Some((data, was_changed)) = widget.launcher.get_weather().await {
                                let css_class = if was_changed {
                                    "weather-animate"
                                } else {
                                    "weather-no-animate"
                                };
                                widget.row.upgrade().map(|row| {
                                    row.add_css_class(css_class);
                                    row.add_css_class(&data.icon);
                                });
                                wtr.temperature
                                    .upgrade()
                                    .map(|tmp| tmp.set_text(&data.temperature));
                                wtr.spinner.upgrade().map(|spn| spn.set_spinning(false));
                                wtr.icon
                                    .upgrade()
                                    .map(|ico| ico.set_icon_name(Some(&data.icon)));
                                wtr.location
                                    .upgrade()
                                    .map(|loc| loc.set_text(&data.format_str));
                            } else {
                                widget.row.upgrade().map(|row| row.set_visible(false));
                            }
                        }

                        // Connect row-should-activate signal
                        widget.row.upgrade().map(|row| {
                            row.connect("row-should-activate", false, move |row| {
                                let row = row.first().map(|f| f.get::<SherlockRow>().ok())??;
                                execute_from_attrs(&row, &attrs);
                                None
                            });
                        })
                    }
                })
                .collect();

            let _ = join_all(futures).await;
            // Set spinner inactive
            results
                .upgrade()
                .map(|r| r.activate_action("win.spinner-mode", Some(&false.to_variant())));
            *current_task_clone.borrow_mut() = None;
        }
    });
    *current_task.borrow_mut() = Some(task);
}

pub fn populate(
    keyword: &str,
    mode: &str,
    results_frame: &WeakRef<ListBox>,
    launchers: &Vec<Launcher>,
    async_widgets: Option<Vec<ResultItem>>,
    animate: bool,
    mod_str: &str,
) {
    // Remove all elements inside to avoid duplicates
    if let Some(frame) = results_frame.upgrade() {
        while let Some(row) = frame.last_child() {
            frame.remove(&row);
        }
    }
    let mut launcher_tiles = construct_tiles(&keyword.to_string(), &launchers, &mode.to_string());
    if let Some(widgets) = async_widgets {
        launcher_tiles.extend(widgets);
    }

    launcher_tiles.sort_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap());

    if let Some(c) = CONFIG.get() {
        let mut shortcut_index = 1;
        if let Some(frame) = results_frame.upgrade() {
            launcher_tiles.into_iter().for_each(|widget| {
                if animate && c.behavior.animate {
                    widget.row_item.add_css_class("animate");
                }
                if let Some(shortcut_holder) = &widget.shortcut_holder {
                    shortcut_index += shortcut_holder
                        .upgrade()
                        .map_or(0, |holder| holder.apply_shortcut(shortcut_index, mod_str));
                }
                frame.append(&widget.row_item);
            });
        }
    }
    results_frame.upgrade().map(|r| r.focus_first());
}
