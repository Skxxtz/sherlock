use gtk4::glib;
use gtk4::{
    self,
    gdk::{self, Key, ModifierType},
    prelude::*,
    Builder, EventControllerKey, Image,
};
use gtk4::{Box as HVBox, Entry, Label, ListBox, ScrolledWindow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::tiles::util::AsyncLauncherTile;
use super::util::*;
use crate::actions::execute_from_attrs;
use crate::g_subclasses::sherlock_row::SherlockRow;
use crate::launcher::{construct_tiles, Launcher, ResultItem};
use crate::{AppState, APP_STATE, CONFIG};

#[allow(dead_code)]
struct SearchUI {
    result_viewport: ScrolledWindow,
    // will be later used for split view to display information about apps/commands
    preview_box: HVBox,
    search_bar: Entry,
    mode_title: Label,
}

pub fn search(launchers: Vec<Launcher>) {
    // Initiallize the view to show all apps
    let (mode, modes, vbox, ui, results) = construct_window(&launchers);
    ui.result_viewport
        .set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    ui.search_bar.grab_focus();

    let custom_binds = ConfKeys::new();

    change_event(&ui, modes, &mode, &launchers, &results, &custom_binds);
    nav_event(results, ui, mode, custom_binds);
    APP_STATE.with(|state| {
        if let Some(ref state) = *state.borrow() {
            state.add_stack_page(vbox, "search-page");
        }
    });
}

fn construct_window(
    launchers: &Vec<Launcher>,
) -> (
    Rc<RefCell<String>>,
    HashMap<String, String>,
    HVBox,
    SearchUI,
    Rc<ListBox>,
) {
    // Collect Modes
    let mode = Rc::new(RefCell::new("all".to_string()));
    let mut modes: HashMap<String, String> = HashMap::new();
    for item in launchers.iter() {
        let alias = item.alias.clone();
        if !alias.is_none() {
            modes.insert(format!("{} ", alias.unwrap()), item.name.clone());
        }
    }

    // Initialize the builder with the correct path
    let builder = Builder::from_resource("/dev/skxxtz/sherlock/ui/search.ui");

    // Get the requred object references
    let vbox: HVBox = builder.object("vbox").unwrap();
    let results: Rc<ListBox> = Rc::new(builder.object("result-frame").unwrap());
    let ui = SearchUI {
        result_viewport: builder.object("scrolled-window").unwrap_or_default(),
        preview_box: builder.object("preview_box").unwrap_or_default(),
        search_bar: builder.object("search-bar").unwrap_or_default(),
        mode_title: builder.object("category-type-label").unwrap_or_default(),
    };
    if let Some(c) = CONFIG.get() {
        ui.result_viewport
            .set_size_request((c.appearance.width as f32 * 0.4) as i32, -1);
    }

    APP_STATE.with(|app_state| {
        let new_state = app_state.borrow_mut().take().map(|old_state| {
            Rc::new(AppState {
                window: old_state.window.clone(),
                stack: old_state.stack.clone(),
                search_bar: Some(ui.search_bar.clone()),
            })
        });
        *app_state.borrow_mut() = new_state;
    });
    (mode, modes, vbox, ui, results)
}

fn nav_event(
    results: Rc<ListBox>,
    ui: SearchUI,
    mode: Rc<RefCell<String>>,
    custom_binds: ConfKeys,
) {
    let event_controller = EventControllerKey::new();
    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed(move |_, key, i, modifiers| {
        match key {
            k if Some(k) == custom_binds.prev
                && custom_binds
                    .prev_mod
                    .map_or(true, |m| modifiers.contains(m)) =>
            {
                results.focus_prev(&ui.result_viewport);
                return true.into();
            }
            k if Some(k) == custom_binds.next
                && custom_binds
                    .next_mod
                    .map_or(true, |m| modifiers.contains(m)) =>
            {
                results.focus_next(&ui.result_viewport);
                return true.into();
            }
            gdk::Key::Up => {
                results.focus_prev(&ui.result_viewport);
            }
            gdk::Key::Down => {
                results.focus_next(&ui.result_viewport);
                return true.into();
            }
            gdk::Key::BackSpace => {
                let ctext = &ui.search_bar.text();
                if custom_binds
                    .shortcut_modifier
                    .map_or(false, |modifier| modifiers.contains(modifier))
                {
                    let _ = &ui.search_bar.set_text("");
                } else {
                    if ctext.is_empty() && mode.borrow().as_str() != "all" {
                        set_mode(&ui.mode_title, &mode, "all", &"Home".to_string());
                        // to trigger homescreen rebuild
                        let _ = &ui.search_bar.set_text("a");
                        let _ = &ui.search_bar.set_text("");
                    }
                }
                results.focus_first();
            }
            gdk::Key::Return => {
                if let Some(row) = results.selected_row().and_downcast_ref::<SherlockRow>() {
                    let attrs: HashMap<String, String> = get_row_attrs(row);
                    execute_from_attrs(attrs);
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
                    results.focus_prev(&ui.result_viewport);
                    return true.into();
                } else if custom_binds.next_mod == shift && custom_binds.next == tab {
                    results.focus_next(&ui.result_viewport);
                    return true.into();
                }
            }
            _ => (),
        }
        false.into()
    });
    APP_STATE.with(|state| {
        if let Some(ref state) = *state.borrow() {
            state.add_event_listener(event_controller);
        }
    });
}

fn change_event(
    ui: &SearchUI,
    modes: HashMap<String, String>,
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
    );

    ui.search_bar.connect_changed({
        let mode_title_clone = ui.mode_title.clone();
        let launchers_clone = launchers.clone();
        let mode_clone = Rc::clone(mode);
        let results_clone = Rc::clone(results);

        move |search_bar| {
            let mut current_text = search_bar.text().to_string();
            if let Some(task) = current_task.borrow_mut().take() {
                task.abort();
            };
            *cancel_flag.borrow_mut() = true;
            if !current_text.trim().is_empty() && modes.contains_key(&current_text) {
                // Logic to apply modes
                if let Some(mode_name) = modes.get(&current_text) {
                    set_mode(&mode_title_clone, &mode_clone, &current_text, mode_name);
                    // Logic to safely reset the search bar
                    let search_bar_clone = search_bar.clone();
                    glib::idle_add_local(move || {
                        search_bar_clone.set_text("");
                        glib::ControlFlow::Break
                    });
                    current_text.clear();
                }
            }
            async_calc(
                &cancel_flag,
                &current_task,
                &launchers_clone,
                &mode_clone,
                current_text,
                &results_clone,
                &mod_str,
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
) {
    *cancel_flag.borrow_mut() = false;
    // If task is currently running, abort it
    if let Some(t) = current_task.borrow_mut().take() {
        t.abort();
    };
    let cancel_flag = Rc::clone(&cancel_flag);
    let home = current_text.is_empty() && mode.borrow().as_str() == "all";
    let filtered_launchers: Vec<Launcher> = launchers
        .iter()
        .filter(|launcher| (home && launcher.home) || (!home && !launcher.only_home))
        .cloned()
        .collect();
    let (async_launchers, non_async_launchers): (Vec<Launcher>, Vec<Launcher>) = filtered_launchers
        .into_iter()
        .partition(|launcher| launcher.r#async);

    // Create loader widgets
    // TODO
    let current_mode = mode.borrow().trim().to_string();
    let async_widgets: Vec<AsyncLauncherTile> = async_launchers
        .iter()
        .filter_map(|launcher| {
            if (launcher.priority == 0 && current_mode == launcher.alias.as_deref().unwrap_or(""))
                || (current_mode == "all" && launcher.priority > 0)
            {
                launcher.get_loader_widget(&current_text).map(
                    |(widget, title, body, async_opts, attrs)| AsyncLauncherTile {
                        launcher: launcher.clone(),
                        result_item: widget,
                        title,
                        body,
                        async_opts,
                        attrs,
                    },
                )
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
        Some(&async_widgets),
        home,
        mod_str,
    );

    // Gather results for aynchronous widgets

    let task = glib::MainContext::default().spawn_local({
        let current_task_clone = Rc::clone(current_task);
        async move {
            if *cancel_flag.borrow() {
                return;
            }
            // get results for aysnc launchers
            for widget in async_widgets.iter() {
                if let Some((title, body, next_content)) =
                    widget.launcher.get_result(&current_text).await
                {
                    widget.title.as_ref().map(|t| t.set_text(&title));
                    widget.body.as_ref().map(|b| b.set_text(&body));
                    if let Some(next_content) = next_content {
                        let label =
                            Label::new(Some(format!("next_content | {}", next_content).as_str()));
                        widget.attrs.append(&label);
                    }
                }
                if let Some(opts) = &widget.async_opts {
                    // Replace one image with another
                    if let Some(overlay) = &opts.icon_holder_overlay {
                        if let Some((image, was_cached)) = widget.launcher.get_image().await {
                            // Also check for animate key
                            if !was_cached {
                                overlay.add_css_class("image-replace-overlay");
                            }
                            let gtk_image = Image::from_pixbuf(Some(&image));
                            gtk_image.set_widget_name("album-cover");
                            gtk_image.set_pixel_size(50);
                            overlay.add_overlay(&gtk_image);
                        } else {
                            println!("failed to load image");
                        }
                    }
                }
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
    async_launchers: Option<&Vec<AsyncLauncherTile>>,
    home: bool,
    mod_str: &str,
) {
    // Remove all elements inside to avoid duplicates
    let mut launcher_tiles = Vec::new();
    while let Some(row) = results_frame.last_child() {
        results_frame.remove(&row);
    }
    let widgets = construct_tiles(&keyword.to_string(), &launchers, &mode.to_string());
    launcher_tiles.extend(widgets);
    if let Some(a_wid) = async_launchers {
        let widgets: Vec<ResultItem> = a_wid.into_iter().map(|l| l.result_item.clone()).collect();
        launcher_tiles.extend(widgets);
    }

    launcher_tiles.sort_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap());

    if let Some(c) = CONFIG.get() {
        let mut shortcut_index = 1;
        for widget in launcher_tiles {
            if home && c.behavior.animate {
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
