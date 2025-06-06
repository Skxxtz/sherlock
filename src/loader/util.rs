use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Debug,
    fs::{self, File},
    hash::{Hash, Hasher},
    path::PathBuf,
};

use crate::{
    sherlock_error,
    utils::{
        errors::{SherlockError, SherlockErrorType},
        files::{expand_path, home_dir}, // Assuming these are external utility functions
    },
};

#[derive(Deserialize, Debug)]
pub struct RawLauncher {
    pub name: Option<String>,
    pub alias: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub display_name: Option<String>,
    pub on_return: Option<String>,
    pub next_content: Option<String>,
    pub r#type: String,
    pub priority: f32,

    #[serde(default = "default_true")]
    pub shortcut: bool,
    #[serde(default = "default_true")]
    pub spawn_focus: bool,
    #[serde(default)]
    pub r#async: bool,
    #[serde(default)]
    pub home: bool,
    #[serde(default)]
    pub only_home: bool,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub actions: Option<Vec<ApplicationAction>>,
    #[serde(default)]
    pub add_actions: Option<Vec<ApplicationAction>>,
}
fn default_true() -> bool {
    true
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationAction {
    pub name: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub method: String,
    #[serde(default = "default_true")]
    pub exit: bool,
}
impl ApplicationAction {
    pub fn new(method: &str) -> Self {
        Self {
            name: None,
            exec: None,
            icon: None,
            method: method.to_string(),
            exit: true,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.name.is_some() && self.exec.is_some()
    }
    pub fn is_full(&self) -> bool {
        self.name.is_some() && self.exec.is_some() && self.icon.is_some()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppData {
    #[serde(default)]
    pub name: String,
    pub exec: Option<String>,
    pub search_string: String,
    #[serde(default)]
    pub priority: f32,
    pub icon: Option<String>,
    pub icon_class: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub desktop_file: Option<PathBuf>,
    #[serde(default)]
    pub actions: Vec<ApplicationAction>,
    #[serde(default)]
    pub terminal: bool,
}
impl AppData {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            exec: None,
            search_string: String::new(),
            priority: 0.0,
            icon: None,
            icon_class: None,
            tag_start: None,
            tag_end: None,
            desktop_file: None,
            actions: vec![],
            terminal: false,
        }
    }
    pub fn from_raw_launcher(raw: &RawLauncher) -> Self {
        let mut data = Self::new();
        data.priority = raw.priority;
        data.name = raw.name.as_deref().unwrap_or("").to_string();
        data.icon = raw
            .args
            .get("icon")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        data.icon_class = raw
            .args
            .get("icon_class")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        data.tag_start = raw.tag_start.clone();
        data.tag_end = raw.tag_end.clone();
        data.actions = raw.actions.clone().unwrap_or(vec![]);
        let search = format!(
            "{};{}",
            raw.name.as_deref().unwrap_or(""),
            raw.args
                .get("search_string")
                .and_then(Value::as_str)
                .unwrap_or("")
        );
        data.search_string = search;
        data
    }
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }
    pub fn apply_alias(&mut self, alias: Option<SherlockAlias>) {
        if let Some(alias) = alias {
            if let Some(alias_name) = alias.name.as_ref() {
                self.name = alias_name.to_string();
            }
            if let Some(alias_icon) = alias.icon.as_ref() {
                self.icon = Some(alias_icon.to_string());
            }
            if let Some(alias_keywords) = alias.keywords.as_ref() {
                self.search_string = format!("{};{}", self.name, alias_keywords)
            } else {
                self.search_string = format!("{};{}", self.name, self.search_string);
            }
            if let Some(alias_exec) = alias.exec.as_ref() {
                self.exec = Some(alias_exec.to_string());
            }
            if let Some(add_actions) = alias.add_actions {
                add_actions.into_iter().for_each(|mut a| {
                    if a.icon.is_none() {
                        a.icon = self.icon.clone();
                    }
                    self.actions.push(a);
                });
            }
            if let Some(actions) = alias.actions {
                self.actions = actions
                    .into_iter()
                    .map(|mut a| {
                        if a.icon.is_none() {
                            a.icon = self.icon.clone();
                        }
                        a
                    })
                    .collect();
            }
        } else {
            self.search_string = format!("{};{}", self.name, self.search_string);
        }
    }
}
impl Eq for AppData {}
impl Hash for AppData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Make more efficient and handle error using f32
        self.exec.hash(state);
        self.desktop_file.hash(state);
    }
}

