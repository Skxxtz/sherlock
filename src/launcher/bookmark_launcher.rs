use lz4_flex::block::decompress_size_prepended;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::loader::util::{AppData, RawLauncher};
use crate::ui::util::truncate_if_needed;
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
            _ => {
                // @BaxoPlenty you can check here what this ↑ should be.
                println!("{:?}", browser);
                BookmarkParser::zen(raw)
            }
        }
    }
}

struct BookmarkParser;

impl BookmarkParser {
    fn chrome_internal(raw: &RawLauncher, data: String) -> Result<HashSet<AppData>, SherlockError> {
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
                        let name = truncate_if_needed(bookmark.name.clone(), 72);

                        bookmarks.insert(AppData {
                            name: name.clone(),
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

    fn brave(raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        let path = home_dir()
            .unwrap_or("~/".into())
            .join(".config/BraveSoftware/Brave-Browser/Default/Bookmarks");
        let data = fs::read_to_string(&path).map_err(|e| SherlockError {
            error: SherlockErrorType::FileReadError(path),
            traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
        })?;

        Self::chrome_internal(raw, data)
    }

    fn zen(raw: &RawLauncher) -> Result<HashSet<AppData>, SherlockError> {
        let path = get_path()?;

        let data = fs::read(&path).map_err(|e| SherlockError {
            error: SherlockErrorType::FileReadError(path.clone()),
            traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
        })?;

        if &data[..8] != b"mozLz40\0" {
            return Err(SherlockError {
                error: SherlockErrorType::FileReadError(path.clone()),
                traceback: format!("{}:{}\nInvalid JSONLZ4 header", file!(), line!()),
            });
        }

        let decompressed = decompress_size_prepended(&data[8..]).map_err(|e| SherlockError {
            error: SherlockErrorType::FileReadError(path.clone()),
            traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
        })?;

        let json_value: Value =
            serde_json::from_slice(&decompressed).map_err(|e| SherlockError {
                error: SherlockErrorType::FileParseError(path.clone()),
                traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
            })?;
        let mut bookmarks = HashSet::new();
        if let Some(children) = json_value["children"].as_array() {
            for folder in children.iter().skip(1) {
                extract_bookmarks(&folder, &mut bookmarks, raw);
            }
        }

        fn get_path() -> Result<PathBuf, SherlockError> {
            let zen_root = home_dir()?.join(".zen");
            let backup_dir = fs::read_dir(&zen_root)
                .map_err(|e| SherlockError {
                    error: SherlockErrorType::DirReadError(String::from("~/.zen")),
                    traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
                })?
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    if path.is_dir() && path.join("bookmarkbackups").exists() {
                        Some(path.join("bookmarkbackups"))
                    } else {
                        None
                    }
                })
                .next()
                .ok_or_else(|| SherlockError {
                    error: SherlockErrorType::DirReadError(String::from("~/.zen/")),
                    traceback: format!(
                        "{}:{}\nFailed to find 'bookmarkbackups' child directory",
                        file!(),
                        line!()
                    ),
                })?;

            let mut backups: Vec<_> = fs::read_dir(&backup_dir)
                .map_err(|e| SherlockError {
                    error: SherlockErrorType::DirReadError(String::from("~/.zen")),
                    traceback: format!("{}:{}\n{}", file!(), line!(), e.to_string()),
                })?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.file_name()?.to_str()?.starts_with("bookmarks-") {
                        Some((entry.metadata().ok()?.modified().ok()?, path))
                    } else {
                        None
                    }
                })
                .collect();

            backups.sort_by(|a, b| b.0.cmp(&a.0));
            backups
                .first()
                .map(|(_, path)| path.clone())
                .ok_or_else(|| SherlockError {
                    error: SherlockErrorType::DirReadError(String::from("~/.zen/")),
                    traceback: format!("{}:{}\nFailed to find bookmark backups", file!(), line!()),
                })
        }
        fn deserialize_bookmark(bookmark: &Value) -> Option<(String, String)> {
            if let (Some(title), Some(url)) = (bookmark["title"].as_str(), bookmark["uri"].as_str())
            {
                return Some((title.to_string(), url.to_string()));
            }
            None
        }

        fn extract_bookmarks(
            value: &serde_json::Value,
            out: &mut HashSet<AppData>,
            raw: &RawLauncher,
        ) {
            if let Some(children) = value["children"].as_array() {
                for child in children {
                    if let Some((title, url)) = deserialize_bookmark(child) {
                        if !title.is_empty() {
                            let bookmark = AppData {
                                name: title.to_string(),
                                icon: None,
                                icon_class: raw
                                    .args
                                    .get("icon_class")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                exec: url,
                                search_string: title,
                                tag_start: raw.tag_start.clone(),
                                tag_end: raw.tag_end.clone(),
                                desktop_file: None,
                                priority: raw.priority,
                            };
                            out.insert(bookmark);
                        }
                    } else {
                        extract_bookmarks(child, out, raw);
                    }
                }
            }
        }
        Ok(bookmarks)
    }
}
