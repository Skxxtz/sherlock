use std::collections::HashMap;

use gio::glib::{object::IsA, variant::ToVariant};
use gtk4::{prelude::WidgetExt, Widget};
use teamslaunch::teamslaunch;
use util::{clear_cached_files, reset_app_counter};

use crate::{
    launcher::{audio_launcher::MusicPlayerLauncher, process_launcher::ProcessLauncher},
    loader::util::CounterReader,
};

pub mod applaunch;
pub mod commandlaunch;
pub mod teamslaunch;
pub mod util;
pub mod websearch;

pub fn execute_from_attrs<T: IsA<Widget>>(row: &T, attrs: &HashMap<String, String>) {
    //construct HashMap
    let attrs: HashMap<String, String> = attrs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    if let Some(method) = attrs.get("method") {
        match method.as_str() {
            "categories" => {
                attrs.get("exec").map(|mode| {
                    let _ = row.activate_action("win.switch-mode", Some(&mode.to_variant()));
                });
            }
            "app_launcher" => {
                let exec = attrs.get("exec").map_or("", |s| s.as_str());
                applaunch::applaunch(exec);
                increment(&exec);
                eval_close(row);
            }
            "bookmarks" => {
                let query = attrs.get("exec").map_or("", |s| s.as_str());
                let engine = attrs.get("engine").map_or("plain", |s| s.as_str());
                let _ = websearch::websearch(engine, query);
                eval_close(row);
            }
            "web_launcher" => {
                let query = attrs.get("keyword").map_or("", |s| s.as_str());
                let engine = attrs.get("engine").map_or("", |s| s.as_str());
                let _ = websearch::websearch(engine, query);
                let exec = format!("websearch-{}", engine);
                increment(&exec);
                eval_close(row);
            }
            "command" => {
                let exec = attrs.get("exec").map_or("", |s| s.as_str());
                let keyword = attrs.get("keyword").map_or("", |s| s.as_str());
                let _ = commandlaunch::command_launch(exec, keyword);
                increment(&exec);
                eval_close(row);
            }
            "copy" => {
                if let Some(field) = attrs.get("field") {
                    if let Some(output) = attrs.get(field) {
                        let _ = util::copy_to_clipboard(output.as_str());
                    }
                } else if let Some(result) = attrs.get("result") {
                    let _ = util::copy_to_clipboard(result.as_str());
                }
                eval_close(row);
            }
            "print" => {
                if let Some(field) = attrs.get("field") {
                    if let Some(output) = attrs.get(field) {
                        print!("{}", output);
                    }
                } else if let Some(result) = attrs.get("result") {
                    print!("{}", result);
                }
                eval_close(row);
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
            "next" => {
                let next_content = attrs
                    .get("next_content")
                    .map_or("No next_content provided...", |s| s.trim());

                let _ = row
                    .activate_action("win.add-page", Some(&next_content.to_string().to_variant()));
            }
            "play-pause" | "audio_sink" => {
                attrs
                    .get("player")
                    .map(|player| MusicPlayerLauncher::playpause(player));
            }
            "kill-process" => {
                let _ = attrs
                    .get("parent-pid")
                    .and_then(|p| p.parse::<i32>().ok())
                    .zip(attrs.get("child-pid").and_then(|c| c.parse::<i32>().ok()))
                    .map(|(ppid, cpid)| ProcessLauncher::kill((ppid, cpid)));
                eval_close(row);
            }
            "debug" => {
                let exec = attrs.get("exec").map_or("", |s| s.as_str());
                match exec {
                    "show_errors" => {
                        let _result = row.activate_action(
                            "win.switch-page",
                            Some(&String::from("search-page->error-page").to_variant()),
                        );
                        increment("debug.show_errors");
                    }
                    "clear_cache" => {
                        let _result = clear_cached_files();
                        increment("debug.clear_cache");
                        eval_close(row);
                    }
                    "reset_counts" => {
                        let _result = reset_app_counter();
                        eval_close(row);
                        increment("debug.reset_counts");
                    }
                    _ => {}
                }
            }
            "clear_cache" => {
                let _result = clear_cached_files();
            }
            _ => {
                if let Some(out) = attrs.get("result") {
                    print!("{}", out);
                } else {
                    println!("Return method \"{}\" not recognized", method);
                }
                eval_close(row);
            }
        }
    }
}
pub fn get_attrs_map(in_attrs: Vec<(&str, &str)>) -> HashMap<String, String> {
    in_attrs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
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
