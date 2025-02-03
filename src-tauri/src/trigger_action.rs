use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use std::env;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct TriggerAction {
    pub id: String, // UUID
    pub name: String,
    pub temp_plan_guid: String,   // 临时计划 A 的 GUID
    pub target_plan_guid: String, // 目标计划 B 的 GUID
    pub pause_seconds: u32,       // 停顿时间（秒）
    pub enabled: bool,            // 是否启用
}

fn get_actions_path(app: &AppHandle) -> PathBuf {
    let mut path = env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    path.push("trigger_actions.json");
    path
}

#[tauri::command]
pub async fn save_trigger_action(app: AppHandle, action: TriggerAction) -> Result<(), String> {
    let mut actions = load_trigger_actions(app.clone()).await?;

    // 查找并更新或添加新动作
    if let Some(index) = actions.iter().position(|a| a.id == action.id) {
        actions[index] = action;
    } else {
        actions.push(action);
    }

    save_trigger_actions(&app, &actions)
}

#[tauri::command]
pub async fn delete_trigger_action(app: AppHandle, action_id: String) -> Result<(), String> {
    let mut actions = load_trigger_actions(app.clone()).await?;
    actions.retain(|a| a.id != action_id);
    save_trigger_actions(&app, &actions)
}

#[tauri::command]
pub async fn load_trigger_actions(app: AppHandle) -> Result<Vec<TriggerAction>, String> {
    let actions_path = get_actions_path(&app);

    if !actions_path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(actions_path).map_err(|e| format!("读取触发动作失败: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("解析触发动作失败: {}", e))
}

fn save_trigger_actions(app: &AppHandle, actions: &[TriggerAction]) -> Result<(), String> {
    let actions_path = get_actions_path(app);

    if let Some(parent) = actions_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json =
        serde_json::to_string_pretty(actions).map_err(|e| format!("序列化触发动作失败: {}", e))?;

    fs::write(actions_path, json).map_err(|e| format!("保存触发动作失败: {}", e))
}
