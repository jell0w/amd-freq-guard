use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use std::env;
use std::path::Path;
use uuid;
use std::time::Duration;
use log;

use crate::notification::send_notification;
use crate::power_plan::{check_if_scheme_is_valid, set_active_plan};

// 定义不同类型的执行体
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]  // 使用 untagged 让序列化时不包含类型标记
pub enum TriggerActionWorker {
    Simple {
        temp_plan_guid: String,
        pause_seconds: u32,
        target_plan_guid: String,
    },
    SettingSwitch {
        // 未来实现
    },
    Workflow {
        // 未来实现
    }
}

// 修改触发动作结构体
#[derive(Serialize, Deserialize, Clone)]
pub struct TriggerAction {
    pub id: String,
    pub name: String,
    pub version: String,  // "simple", "setting_switch", "workflow"
    pub enabled: bool,
    pub worker: TriggerActionWorker,
}

// 实现默认值
impl Default for TriggerAction {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            version: "simple".to_string(),
            enabled: false,
            worker: TriggerActionWorker::Simple {
                temp_plan_guid: String::new(),
                pause_seconds: 1,
                target_plan_guid: String::new(),
            },
        }
    }
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

// 添加新函数
pub async fn set_trigger_action_enabled(app: &tauri::AppHandle, action_id: &str, enabled: bool) -> Result<(), String> {
    let mut actions = load_trigger_actions(app.clone()).await?;
    
    // 查找并更新指定动作的启用状态
    if let Some(action) = actions.iter_mut().find(|a| a.id == action_id) {
        action.enabled = enabled;
        // 保存更新后的动作列表
        save_trigger_actions(app, &actions)?;
    }
    
    Ok(())
}

// 添加新函数用于获取动作详情
pub async fn get_trigger_action_by_id(app: &tauri::AppHandle, action_id: &str) -> Result<Option<TriggerAction>, String> {
    let actions = load_trigger_actions(app.clone()).await?;
    Ok(actions.into_iter().find(|a| a.id == action_id))
}

//获取所有动作的个数
pub fn get_trigger_action_count(app: &tauri::AppHandle) -> Result<usize, String> {
    // let actions = load_trigger_actions(app.clone()).await?;
    let app = app.clone();
    let handle = std::thread::spawn(move || {
         tauri::async_runtime::block_on(load_trigger_actions(app))
    });
    let actions = handle.join().map_err(|_| "独立线程执行出错".to_string())??;
    Ok(actions.len())
}

//获取所有启用的动作的个数
pub fn get_trigger_action_enabled_count(app: &tauri::AppHandle) -> Result<usize, String> {
    let app = app.clone();
    let handle = std::thread::spawn(move || {
         tauri::async_runtime::block_on(load_trigger_actions(app))
    });
    let actions = handle.join().map_err(|_| "独立线程执行出错".to_string())??;
    Ok(actions.iter().filter(|a| a.enabled).count())
}

pub async fn execute_trigger_action(action: &TriggerAction) {
    log::info!("开始执行触发动作: {}", action.name);

    match action.version.as_str() {
        "simple" => {
            if let TriggerActionWorker::Simple { temp_plan_guid, pause_seconds, target_plan_guid } = &action.worker {
                // 执行简单模式的逻辑
                if let Err(e) = set_active_plan(temp_plan_guid) {
                    log::error!("切换到临时计划失败: {}", e);
                    send_notification("触发动作执行失败", &format!("切换到临时计划失败: {}", e));
                    return;
                }

                tokio::time::sleep(Duration::from_secs(*pause_seconds as u64)).await;

                if let Err(e) = set_active_plan(target_plan_guid) {
                    log::error!("切换到目标计划失败: {}", e);
                    send_notification("触发动作执行失败", &format!("切换到目标计划失败: {}", e));
                } else {
                    send_notification("触发动作执行完成", &format!("成功执行触发动作: {}", action.name));
                }
            }
        },
        "setting_switch" => {
            // 未来实现
        },
        "workflow" => {
            // 未来实现
        },
        _ => log::error!("未知的触发动作类型: {}", action.version)
    }
}

pub async fn is_valid_trigger_action(action: &TriggerAction) -> Result<(), String> {
    match action.version.as_str() {
        "simple" => {
            if let TriggerActionWorker::Simple { temp_plan_guid, target_plan_guid, pause_seconds } = &action.worker {
                //逐个检查并抛出异常
                if !check_if_scheme_is_valid(temp_plan_guid) {
                    log::error!("临时计划不存在: {}", temp_plan_guid);
                    return Err(format!("临时计划不存在: {}", temp_plan_guid));
                }
                if !check_if_scheme_is_valid(target_plan_guid) {
                    log::error!("目标计划不存在: {}", target_plan_guid);
                    return Err(format!("目标计划不存在: {}", target_plan_guid));
                }
                //检查pause_seconds
                if *pause_seconds < 1 {
                    log::error!("暂停时间必须大于0，当前: {}", pause_seconds);
                    return Err(format!("暂停时间必须大于0，当前: {}", pause_seconds));
                }
            }
            Ok(())
        }
        "setting_switch" => {
            // 未来实现
            Ok(())
        }
        "workflow" => {
            // 未来实现
            Ok(())
        }
        _ => Err(format!("未知的触发动作类型: {}", action.version))
    }
}
