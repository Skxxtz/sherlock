use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::loader::application_loader::file_has_changed;
use crate::loader::util::{AppData, RawLauncher};
use crate::utils::cache::BinaryCache;
use crate::utils::errors::{SherlockError, SherlockErrorType};
use crate::utils::files::home_dir;
use crate::utils::paths::get_cache_dir;
use crate::{sher_log, sherlock_error};

#[derive(Clone, Debug)]
pub struct BookmarkLauncher {
    pub bookmarks: Arc<Vec<AppData>>,
}
impl BookmarkLauncher {
    pub fn find_bookmarks(
        browser: &str,
        raw: &RawLauncher,
    ) -> Result<Arc<Vec<AppData>>, SherlockError> {
        match browser.to_lowercase().as_str() {
            "zen" | "zen-browser" | "/opt/zen-browser-bin/zen-bin %u" => BookmarkParser::zen(raw),
            "brave" | "brave %u" => BookmarkParser::brave(raw),
            "firefox" | "/usr/lib/firefox/firefox %u" => BookmarkParser::firefox(raw),
            "chrome" | "google-chrome" | "/usr/bin/google-chrome-stable %u" => {
                BookmarkParser::chrome(raw)
            }
            "thorium" | "/usr/bin/thorium-browser %u" => BookmarkParser::thorium(raw),
            _ => {
                sher_log!(format!(
                    r#"Failed to gather bookmarks for browser: "{}""#,
                    browser
                ))?;
                Err(sherlock_error!(
                    SherlockErrorType::UnsupportedBrowser(browser.to_string()),
                    format!("The browser \"<i>{}</i>\" is either not supported or not recognized.\n\
                        Check the \
                        <span foreground=\"#247BA0\"><u><a href=\"https://github.com/Skxxtz/sherlock/blob/main/docs/launchers.md#bookmark-launcher\">documentation</a></u></span> \
                        for more information.\n\
                        ", browser)
                ))
            }
        }
    }
}

struct BookmarkParser;
impl BookmarkParser {
    fn brave(raw: &RawLauncher) -> Result<Arc<Vec<AppData>>, SherlockError> {
        let path = home_dir()?.join(".config/BraveSoftware/Brave-Browser/Default/Bookmarks");
        let data = fs::read_to_string(&path)
            .map_err(|e| sherlock_error!(SherlockErrorType::FileReadError(path), e.to_string()))?;

        ChromeParser::parse(raw, data)
    }
    fn thorium(raw: &RawLauncher) -> Result<Arc<Vec<AppData>>, SherlockError> {
        let path = home_dir()?.join(".config/thorium/Default/Bookmarks");
        let data = fs::read_to_string(&path)
            .map_err(|e| sherlock_error!(SherlockErrorType::FileReadError(path), e.to_string()))?;
        ChromeParser::parse(raw, data)
    }
    fn chrome(raw: &RawLauncher) -> Result<Arc<Vec<AppData>>, SherlockError> {
        let path = home_dir()?.join(".config/google-chrome/Default/Bookmarks");
        let data = fs::read_to_string(&path)
            .map_err(|e| sherlock_error!(SherlockErrorType::FileReadError(path), e.to_string()))?;
        ChromeParser::parse(raw, data)
    }

