use gio::glib::{object::IsA, variant::ToVariant};
use gtk4::{prelude::WidgetExt, Widget};
use std::collections::HashMap;
use std::fs::File;
use teamslaunch::teamslaunch;
use util::{clear_cached_files, reset_app_counter};

use crate::{
    daemon::daemon::print_reponse,
    launcher::{
        audio_launcher::MusicPlayerLauncher, process_launcher::ProcessLauncher,
        theme_picker::ThemePicker,
    },
    loader::util::CounterReader,
    sherlock_error,
    utils::{errors::SherlockErrorType, files::home_dir},
    CONFIG,
};

pub mod applaunch;
pub mod commandlaunch;
pub mod teamslaunch;
pub mod util;
pub mod websearch;

pub fn execute_from_attrs<T: IsA<Widget>>(
    row: &T,
    attrs: &HashMap<String, String>,
    do_exit: Option<bool>,
) {
    //construct HashMap
    let attrs: HashMap<String, String> = attrs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    if let Some(method) = attrs.get("method") {
        let mut exit = attrs.get("exit").map_or(true, |s| s == "true");

        match method.as_str() {
            "categories" => {
                exit = false;
                attrs.get("exec").map(|mode| {
                    let _ = row.activate_action("win.switch-mode", Some(&mode.to_variant()));
                    let _ = row.activate_action("win.clear-search", None);
                });
            }
            "app_launcher" => {
                let exec = attrs.get("exec").map_or("", |s| s.as_str());
                let term = attrs.get("term").map_or(false, |s| s.as_str() == "true");
                applaunch::applaunch(exec, term);
                increment(&exec);
            }
            "web_launcher" | "bookmarks" => {
                let engine = attrs.get("engine").map_or("plain", |s| s.as_str());
                let query = if let Some(query) = attrs.get("exec") {
                    query.as_str()
                } else if let Some(query) = attrs.get("keyword") {
                    let exec = format!("websearch-{}", engine);
                    increment(&exec);
                    query.as_str()
                } else {
                    ""
                };
                if let Err(error) = websearch::websearch(engine, query) {
                    exit = false;
                    let _result = error.insert();
                }
            }
            "command" => {
                let exec = attrs.get("exec").map_or("", |s| s.as_str());
                let keyword = attrs.get("keyword").map_or("", |s| s.as_str());
                if let Err(error) = commandlaunch::command_launch(exec, keyword) {
                    exit = false;
                    let _result = error.insert();
                } else {
                    increment(&exec);
                }
            }
            "copy" => {
                let field = attrs
                    .get("field")
                    .or(CONFIG.get().and_then(|c| c.behavior.field.as_ref()));
                if let Some(field) = field {
                    if let Some(output) = attrs.get(field) {
                        let _ = util::copy_to_clipboard(output.as_str());
                    }
                } else if let Some(output) = attrs.get("result").or(attrs.get("exec")) {
                    if let Err(err) = util::copy_to_clipboard(output.as_str()) {
                        exit = false;
                        let _result = err.insert();
                    }
                }
            }
            "print" => {
                if let Some(field) = attrs.get("field") {
                    if let Some(output) = attrs.get(field) {
                        let _result = print_reponse(output);
                    }
                } else if let Some(output) = attrs.get("result").or(attrs.get("exec")) {
                    let _result = print_reponse(output);
                }
            }
            "teams_event" => {
                if let Some(meeting) = attrs.get("meeting_url") {
                    match teamslaunch(meeting) {
                        Ok(_) => {
                            let _ = row.activate_action("win.close", None);
                        }
                        Err(_) => {
                            let _ = row.activate_action(
                                "win.switch-page",
                                Some(&String::from("search-page->error-page").to_variant()),
                            );
                        }
                    }
                }
            }
            "emoji_picker" => {
                exit = false;
                let _ = row.activate_action("win.emoji-page", None);
                let _ = row.activate_action(
                    "win.switch-page",
                    Some(&String::from("search-page->emoji-page").to_variant()),
                );
            }
            "theme_picker" => {
                if let Some(theme) = attrs.get("result").or(attrs.get("exec")) {
                    if let Err(error) = ThemePicker::select_theme(theme) {
                        exit = false;
                        let _result = error.insert();
                    }
                } else {
                    exit = false;
                }
            }
            "next" => {
                exit = false;
                let next_content = attrs
                    .get("next_content")
                    .map_or("No next_content provided...", |s| s.trim());

                let _ = row
                    .activate_action("win.add-page", Some(&next_content.to_string().to_variant()));
            }
            "play-pause" | "audio_sink" => {
                if let Some(player) = attrs.get("player") {
                    if let Err(error) = MusicPlayerLauncher::playpause(player) {
                        exit = false;
                        let _result = error.insert();
                    }
                }
            }
            "kill-process" => {
                if let Some((ppid, cpid)) = attrs
                    .get("parent-pid")
                    .and_then(|p| p.parse::<i32>().ok())
                    .zip(attrs.get("child-pid").and_then(|c| c.parse::<i32>().ok()))
                {
                    if let Err(error) = ProcessLauncher::kill((ppid, cpid)) {
                        let _result = error.insert();
                    }
                };
            }
            "debug" => {
                let exec = attrs.get("exec").map_or("", |s| s.as_str());
                match exec {
                    "show_errors" => {
                        exit = false;
                        if let Ok(_) = row.activate_action(
                            "win.switch-page",
                            Some(&String::from("search-page->error-page").to_variant()),
                        ) {
                            increment("debug.show_errors");
                        }
                    }
                    "clear_cache" => {
                        if let Err(error) = clear_cached_files() {
                            let _result = error.insert();
                        } else {
                            increment("debug.clear_cache");
                        }
                    }
                    "reset_counts" => {
                        if let Err(error) = reset_app_counter() {
                            let _result = error.insert();
                        } else {
                            increment("debug.reset_counts");
                        }
                    }
                    "reset_log" => {
                        if let Ok(home) = home_dir() {
                            let file = home.join(".sherlock/sherlock.log");
                            if file.is_file() {
                                if let Err(err) = File::create(&file).map_err(|e| {
                                    sherlock_error!(
                                        SherlockErrorType::FileWriteError(file.clone()),
                                        e.to_string()
                                    )
                                }) {
                                    exit = false;
                                    let _result = err.insert();
                                }
                            }
                        }
                    }
                    "test_error" => {
                        exit = false;
                        let err = sherlock_error!(
                            SherlockErrorType::DebugError(String::from("Test Error")),
                            format!("This is a test error message, it can be disregarded.")
                        );
                        let _result = err.insert();
                    }
                    _ => {}
                }
            }
            "clear_cache" => {
                let _result = clear_cached_files();
            }
            _ => {
                if let Some(out) = attrs.get("result") {
                    let _result = print_reponse(out);
                } else {
                    let out = format!("Return method \"{}\" not recognized", method);
                    let _result = print_reponse(out);
                }
            }
        }

        exit = do_exit.unwrap_or(exit);
        if exit {
            eval_close(row);
        }
    }
}
pub fn get_attrs_map(in_attrs: Vec<(&str, Option<&str>)>) -> HashMap<String, String> {
    in_attrs
        .into_iter()
        .filter_map(|(k, v)| {
            if let (k, Some(v)) = (k, v) {
                Some((k.to_string(), v.to_string()))
            } else {
                None
            }
        })
        .collect()
}
fn increment(key: &str) {
    if let Ok(count_reader) = CounterReader::new() {
        let _ = count_reader.increment(key);
    };
}
fn eval_close<T: IsA<Widget>>(row: &T) {
    let _ = row.activate_action("win.close", None);
}
