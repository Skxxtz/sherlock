use chrono::{DateTime, Local, Utc};
use rusqlite::Connection;
use std::fs::create_dir_all;
use std::{
    env::{self, home_dir},
    fs,
    path::{Path, PathBuf},
};

use crate::loader::application_loader::file_has_changed;
use crate::utils::config::ConfigGuard;

#[derive(Clone, Debug)]
pub struct TeamsEvent {
    pub title: String,
    pub meeting_url: String,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Clone, Debug)]
pub struct EventLauncher {
    pub event: Option<TeamsEvent>,
    pub icon: String,
}

impl EventLauncher {
    pub fn get_event(date: &str, event_start: &str, event_end: &str) -> Option<TeamsEvent> {
        let config = ConfigGuard::read().ok()?;
        let calendar_client = config.default_apps.calendar_client.as_ref();
        match calendar_client {
            "thunderbird" => {
                let thunderbird_manager = ThunderBirdEventManager::new()?;
                if let Some(path) = &thunderbird_manager.database_path {
                    match Connection::open(Path::new(path)) {
                        Ok(conn) => {
                            if let Some((meeting_url, title, start_time, end_time)) =
                                thunderbird_manager.get_teams_event_by_time(
                                    &conn,
                                    date,
                                    event_start,
                                    event_end,
                                )
                            {
                                return Some(TeamsEvent {
                                    title,
                                    meeting_url: meeting_url.to_string(),
                                    start_time,
                                    end_time,
                                });
                            }
                        }
                        Err(_) => return None,
                    }
                }
            }
            _ => {}
        }
        return None;
    }
}

struct ThunderBirdEventManager {
    database_path: Option<PathBuf>,
}
impl ThunderBirdEventManager {
    pub fn new() -> Option<Self> {
        let home = env::var("HOME").ok().map(PathBuf::from);
        if let Some(home) = home {
            let thunderbird_dir = home.join(".thunderbird");
            match fs::read_dir(&thunderbird_dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir()
                            && path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .map(|n| n.ends_with(".default-release"))
                                .unwrap_or(false)
                        {
                            let database_path = path.join("calendar-data").join("cache.sqlite");
                            // check if cached database exists
                            let cached_path =
                                home_dir()?.join(".cache/sherlock/calendar/cache.sqlite");
                            // check if cached database was modified later than the uncached one
                            let changed = if !cached_path.exists() {
                                if let Some(parent) = cached_path.parent() {
                                    let _ = create_dir_all(parent);
                                }
                                true
                            } else {
                                file_has_changed(&database_path, &cached_path)
                            };
                            if changed {
                                let _ = std::fs::copy(database_path, &cached_path);
                            }

                            return Some(Self {
                                database_path: Some(cached_path),
                            });
                        }
                    }
                }
                Err(_) => {
                    return Some(Self {
                        database_path: None,
                    })
                }
            }
        }
        Some(Self {
            database_path: None,
        })
    }

    pub fn get_teams_event_by_time(
        &self,
        conn: &Connection,
        date: &str,
        event_start: &str,
        event_end: &str,
    ) -> Option<(String, String, String, String)> {
        let query = format!(
            "
                SELECT
                    e.id,
                    e.title,
                    e.event_start,
                    e.event_end,
                    p.value AS meeting_url
                FROM
                    cal_events e
                JOIN
                    cal_properties p
                    ON e.id = p.item_id
                WHERE
                    p.key = 'X-MICROSOFT-SKYPETEAMSMEETINGURL'
                    AND e.event_start BETWEEN strftime('%s', '{}', '{}') * 1000000
                                          AND strftime('%s', '{}', '{}', 'start of day') * 1000000
                ORDER BY
                    e.event_start;
                ",
            date, event_start, date, event_end
        );

        if let Ok(mut stmt) = conn.prepare(&query) {
            let event_iter = stmt.query_map([], |row| {
                let title: String = row.get(1).unwrap_or(String::from("untitled"));
                let start_time: i64 = row.get(2)?;
                let end_time: i64 = row.get(3)?;
                let url: String = row.get(4)?;
                Ok((title, start_time, end_time, url))
            });

            if let Ok(rows) = event_iter {
                if let Some(row) = rows.flatten().nth(0) {
                    let t1 = row.1 / 1_000_000;
                    let t2 = row.2 / 1_000_000;

                    let start_datetime: DateTime<Utc> = DateTime::from_timestamp(t1, 0)?;
                    let end_datetime: DateTime<Utc> = DateTime::from_timestamp(t2, 0)?;

                    let start_time = start_datetime.with_timezone(&Local);
                    let end_time = end_datetime.with_timezone(&Local);

                    let event_start = start_time.format("%H:%M").to_string();
                    let event_end = end_time.format("%H:%M").to_string();

                    return Some((row.3, row.0, event_start, event_end));
                }
            }
        }

        return None;
    }
}
