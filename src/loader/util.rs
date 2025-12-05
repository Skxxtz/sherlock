use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    fs::{self, File},
    hash::{Hash, Hasher},
    path::PathBuf,
};

use crate::{
    g_subclasses::sherlock_row::SherlockRowBind,
    launcher::utils::HomeType,
    sherlock_error,
    utils::{
        cache::BinaryCache,
        errors::{SherlockError, SherlockErrorType},
        files::{expand_path, home_dir},
        paths,
    },
};

#[derive(Deserialize, Debug, Serialize)]
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
    pub exit: bool,
    #[serde(default = "default_true")]
    pub shortcut: bool,
    #[serde(default = "default_true")]
    pub spawn_focus: bool,
    #[serde(default)]
    pub r#async: bool,
    #[serde(default)]
    pub home: HomeType,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub binds: Option<Vec<SherlockRowBind>>,
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
    pub fn new_for_theme<'a, T, S>(name: T, path: Option<S>, raw: &RawLauncher) -> Self
    where
        T: Into<Cow<'a, str>>,
        S: Into<Cow<'a, str>>,
    {
        let name: Cow<'a, str> = name.into();
        let path = path.map(|s| s.into().into_owned());
        let name_string = name.into_owned();
        let icon = raw
            .args
            .get("icon")
            .and_then(|s| s.as_str())
            .unwrap_or("sherlock-devtools")
            .to_string();
        Self {
            name: name_string.clone(),
            exec: path,
            search_string: name_string,
            priority: raw.priority,
            icon: Some(icon),
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
    pub fn apply_alias(&mut self, alias: Option<SherlockAlias>, use_keywords: bool) {
        if let Some(alias) = alias {
            if let Some(alias_name) = alias.name.as_ref() {
                self.name = alias_name.to_string();
            }
            if let Some(alias_icon) = alias.icon.as_ref() {
                self.icon = Some(alias_icon.to_string());
            }
            if let Some(alias_keywords) = alias.keywords.as_ref() {
                self.search_string =
                    Self::construct_search(&self.name, &alias_keywords, use_keywords);
            } else {
                self.search_string =
                    Self::construct_search(&self.name, &self.search_string, use_keywords);
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
            self.search_string =
                Self::construct_search(&self.name, &self.search_string, use_keywords);
        }
        println!("{:?}", self.search_string);
    }
    fn construct_search(name: &str, search_str: &str, use_keywords: bool) -> String {
        if use_keywords {
            format!("{};{}", name, search_str)
        } else {
            name.to_string()
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
        let data_dir = paths::get_data_dir()?;
        let path = data_dir.join("counts.bin");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::DirCreateError(parent.to_string_lossy().to_string()),
                    e.to_string()
                )
            })?;
        }
        Ok(CounterReader { path })
    }
    pub fn increment(&self, key: &str) -> Result<(), SherlockError> {
        let mut content: HashMap<String, u32> = BinaryCache::read(&self.path)?;
        let unique_values: HashMap<u32, u32> = content
            .values()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, v)| (v, (i + 1) as u32))
            .collect();

        content.iter_mut().for_each(|(_, v)| {
            if let Some(new) = unique_values.get(v) {
                *v = new.clone();
            }
        });

        *content.entry(key.to_string()).or_insert(0) += 1;
        BinaryCache::write(&self.path, &content)?;
        Ok(())
    }
}

pub struct JsonCache;
impl JsonCache {
    pub fn _write<T>(path: &PathBuf, to: &T) -> Result<(), SherlockError>
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

        let file = File::open(&path).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileExistError(path.clone()),
                e.to_string()
            )
        })?;
        simd_json::from_reader(file)
            .map_err(|e| sherlock_error!(SherlockErrorType::DeserializationError, e.to_string()))
    }
}