/// Custom deserializer to deserialize named json struct into a hashset instead of hashmap
pub fn deserialize_named_appdata<'de, D>(deserializer: D) -> Result<HashSet<AppData>, D::Error>
where
    D: Deserializer<'de>,
{
    struct AppDataMapVisitor;
    impl<'de> Visitor<'de> for AppDataMapVisitor {
        type Value = HashSet<AppData>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map of AppData keyed by 'name'")
        }
        fn visit_map<M>(self, mut map: M) -> Result<HashSet<AppData>, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut set = HashSet::new();
            while let Some((key, mut value)) = map.next_entry::<String, AppData>()? {
                value.name = key;
                set.insert(value);
            }
            Ok(set)
        }
    }
    deserializer.deserialize_map(AppDataMapVisitor)
}

#[derive(Deserialize, Clone, Debug)]
pub struct SherlockAlias {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub keywords: Option<String>,
    pub actions: Option<Vec<ApplicationAction>>,
    pub add_actions: Option<Vec<ApplicationAction>>,
}

pub struct CounterReader {
    pub path: PathBuf,
}
impl CounterReader {
    pub fn new() -> Result<Self, SherlockError> {
        let cache_home = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                // Fallback to ~/.cache if XDG_CACHE_HOME is not set
                env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"))
            })
            .ok_or_else(|| {
                sherlock_error!(
                    SherlockErrorType::EnvVarNotFoundError("HOME or XDG_CACHE_HOME".to_string()),
                    "Neither HOME nor XDG_CACHE_HOME environment variable found".to_string()
                )
            })?;

        let path = cache_home.join("sherlock/counts.json");

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::DirCreateError(parent.display().to_string()), // Improved error message
                    e.to_string()
                )
            })?;
        }
        Ok(CounterReader { path })
    }
    pub fn increment(&self, key: &str) -> Result<(), SherlockError> {
        let mut content: HashMap<String, f32> = JsonCache::read(&self.path)?;
        *content.entry(key.to_string()).or_insert(0.0) += 1.0;
        JsonCache::write(&self.path, &content)?;
        Ok(())
    }
}

pub struct JsonCache;
impl JsonCache {
    pub fn write<T>(path: &PathBuf, to: &T) -> Result<(), SherlockError>
    where
        T: serde::Serialize + ?Sized,
    {
        let tmp_path = path.with_extension(".tmp");
        if let Ok(f) = File::create(&tmp_path) {
            if let Ok(_) = simd_json::to_writer(f, to) {
                let _ = fs::rename(&tmp_path, &path);
            } else {
                let _ = fs::remove_file(&tmp_path);
            }
        }
        Ok(())
    }
    pub fn read<T>(path: &PathBuf) -> Result<T, SherlockError>
    where
        T: DeserializeOwned + Default + Debug,
    {
        // The `home_dir()` and `expand_path()` utilities are used here.
        // Assuming they correctly resolve the home directory and expand paths.
        // For consistency with XDG, ensure that if path already respects XDG_CACHE_HOME,
        // expand_path doesn't inadvertently re-base it to ~/.cache.
        // If `path` is already derived from XDG_CACHE_HOME, `expand_path` might not be strictly needed here.
        let home = home_dir()?;
        let path = expand_path(path, &home);

        let file = if path.exists() {
            File::open(&path)
        } else {
            // Note: Creating the file here might be premature if it's expected to be written to later.
            // Consider if this `File::create` should only happen on `write` operations.
            println!("{:?}", path);
            File::create(&path)
        }
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileExistError(path.clone()),
                e.to_string()
            )
        })?;
        let res: Result<T, simd_json::Error> = simd_json::from_reader(file);
        Ok(res.unwrap_or_default())
    }
}
