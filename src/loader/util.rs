use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Debug,
    fs::{self, File},
    hash::{Hash, Hasher},
    path::PathBuf,
};

use crate::utils::{
    errors::{SherlockError, SherlockErrorType},
    files::{expand_path, home_dir},
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
}
fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppData {
    #[serde(default)]
    pub name: String,
    pub icon: Option<String>,
    pub icon_class: Option<String>,
    pub exec: String,
    pub search_string: String,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub desktop_file: Option<PathBuf>,
    #[serde(default)]
    pub priority: f32,
}
impl AppData {
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
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
}

pub struct CounterReader {
    pub path: PathBuf,
}
impl CounterReader {
    pub fn new() -> Result<Self, SherlockError> {
        let home = env::var("HOME").map_err(|e| SherlockError {
            error: SherlockErrorType::EnvVarNotFoundError("HOME".to_string()),
            traceback: e.to_string(),
        })?;
        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".sherlock/counts.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| SherlockError {
                error: SherlockErrorType::DirCreateError(".sherlock".to_string()),
                traceback: e.to_string(),
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
        let home = home_dir()?;
        let path = expand_path(path, &home);

        let file = if path.exists() {
            File::open(&path)
        } else {
            println!("{:?}", path);
            File::create(&path)
        }
        .map_err(|e| SherlockError {
            error: SherlockErrorType::FileExistError(path.clone()),
            traceback: e.to_string(),
        })?;
        let res: Result<T, simd_json::Error> = simd_json::from_reader(file);
        Ok(res.unwrap_or_default())
    }
}