    fn zen(raw: &RawLauncher) -> Result<Arc<Vec<AppData>>, SherlockError> {
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
        let path = get_path().ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::FileExistError(PathBuf::from("~/.zen/../places.sqlite")),
                "File does not exist"
            )
        })?;
        let parser = MozillaSqliteParser::new(path, "zen");
        parser.read(raw, "zen")
    }
    fn firefox(raw: &RawLauncher) -> Result<Arc<Vec<AppData>>, SherlockError> {
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
        let path = get_path().ok_or_else(|| {
            sherlock_error!(
                SherlockErrorType::FileExistError(PathBuf::from(
                    "~/.mozilla/firefox/../places.sqlite",
                )),
                "File does not exist"
            )
        })?;
        let parser = MozillaSqliteParser::new(path, "firefox");
        parser.read(raw, "firefox")
    }
}
struct MozillaSqliteParser {
    path: PathBuf,
}
impl MozillaSqliteParser {
    fn new(file: PathBuf, prefix: &str) -> Self {
        let path = if let Ok(cache) = get_cache_dir() {
            let target = cache.join(format!("bookmarks/{}-places.sqlite", prefix));
            Self::copy_if_needed(&file, &target);
            target
        } else {
            file.to_path_buf()
        };
        Self { path }
    }
    fn read(&self, raw: &RawLauncher, prefix: &str) -> Result<Arc<Vec<AppData>>, SherlockError> {
        let cache_dir = get_cache_dir()?;
        let cache = cache_dir.join(format!("bookmarks/{}-cache.bin", prefix));

        if !file_has_changed(&cache, &self.path) {
            if let Ok(mut app_data) = BinaryCache::read::<Vec<AppData>, _>(&cache) {
                let icon_class = raw.args.get("icon_class").and_then(|v| v.as_str());

                app_data.iter_mut().for_each(|ad| {
                    ad.icon_class = icon_class.map(|s| s.to_string());
                    if raw.tag_start.is_some() {
                        ad.tag_start = raw.tag_start.clone();
                    }
                    if raw.tag_end.is_some() {
                        ad.tag_end = raw.tag_end.clone();
                    }
                    ad.priority = raw.priority + 1.0;
                });
                return Ok(Arc::new(app_data));
            }
        }

        let bookmarks = Arc::new(self.read_new(raw)?);
        tokio::spawn({
            let bookmarks = Arc::clone(&bookmarks);
            async move {
                let _ = BinaryCache::write(&cache, &*bookmarks);
            }
        });
        Ok(bookmarks)
    }
    fn read_new(&self, raw: &RawLauncher) -> Result<Vec<AppData>, SherlockError> {
        let mut res: Vec<AppData> = Vec::new();
        let query = "
            SELECT b.title, p.url
            FROM moz_bookmarks b
            JOIN moz_places p ON b.fk = p.id
            WHERE b.type = 1
            AND b.title IS NOT NULL
            AND p.url IS NOT NULL
            AND b.parent != 7;
            ";
        let conn = Connection::open(&self.path)
            .map_err(|e| sherlock_error!(SherlockErrorType::SqlConnectionError(), e.to_string()))?;

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
                        exec: Some(row.1),
                        search_string: row.0,
                        tag_start: raw.tag_start.clone(),
                        tag_end: raw.tag_end.clone(),
                        desktop_file: None,
                        priority: raw.priority + 1.0,
                        actions: vec![],
                        vars: vec![],
                        terminal: false,
                    };
                    res.push(bookmark);
                }
            }
        }
        Ok(res)
    }
    fn should_update_cache(dest: &PathBuf, source: &PathBuf) -> bool {
        if !dest.exists() {
            return true;
        }

        let source_mod = fs::metadata(source)
            .ok()
            .and_then(|meta| meta.modified().ok());
        let dest_mod = fs::metadata(dest)
            .ok()
            .and_then(|meta| meta.modified().ok());

        if let (Some(source), Some(dest)) = (source_mod, dest_mod) {
            return source > dest;
        }
        true
    }
    fn copy_if_needed(src: &PathBuf, dst: &PathBuf) {
        if Self::should_update_cache(dst, src) {
            let _ = sher_log!(format!(
                r#"Bookmark database "{}" is copied to "{}""#,
                src.display(),
                dst.display()
            ));
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(src, dst);
        }
    }
}
struct ChromeParser;
impl ChromeParser {
    fn parse(raw: &RawLauncher, data: String) -> Result<Arc<Vec<AppData>>, SherlockError> {
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

        let mut bookmarks = Vec::new();
        let file = serde_json::from_str::<parser::ChromeFile>(&data)
            .map_err(|e| sherlock_error!(SherlockErrorType::FlagLoadError, e.to_string()))?;

        fn process_bookmark(
            raw: &RawLauncher,
            bookmarks: &mut Vec<AppData>,
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
                        bookmarks.push(AppData {
                            name: bookmark.name.clone(),
                            icon: None,
                            icon_class: raw
                                .args
                                .get("icon_class")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            exec: Some(url.clone()),
                            search_string: format!("{};{}", bookmark.name, url),
                            tag_start: raw.tag_start.clone(),
                            tag_end: raw.tag_end.clone(),
                            desktop_file: None,
                            priority: raw.priority + 1.0,
                            actions: vec![],
                            vars: vec![],
                            terminal: false,
                        });
                    }
                }
                _ => {}
            };
        }

        for (_name, bookmark) in file.roots {
            process_bookmark(raw, &mut bookmarks, bookmark);
        }

        Ok(Arc::new(bookmarks))
    }
}
