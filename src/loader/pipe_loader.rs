use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::os::linux::fs::MetadataExt;

use gtk4::gdk::Display;
use gtk4::IconTheme;
use serde::Deserialize;
use simd_json::base::ValueTryAsMutObject;
use simd_json::OwnedValue;

use crate::api::call::ApiCall;
use crate::utils::config::{ConfigGuard, OtherDefaults};

use super::Loader;

impl Loader {
    pub fn load_pipe_args() -> Vec<u8> {
        if let Ok(metadata) = File::open("/dev/stdin").and_then(|f| f.metadata()) {
            // 0o020000 - Character device (e.g. TTY)
            // 0o170000 - octal mask to extract all file types
            if metadata.st_mode() & 0o170000 == 0o020000 {
                return vec![];
            }
        }
        let stdin = io::stdin();
        let mut buf = Vec::new();
        let _ = stdin.lock().read_to_end(&mut buf);
        return buf;
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PipedElements {
    pub title: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub icon_size: Option<i32>,
    pub result: Option<String>,
    pub binary: Option<Vec<u8>>,
    pub method: Option<String>,
    pub field: Option<String>,
    pub hidden: Option<HashMap<String, String>>,
    #[serde(default = "OtherDefaults::bool_true")]
    pub exit: bool,
}

impl PipedElements {
    pub fn clean(&mut self) {
        if let Some(title) = &self.title {
            let cleaned: String = title
                .chars()
                .filter(|&c| !c.is_control() || c == '\t' || c == '\n')
                .collect();
            self.title = Some(cleaned);
        }
        if let Some(desc) = &self.description {
            let cleaned: String = desc
                .chars()
                .filter(|&c| c.is_ascii() && (!c.is_control() || c == '\t' || c == '\n'))
                .collect();
            self.description = Some(cleaned);
        }
        if let Some(res) = &self.result {
            let cleaned: String = res
                .chars()
                .filter(|&c| c.is_ascii() && (!c.is_control() || c == '\t' || c == '\n'))
                .collect();
            self.result = Some(cleaned);
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PipedData {
    pub settings: Option<Vec<ApiCall>>,
    pub elements: Option<String>,
}
impl PipedData {
    pub fn new<T: AsRef<[u8]>>(msg: T) -> Option<Self> {
        let mut buf = msg.as_ref().to_vec();

        let json: Option<OwnedValue> = simd_json::from_slice(&mut buf).ok();
        if let Some(mut json) = json {
            let obj = json.try_as_object_mut().ok()?;
            // Extract Settings
            let settings = if let Some(settings_val) = obj.remove("settings") {
                simd_json::serde::from_owned_value::<Vec<ApiCall>>(settings_val).ok()
            } else {
                None
            };
            let elements = if !obj.is_empty() {
                simd_json::to_string(&json).ok()
            } else {
                None
            };
            Some(Self { settings, elements })
        } else {
            let elements = String::from_utf8_lossy(&buf).to_string();
            Some(Self {
                settings: None,
                elements: Some(elements),
            })
        }
    }
    pub fn elements<T: AsRef<[u8]>>(msg: T) -> Option<Vec<PipedElements>> {
        let mut buf = msg.as_ref().to_vec();

        let mut json: OwnedValue = simd_json::from_slice(&mut buf).ok()?;
        let obj = json.try_as_object_mut().ok()?;

        // Extract Elements
        let elements_val = obj.remove("elements")?;
        let mut elements =
            simd_json::serde::from_owned_value::<Vec<PipedElements>>(elements_val).ok()?;
        let config = ConfigGuard::read().ok()?;
        for item in elements.iter_mut() {
            item.clean();
            if item.method.is_none() {
                item.method = config.runtime.method.clone();
            }
        }
        Some(elements)
    }
    pub fn deserialize_pipe<T: AsRef<[u8]>>(buf: T) -> Option<Vec<PipedElements>> {
        let config = ConfigGuard::read().ok()?;

        let buf = buf.as_ref().to_vec();

        let icon_theme = IconTheme::for_display(Display::default().as_ref().unwrap());
        let mut result = Vec::new();
        let mut start = 0;
        while start < buf.len() {
            // Detect if the current byte sequence is a valid UTF-8 string
            let end = match buf[start..].iter().position(|&b| b == b'\n') {
                Some(pos) => start + pos + 1,
                None => buf.len(),
            };

            // Try to convert the current chunk to a UTF-8 string
            let chunk = &buf[start..end];

            // Check if the chunk contains valid UTF-8
            if let Ok(line) = std::str::from_utf8(chunk) {
                // Treat it as a normal string (text line)
                let mut raw_meta: Vec<&str> = line.split('\0').collect();
                let name = raw_meta.remove(0).to_string();
                let mut meta_data: HashMap<String, String> = raw_meta
                    .into_iter()
                    .filter_map(|s| {
                        let mut parts = s.split('\x1f');
                        match (parts.next(), parts.next()) {
                            (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                            _ => None,
                        }
                    })
                    .collect();

                let icons: Vec<String> = meta_data
                    .remove("icon")
                    .map(|icon| {
                        icon.split(',')
                            .filter(|name| icon_theme.has_icon(name))
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();

                result.push(PipedElements {
                    title: Some(name.clone()),
                    description: None,
                    result: Some(name),
                    icon: icons.get(0).cloned(),
                    icon_size: None,
                    binary: None,
                    method: None,
                    field: config.runtime.field.clone(),
                    hidden: None,
                    exit: true,
                });
            } else {
                // If it's not valid UTF-8, treat it as binary data
                result.push(PipedElements {
                    title: None,
                    description: None,
                    result: None,
                    icon: None,
                    icon_size: None,
                    binary: Some(chunk.to_vec()),
                    field: config.runtime.field.clone(),
                    method: None,
                    hidden: None,
                    exit: true,
                });
            }

            start = end;
        }

        Some(result)
    }
}
