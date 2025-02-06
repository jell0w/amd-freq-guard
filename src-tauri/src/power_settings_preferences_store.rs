use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
pub struct PowerSettingIdentifier {
    pub subgroup_guid: String,
    pub setting_guid: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PowerSettingsPreferences {
    pub liked_settings: HashSet<PowerSettingIdentifier>,
}

impl PowerSettingsPreferences {
    fn get_preferences_path() -> PathBuf {
        let mut path = env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        path.push("liked_power_settings.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::get_preferences_path();
        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    fn save(&self) -> Result<(), String> {
        let path = Self::get_preferences_path();
        let json = serde_json::to_string_pretty(&self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn toggle_setting_liked(&mut self, subgroup_guid: &str, setting_guid: &str, is_liked: bool) -> Result<(), String> {
        let identifier = PowerSettingIdentifier {
            subgroup_guid: subgroup_guid.to_string(),
            setting_guid: setting_guid.to_string(),
        };

        if is_liked {
            self.liked_settings.insert(identifier);
        } else {
            self.liked_settings.remove(&identifier);
        }

        self.save()
    }

    pub fn is_setting_liked(&self, subgroup_guid: &str, setting_guid: &str) -> bool {
        let identifier = PowerSettingIdentifier {
            subgroup_guid: subgroup_guid.to_string(),
            setting_guid: setting_guid.to_string(),
        };
        self.liked_settings.contains(&identifier)
    }
}

#[tauri::command]
pub async fn toggle_power_setting_liked(subgroup_guid: &str, setting_guid: &str, is_liked: bool) -> Result<(), String> {
    let mut preferences = PowerSettingsPreferences::load();
    preferences.toggle_setting_liked(subgroup_guid, setting_guid, is_liked)
}

#[tauri::command]
pub async fn get_liked_power_settings() -> Result<PowerSettingsPreferences, String> {
    Ok(PowerSettingsPreferences::load())
} 