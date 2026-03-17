use gpui::SharedString;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    launcher::{Launcher, LauncherType},
    loader::resolve_icon_path,
    sherlock_error,
    utils::{
        cache::BinaryCache,
        config::HomeType,
        errors::{SherlockError, SherlockErrorType},
        paths,
    },
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationAction {
    pub name: Option<SharedString>,
    pub exec: Option<String>,
    pub icon: Option<Arc<Path>>,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AppData {
    #[serde(default)]
    pub name: Option<SharedString>,
    pub exec: Option<String>,
    pub search_string: String,
    #[serde(default)]
    pub priority: Option<f32>,
    pub icon: Option<Arc<Path>>,
    pub desktop_file: Option<PathBuf>,
    #[serde(default)]
    pub actions: Arc<[Arc<ApplicationAction>]>,
    #[serde(default)]
    #[serde(rename = "variables")]
    pub vars: Vec<ExecVariable>,
    #[serde(default)]
    pub terminal: bool,
}
impl Eq for AppData {}
impl Hash for AppData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Make more efficient and handle error using f32
        self.exec.hash(state);
        self.desktop_file.hash(state);
    }
}
impl AppData {
    pub fn new() -> Self {
        Self {
            name: None,
            exec: None,
            search_string: String::new(),
            priority: None,
            icon: None,
            desktop_file: None,
            actions: Arc::new([]),
            vars: vec![],
            terminal: false,
        }
    }
    pub fn apply_alias(
        &mut self,
        launcher: &Arc<Launcher>,
        alias: Option<SherlockAlias>,
        use_keywords: bool,
        mut buffer: Vec<Arc<ApplicationAction>>,
    ) {
        if let Some(alias) = alias {
            if let Some(alias_name) = alias.name.as_ref() {
                self.name = Some(SharedString::from(alias_name));
            }

            if let Some(alias_icon) = alias.icon.as_ref().map(|i| resolve_icon_path(i)) {
                self.icon = alias_icon;
            }

            let name: Option<&str> = self
                .name
                .as_ref()
                .map(|s| s.as_str())
                .or(launcher.display_name.as_ref().map(|s| s.as_str()));
            if let Some(alias_keywords) = alias.keywords.as_ref() {
                self.search_string = construct_search(name, &alias_keywords, use_keywords);
            } else {
                self.search_string = construct_search(name, &self.search_string, use_keywords);
            }

            if let Some(alias_exec) = alias.exec.as_ref() {
                self.exec = Some(alias_exec.to_string());
            }

            if let Some(add_actions) = alias.add_actions {
                add_actions.into_iter().for_each(|mut a| {
                    if a.icon.is_none() {
                        a.icon = self.icon.clone();
                    }
                    buffer.push(a.into());
                });
            }

            if let Some(actions) = alias.actions {
                self.actions = actions
                    .into_iter()
                    .map(|mut a| {
                        if a.icon.is_none() {
                            a.icon = self.icon.clone();
                        }
                        a.into()
                    })
                    .collect();
            } else {
                self.actions = buffer.into();
            }

            if let Some(variables) = alias.variables {
                self.vars.extend(variables);
            }
        } else {
            let name: Option<&str> = self
                .name
                .as_ref()
                .map(|s| s.as_str())
                .or(launcher.display_name.as_ref().map(|s| s.as_str()));
            self.search_string = construct_search(name, &self.search_string, use_keywords);
        }
    }
    pub fn get_exec(&self, launcher: &Arc<Launcher>) -> Option<String> {
        match &launcher.launcher_type {
            LauncherType::Web(web) => Some(format!("websearch-{}", web.engine)),

            LauncherType::App(_) | LauncherType::Command(_) | LauncherType::Category(_) => {
                self.exec.clone()
            }

            // None-Home Launchers
            LauncherType::Calc(_) => None,
            LauncherType::Event(_) => None,
            _ => None,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SherlockAlias {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub keywords: Option<String>,
    pub actions: Option<Vec<ApplicationAction>>,
    pub add_actions: Option<Vec<ApplicationAction>>,
    pub variables: Option<Vec<ExecVariable>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecVariable {
    StringInput(SharedString),
    PathInput(SharedString),
    PasswordInput(SharedString),
}
impl ExecVariable {
    pub fn placeholder(&self) -> SharedString {
        match self {
            Self::StringInput(s) => s.clone(),
            Self::PathInput(s) => s.clone(),
            Self::PasswordInput(s) => s.clone(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug, Serialize)]
pub struct RawLauncher {
    pub name: Option<String>,
    pub alias: Option<String>,
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
    pub args: Arc<serde_json::Value>,
    #[serde(default)]
    pub actions: Option<Vec<ApplicationAction>>,
    #[serde(default)]
    pub add_actions: Option<Vec<ApplicationAction>>,
    #[serde(default)]
    pub variables: Option<Vec<ExecVariable>>,
}

pub struct CounterReader {
    pub path: PathBuf,
}
impl CounterReader {
    pub fn new() -> Result<Self, SherlockError> {
        let data_dir = paths::get_data_dir()?;
        let path = data_dir.join("counts.bin");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
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
                value.name = Some(SharedString::from(key));
                set.insert(value);
            }
            Ok(set)
        }
    }
    deserializer.deserialize_map(AppDataMapVisitor)
}

pub fn construct_search(name: Option<&str>, search_str: &str, use_keywords: bool) -> String {
    let mut s = if use_keywords {
        let name_val = name.unwrap_or("");
        let mut s = String::with_capacity(name_val.len() + 1 + search_str.len());
        s.push_str(name_val);
        s.push(';');
        s.push_str(search_str);
        s
    } else {
        name.unwrap_or_default().to_string()
    };

    s.make_ascii_lowercase();
    s
}
