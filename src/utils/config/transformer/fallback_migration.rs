use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    launcher::{BindSerde, variant_type::LauncherVariant},
    loader::utils::{ExecVariable, RawLauncher},
    ui::launcher::context_menu::ContextMenuAction,
    utils::config::HomeType,
};

fn default_true() -> bool {
    true
}

pub struct MigrationResult {
    pub launcher: RawLauncher,
    pub logs: Vec<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct LegacyRawLauncher {
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
    pub args: Value,
    #[serde(default)]
    pub binds: Option<Vec<BindSerde>>,
    #[serde(default)]
    pub actions: Option<Arc<Vec<Arc<ContextMenuAction>>>>,
    #[serde(default)]
    pub add_actions: Option<Vec<Arc<ContextMenuAction>>>,
    #[serde(default)]
    pub variables: Option<Vec<LegacyExecVariable>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyExecVariable {
    StringInput(String),
    PasswordInput(String),
}

impl From<LegacyExecVariable> for ExecVariable {
    fn from(value: LegacyExecVariable) -> Self {
        match value {
            LegacyExecVariable::StringInput(s) => ExecVariable::StringInput(s.into()),
            LegacyExecVariable::PasswordInput(s) => ExecVariable::PasswordInput(s.into()),
        }
    }
}

impl LegacyRawLauncher {
    pub fn migrate_args(
        &mut self,
        variant: Option<LauncherVariant>,
        logs: &mut Vec<String>,
        name: &str,
    ) {
        let Some(var) = variant else { return };

        // --- TYPE-SPECIFIC ARGS MIGRATION ---
        let args = &mut self.args;
        match var {
            LauncherVariant::Calculator | LauncherVariant::Clipboard => {
                if let Some(obj) = args.as_object_mut() {
                    if let Some(caps) = obj.get_mut("capabilities").and_then(|c| c.as_array_mut()) {
                        let old_val = json!("colors.all");
                        let new_val = json!("colors");

                        // Check if the old value exists
                        if caps.contains(&old_val) {
                            // Remove all instances of "colors.all"
                            caps.retain(|x| x != &old_val);

                            // Add the simplified "colors" if it's not already there
                            if !caps.contains(&new_val) {
                                caps.push(new_val);
                            }

                            logs.push(format!(
                                "[{}] Renamed 'colors.all' to 'colors' in capabilities.",
                                name
                            ));
                        }
                    }
                }
            }
            LauncherVariant::Commands => {
                if let Some(exec) = args.get_mut("exec").and_then(|e| e.as_str()) {
                    if exec.ends_with('&') {
                        let cleaned = exec.trim_end_matches('&').trim().to_string();
                        args["exec"] = serde_json::Value::String(cleaned);
                        logs.push(format!(
                            "[{}] Removed trailing '&' from command exec.",
                            name
                        ));
                    }
                }
            }
            LauncherVariant::Emoji => {
                if let Some(obj) = args.as_object_mut() {
                    if let Some(value) = obj.remove("default_skin_color") {
                        obj.insert("default_skin_tone".to_string(), value);

                        logs.push(format!(
                            "[{}] Renamed 'default_skin_color' to 'default_skin_tone'.",
                            name
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn migrate_type(&self) -> Option<LauncherVariant> {
        let new_type: Option<LauncherVariant> = serde_json::from_str(&self.r#type.as_str()).ok();
        if new_type.is_some() {
            return new_type;
        }
        match self.r#type.as_str() {
            "app_launcher" => Some(LauncherVariant::Apps),
            "audio_sink" => Some(LauncherVariant::MusicPlayer),
            "bookmarks" => Some(LauncherVariant::Bookmarks),
            "categories" | "category" => Some(LauncherVariant::Categories),
            "clipboard-execution" | "clipboard" => Some(LauncherVariant::Clipboard),
            "command" => Some(LauncherVariant::Commands),
            "emoji_picker" | "emoji" => Some(LauncherVariant::Emoji),
            "weather" => Some(LauncherVariant::Weather),
            "web_launcher" => Some(LauncherVariant::Web),
            "calculation" | "calculator" => Some(LauncherVariant::Calculator),
            "bulk_text" => Some(LauncherVariant::Script),
            "files" => Some(LauncherVariant::Files),
            "teams_event" | "event" => Some(LauncherVariant::Event),
            "debug" => None,
            "theme_picker" | "theme" => None,
            "process" => None,
            "pomodoro" => None,
            _ => None,
        }
    }

    pub fn migrate(mut self) -> MigrationResult {
        let mut logs = Vec::new();
        let name = self.name.clone().unwrap_or_else(|| "Unknown".to_string());

        // 1. Check for removed tags
        if self.tag_start.is_some() || self.tag_end.is_some() {
            logs.push(format!(
                "[{}] Removed legacy tags (tag_start/tag_end).",
                name
            ));
        }

        // 2. Check for removed keybinds
        if self.binds.is_some() {
            logs.push(format!("[{}] Dropped custom keybind(s).", name));
        }

        // 3. Log variable conversion
        if let Some(ref vars) = self.variables {
            logs.push(format!(
                "[{}] Migrated {} variables to new format.",
                name,
                vars.len()
            ));
        }

        let new_type = self.migrate_type();

        self.migrate_args(new_type, &mut logs, &name);

        let launcher = RawLauncher {
            name: self.name,
            alias: self.alias,
            display_name: self.display_name,
            on_return: self.on_return,
            next_content: self.next_content,
            r#type: new_type.unwrap_or_default(),
            priority: self.priority,
            exit: self.exit,
            shortcut: self.shortcut,
            spawn_focus: self.spawn_focus,
            r#async: self.r#async,
            home: self.home,
            binds: self.binds,
            args: Arc::new(self.args),
            actions: self.actions,
            add_actions: self.add_actions,
            variables: self
                .variables
                .map(|vars| vars.into_iter().map(ExecVariable::from).collect()),
        };

        MigrationResult { launcher, logs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_legacy_to_modern_migration() {
        // 1. Create Legacy JSON (Simulating an old config file)
        let legacy_json = json!({
            "name": "Test Launcher",
            "type": "command",
            "priority": 1.0,
            "tag_start": "old_tag", // This should be dropped/logged
            "args": { "cmd": "ls" },
            "variables": [
                { "string_input": "username" }
            ],
            "binds": [] // This should be dropped/logged
        });

        // 2. Deserialize into Legacy struct
        let legacy: LegacyRawLauncher =
            serde_json::from_value(legacy_json).expect("Failed to parse legacy JSON");

        // 3. Perform Migration
        let result = legacy.migrate();

        // 4. Assertions
        assert_eq!(result.launcher.name, Some("Test Launcher".to_string()));
        assert_eq!(result.launcher.r#type, LauncherVariant::Commands);

        // Check if Arc wrapping worked
        assert_eq!(result.launcher.args.get("cmd").unwrap(), "ls");

        // Verify Variable migration
        if let Some(vars) = result.launcher.variables {
            assert_eq!(vars.len(), 1);
            // Ensure the inner enum converted correctly
            match &vars[0] {
                ExecVariable::StringInput(s) => assert_eq!(s, "username"),
                _ => panic!("Variable type mismatch!"),
            }
        } else {
            panic!("Variables were lost during migration!");
        }

        println!("{:?}", result.logs.join("\n"));

        // Verify Logs caught the missing fields
        assert!(
            result
                .logs
                .iter()
                .any(|l| l.contains("Removed legacy tags"))
        );
    }
}
