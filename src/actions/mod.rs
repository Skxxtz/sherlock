use std::collections::HashMap;

use gio::glib::variant::ToVariant;
use gtk4::prelude::WidgetExt;
use teamslaunch::teamslaunch;
use util::eval_exit;

use crate::{
    g_subclasses::sherlock_row::SherlockRow,
    launcher::{audio_launcher::MusicPlayerLauncher, process_launcher::ProcessLauncher},
    loader::launcher_loader::CounterReader,
    ui::user::{display_next, display_raw},
};

pub mod applaunch;
pub mod commandlaunch;
pub mod teamslaunch;
pub mod util;
pub mod websearch;

pub fn execute_from_attrs(row: &SherlockRow, attrs: &HashMap<String, String>) {
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
                let _ = applaunch::applaunch(exec);
                increment(&exec);
                eval_exit();
            }
            "web_launcher" => {
                let query = attrs.get("keyword").map_or("", |s| s.as_str());
                let engine = attrs.get("engine").map_or("", |s| s.as_str());
                let _ = websearch::websearch(engine, query);
                let exec = format!("websearch-{}", engine);
                increment(&exec);
                eval_exit();
            }
            "command" => {
                let exec = attrs.get("exec").map_or("", |s| s.as_str());
                let keyword = attrs.get("keyword").map_or("", |s| s.as_str());
                let _ = commandlaunch::command_launch(exec, keyword);
                increment(&exec);
                eval_exit();
            }
            "copy" => {
                if let Some(field) = attrs.get("field") {
                    if let Some(output) = attrs.get(field) {
                        let _ = util::copy_to_clipboard(output.as_str());
                    }
                } else if let Some(result) = attrs.get("result") {
                    let _ = util::copy_to_clipboard(result.as_str());
                }
                eval_exit();
            }
            "print" => {
                if let Some(field) = attrs.get("field") {
                    if let Some(output) = attrs.get(field) {
                        print!("{}", output);
                    }
                } else if let Some(result) = attrs.get("result") {
                    print!("{}", result);
                }
                eval_exit();
            }
            "teams_event" => {
                if let Some(meeting) = attrs.get("meeting_url") {
                    teamslaunch(meeting);
                }
                eval_exit();
            }
            "next" => {
                let next_content = attrs
                    .get("next_content")
                    .map_or("No next_content provided...", |s| s);
                display_next(next_content);
            }
            "display_raw" => {
                if let Some(next_content) = attrs.get("next_content") {
                    display_raw(next_content, false);
                }
            }
            "play-pause" | "audio_sink" => {
                if let Some(player) = attrs.get("player") {
                    let _ = MusicPlayerLauncher::playpause(player);
                }
            }
            "kill-process" => {
                let _ = attrs
                    .get("parent-pid")
                    .and_then(|p| p.parse::<i32>().ok())
                    .zip(attrs.get("child-pid").and_then(|c| c.parse::<i32>().ok()))
                    .map(|(ppid, cpid)| ProcessLauncher::kill((ppid, cpid)));
                eval_exit();
            }
            _ => {
                if let Some(out) = attrs.get("result") {
                    print!("{}", out);
                } else {
                    println!("Return method \"{}\" not recognized", method);
                }
                eval_exit();
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
