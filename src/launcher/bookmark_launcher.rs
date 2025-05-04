use rusqlite::Connection;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::loader::util::{AppData, RawLauncher};
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::utils::files::home_dir;

#[derive(Clone, Debug)]
pub struct BookmarkLauncher {
    pub bookmarks: HashSet<AppData>,
}
impl BookmarkLauncher {
    pub fn find_bookmarks(
        browser: &str,
        raw: &RawLauncher,
    ) -> Result<HashSet<AppData>, SherlockError> {
        match browser.to_lowercase().as_str() {
            "zen" | "zen-browser" | "/opt/zen-browser-bin/zen-bin %u" => BookmarkParser::zen(raw),
            "brave" | "brave %u" => BookmarkParser::brave(raw),
            "firefox" | "/usr/lib/firefox/firefox %u" => BookmarkParser::firefox(raw),
            "chrome" | "google-chrome" | "/usr/bin/google-chrome-stable %u" => {
                BookmarkParser::chrome(raw)
            }
            "thorium" | "/usr/bin/thorium-browser %u" => BookmarkParser::thorium(raw),
            _ => {
                println!("{:?}", browser);
                Ok(HashSet::new())
            }
        }
    }
}

struct BookmarkParser;
impl BookmarkParser {
    fn brave(raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        let path = home_dir()?.join(".config/BraveSoftware/Brave-Browser/Default/Bookmarks");
        let data = fs::read_to_string(&path).map_err(|e| SherlockError {
            error: SherlockErrorType::FileReadError(path),
            traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
        })?;

        ChromeParser::parse(raw, data)
    }
    fn thorium(raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        let path = home_dir()?.join(".config/thorium/Default/Bookmarks");
        let data = fs::read_to_string(&path).map_err(|e| SherlockError {
            error: SherlockErrorType::FileReadError(path),
            traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
        })?;
        ChromeParser::parse(raw, data)
    }
    fn chrome(raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        let path = home_dir()?.join(".config/google-chrome/Default/Bookmarks");
        let data = fs::read_to_string(&path).map_err(|e| SherlockError {
            error: SherlockErrorType::FileReadError(path),
            traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
        })?;
        ChromeParser::parse(raw, data)
    }

    fn zen(raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        fn get_path() -> Option<PathBuf> {
            let zen_root = home_dir().ok()?.join(".zen");
            fs::read_dir(&zen_root)
                .ok()?
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    if path.is_dir() && path.join("places.sqlite").exists() {
                        Some(path.join("places.sqlite"))
                    } else {
                        None
                    }
                })
                .next()
        }
        let path = get_path().ok_or_else(|| SherlockError {
            error: SherlockErrorType::FileExistError(PathBuf::from("~/.zen/../places.sqlite")),
            traceback: format!("{}:{}\nFile does not exist.", file!(), line!()),
        })?;
        let parser = MozillaSqliteParser::new(path, "zen");
        parser.read(raw)
    }
    fn firefox(raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        fn get_path() -> Option<PathBuf> {
            let zen_root = home_dir().ok()?.join(".mozilla/firefox/");
            fs::read_dir(&zen_root)
                .ok()?
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    if path.is_dir() && path.join("places.sqlite").exists() {
                        Some(path.join("places.sqlite"))
                    } else {
                        None
                    }
                })
                .next()
        }
        let path = get_path().ok_or_else(|| SherlockError {
            error: SherlockErrorType::FileExistError(PathBuf::from(
                "~/.mozilla/firefox//../places.sqlite",
            )),
            traceback: format!("{}:{}\nFile does not exist.", file!(), line!()),
        })?;
        let parser = MozillaSqliteParser::new(path, "firefox");
        parser.read(raw)
    }
}
struct MozillaSqliteParser {
    path: PathBuf,
}
impl MozillaSqliteParser {
    fn new(file: PathBuf, prefix: &str) -> Self {
        let home = home_dir().ok();
        let path: PathBuf = if let Some(home) = home {
            let target = format!(".cache/sherlock/bookmarks/{}-places.sqlite", prefix);
            let cache_path = home.join(target);
            Self::copy_if_needed(&file, &cache_path);
            cache_path
        } else {
            file.to_path_buf()
        };
        Self { path }
    }
    fn read(&self, raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        let mut res: HashSet<AppData> = HashSet::new();
        let query = "
            SELECT b.title, p.url
            FROM moz_bookmarks b
            JOIN moz_places p ON b.fk = p.id
            WHERE b.type = 1
            AND b.title IS NOT NULL
            AND p.url IS NOT NULL
            AND b.parent != 7;
            ";
        let conn = Connection::open(&self.path).map_err(|e| SherlockError {
            error: SherlockErrorType::SqlConnectionError(),
            traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
        })?;

        if let Ok(mut stmt) = conn.prepare(query) {
            let event_iter = stmt.query_map([], |row| {
                let title: String = row.get(0)?;
                let url: String = row.get(1)?;

                Ok((title, url))
            });

            if let Ok(rows) = event_iter {
                for row in rows.flatten() {
                    let bookmark = AppData {
                        name: row.0.to_string(),
                        icon: None,
                        icon_class: raw
                            .args
                            .get("icon_class")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        exec: row.1,
                        search_string: row.0,
                        tag_start: raw.tag_start.clone(),
                        tag_end: raw.tag_end.clone(),
                        desktop_file: None,
                        priority: raw.priority,
                    };
                    res.insert(bookmark);
                }
            }
        }
        Ok(res)
    }
    fn should_update_cache(path: &PathBuf) -> bool {
        if !path.exists() {
            return true;
        }

        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(mod_time) = metadata.modified() {
                if let Ok(age) = SystemTime::now().duration_since(mod_time) {
                    return age.as_secs() > 1 * 24 * 60 * 60; // older than 2 days
                }
            }
        }
        true
    }
    fn copy_if_needed(src: &PathBuf, dst: &PathBuf) {
        if Self::should_update_cache(dst) {
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(src, dst);
        }
    }
}
struct ChromeParser;
impl ChromeParser {
    fn parse(raw: &RawLauncher, data: String) -> Result<HashSet<AppData>, SherlockError> {
        mod parser {
            use std::collections::HashMap;

            use serde::Deserialize;

            #[derive(Deserialize)]
            pub struct ChromeBookmark {
                pub name: String,
                pub r#type: String,
                pub children: Option<Vec<ChromeBookmark>>,
                pub url: Option<String>,
            }

            #[derive(Deserialize)]
            pub struct ChromeFile {
                pub roots: HashMap<String, ChromeBookmark>,
            }
        }

        let mut bookmarks = HashSet::new();
        let file =
            serde_json::from_str::<parser::ChromeFile>(&data).map_err(|e| SherlockError {
                error: SherlockErrorType::FlagLoadError,
                traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
            })?;

        fn process_bookmark(
            raw: &RawLauncher,
            bookmarks: &mut HashSet<AppData>,
            bookmark: parser::ChromeBookmark,
        ) {
            match bookmark.r#type.as_ref() {
                "folder" => {
                    if let Some(children) = bookmark.children {
                        for child in children {
                            process_bookmark(raw, bookmarks, child);
                        }
                    }
                }
                "url" => {
                    if let Some(url) = bookmark.url {
                        bookmarks.insert(AppData {
                            name: bookmark.name.clone(),
                            icon: None,
                            icon_class: raw
                                .args
                                .get("icon_class")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            exec: url.clone(),
                            search_string: format!("{};{}", bookmark.name, url),
                            tag_start: raw.tag_start.clone(),
                            tag_end: raw.tag_end.clone(),
                            desktop_file: None,
                            priority: raw.priority,
                        });
                    }
                }
                _ => {}
            };
        }

        for (_name, bookmark) in file.roots {
            process_bookmark(raw, &mut bookmarks, bookmark);
        }

        Ok(bookmarks)
    }
}
