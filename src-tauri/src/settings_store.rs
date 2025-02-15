use std::sync::Mutex;
use crate::settings::Settings;
use std::path::{Path, PathBuf};
use std::env;
use std::fs;
use serde_json;
use log::{error, info};
use once_cell::sync::{Lazy, OnceCell};
use tauri::{AppHandle, WebviewWindow, Manager,Emitter};
use crate::notification::send_notification;
use serde_json::json;
use crate::trigger_action;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;

// 定义全局变量
static SETTINGS_STORE: OnceCell<SettingsStore> = OnceCell::new();
static SETTINGS_MUTEX: Lazy<Mutex<Settings>> = Lazy::new(|| Mutex::new(Settings::default()));

// 添加一个静态变量来控制频率模式是否持久化
static SKIP_FREQUENCY_MODE_PERSIST: AtomicBool = AtomicBool::new(false);

// 定义钩子函数类型
type HookFunction = Box<dyn Fn(&str, &serde_json::Value) + Send + 'static>;

// 存储钩子的全局变量
static SETTING_HOOKS: Lazy<Mutex<HashMap<String, Vec<HookFunction>>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct SettingsStore {
    settings_path: PathBuf,
    app: AppHandle,
}

impl SettingsStore {
    fn new(app: AppHandle) -> Self {
        let settings_path = Self::get_settings_path();
        
        // 初始化时加载设置到全局 Mutex
        if let Ok(settings) = Self::load_from_file(&settings_path) {
            if let Ok(mut global_settings) = SETTINGS_MUTEX.lock() {
                info!("初始化设置成功：{:?}", settings.clone());
                *global_settings = settings;
                
            }
        }
        // 注意：如果加载失败，SETTINGS_MUTEX 已经在创建时使用了 Settings::default()
        
        Self {
            settings_path,
            app,
        }
    }

    // 基础文件操作方法
    fn get_settings_path() -> PathBuf {
        let mut path = env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        path.push("settings.json");
        path
    }

    fn load_from_file(path: &PathBuf) -> Result<Settings, String> {
        if !path.exists() {
            return Ok(Settings::default());
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("读取设置文件失败: {}", e))?;
        
        // 先解析成 Value，这样我们可以检查和修复缺失的字段
        let mut settings_value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("解析设置失败: {}", e))?;
        
        // 获取默认设置
        let default_settings = Settings::default();
        
        // 检查并添加缺失的字段
        if let Some(obj) = settings_value.as_object_mut() {
            // 检查所有可能缺失的字段
            let fields = [
                ("auto_start", json!(default_settings.auto_start)),
                ("auto_minimize", json!(default_settings.auto_minimize)),
                ("refresh_interval", json!(default_settings.refresh_interval)),
                ("frequency_threshold", json!(default_settings.frequency_threshold)),
                ("frequency_mode", json!(default_settings.frequency_mode)),
                ("auto_switch_enabled", json!(default_settings.auto_switch_enabled)),
                ("auto_switch_threshold", json!(default_settings.auto_switch_threshold)),
                ("trigger_action_enabled", json!(default_settings.trigger_action_enabled)),
                ("frequency_detection_enabled", json!(default_settings.frequency_detection_enabled)),
                ("alert_debounce_seconds", json!(default_settings.alert_debounce_seconds)),
            ];

            for (key, default_value) in fields.iter() {
                if !obj.contains_key(*key) {
                    obj.insert(key.to_string(), default_value.clone());
                }
            }
        }
        
        // 将修复后的值转换为 Settings
        let settings: Settings = serde_json::from_value(settings_value)
            .map_err(|e| format!("转换设置失败: {}", e))?;
        
        Ok(settings)
    }

    fn save_to_file(&self, settings: &Settings) -> Result<(), String> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建设置目录失败: {}", e))?;
        }

        // 如果需要跳过频率模式持久化，创建一个新的设置对象
        let settings_to_save = if SKIP_FREQUENCY_MODE_PERSIST.load(Ordering::SeqCst) {
            // 从文件中读取原来的频率模式
            let old_settings = Self::load_from_file(&self.settings_path)?;
            Settings {
                frequency_mode: old_settings.frequency_mode, // 保持原来的频率模式
                ..settings.clone() // 使用其他新的设置
            }
        } else {
            settings.clone()
        };

        let json = serde_json::to_string_pretty(&settings_to_save)
            .map_err(|e| format!("序列化设置失败: {}", e))?;
        
        fs::write(&self.settings_path, json)
            .map_err(|e| format!("保存设置失败: {}", e))?;
        
        Ok(())
    }

    fn notify_settings_changed(&self, settings: &Settings) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.emit("settings-changed", settings);
        }
    }

    pub fn set_trigger_action_master_switch(&self, enabled: bool) -> Result<(), String> {
        let mut settings = SETTINGS_MUTEX.lock()
            .map_err(|_| "获取设置锁失败".to_string())?;
        
        settings.trigger_action_enabled = enabled;
        self.save_to_file(&settings)?;
        
        // 通知前端
        self.notify_settings_changed(&settings);
        
        Ok(())
    }

    pub fn update_settings(&mut self, new_settings: Settings) -> Result<(), String> {
        let mut settings = SETTINGS_MUTEX.lock()
            .map_err(|_| "获取设置锁失败".to_string())?;
        *settings = new_settings;
        self.save_to_file(&settings)?;
        
        // 通知前端
        self.notify_settings_changed(&settings);
        
        Ok(())
    }

    // 其他便捷方法...
    // pub fn set_frequency_mode(&self, mode: String) -> Result<(), String> {
    //     let mut settings = SETTINGS_MUTEX.lock()
    //         .map_err(|_| "获取设置锁失败".to_string())?;
    //     settings.frequency_mode = mode;
    //     self.save_to_file(&settings)
    // }

    // pub fn set_auto_switch(&self, enabled: bool, threshold: u64) -> Result<(), String> {
    //     let mut settings = SETTINGS_MUTEX.lock()
    //         .map_err(|_| "获取设置锁失败".to_string())?;
    //     settings.auto_switch_enabled = enabled;
    //     settings.auto_switch_threshold = threshold;
    //     self.save_to_file(&settings)
    // }
    
    // pub fn get_frequency_mode(&self) -> Result<String, String> {
    //     let settings = SETTINGS_MUTEX.lock()
    //         .map_err(|_| "获取设置锁失败".to_string())?;
    //     Ok(settings.frequency_mode.clone())
    // }

    pub fn get_trigger_action_master_switch(&self) -> Result<bool, String> {
        let settings = SETTINGS_MUTEX.lock()
            .map_err(|_| "获取设置锁失败".to_string())?;
        Ok(settings.trigger_action_enabled)
    }

    // 将 has_valid_actions 改为异步方法
    async fn has_valid_actions(&self) -> Result<bool, String> {
        let actions = crate::trigger_action::load_trigger_actions(self.app.clone()).await?;
        Ok(actions.iter().any(|action| action.enabled))
    }

    // validate_and_update_setting 也需要改为异步
    // pub async fn validate_and_update_setting(&self, key: &str, value: serde_json::Value) -> Result<Settings, String> {
    //     // 先检查是否是 trigger_action_enabled 并需要验证
    //     let needs_validation = if key == "trigger_action_enabled" {
    //         value.as_bool().ok_or("无效的值类型")? // 提前获取 enabled 值
    //     } else {
    //         false
    //     };

    //     // 如果需要验证，在获取锁之前进行
    //     if needs_validation {
    //         if !self.has_valid_actions().await? {
    //             return Err("没有可用的触发动作".to_string());
    //         }
    //     }

    //     // 获取锁并更新设置
    //     let mut settings = SETTINGS_MUTEX.lock()
    //         .map_err(|_| "获取设置锁失败".to_string())?;
        
    //     // 更新设置值
    //     match key {
    //         "trigger_action_enabled" => {
    //             settings.trigger_action_enabled = needs_validation; // 使用之前验证的值
    //         },
    //         "frequency_threshold" => {
    //             let threshold = value.as_f64().ok_or("无效的值类型")?;
    //             if !(0.5..=10.0).contains(&threshold) {
    //                 return Err("频率阈值必须在 0.5-10.0 之间".to_string());
    //             }
    //             settings.frequency_threshold = threshold;
    //         },
    //         "frequency_mode" => {
    //             let mode = value.as_str().ok_or("无效的值类型")?;
    //             if !["1", "2", "3"].contains(&mode) {
    //                 return Err("无效的频率模式".to_string());
    //             }
    //             settings.frequency_mode = mode.to_string();
    //         },
    //         "auto_switch_enabled" => {
    //             settings.auto_switch_enabled = value.as_bool().ok_or("无效的值类型")?;
    //         },
    //         "auto_switch_threshold" => {
    //             let threshold = value.as_u64().ok_or("无效的值类型")?;
    //             if !(1..=100).contains(&threshold) {
    //                 return Err("自动切换阈值必须在 1-100 之间".to_string());
    //             }
    //             settings.auto_switch_threshold = threshold;
    //         },
    //         "refresh_interval" => {
    //             let interval = value.as_u64().ok_or("无效的值类型")?;
    //             if !(100..=10000).contains(&interval) {
    //                 return Err("刷新间隔必须在 100-10000 毫秒之间".to_string());
    //             }
    //             settings.refresh_interval = interval;
    //         },
    //         "frequency_detection_enabled" => {
    //             settings.frequency_detection_enabled = value.as_bool().ok_or("无效的值类型")?;
    //         },
    //         "alert_debounce_seconds" => {
    //             let seconds = value.as_u64().ok_or("无效的值类型")?;
    //             if !(1..=3600).contains(&seconds) {
    //                 return Err("提醒防抖时间必须在 1-3600 秒之间".to_string());
    //             }
    //             settings.alert_debounce_seconds = seconds;
    //         },
    //         "auto_start" => {
    //             settings.auto_start = value.as_bool().ok_or("无效的值类型")?;
    //         },
    //         "auto_minimize" => {
    //             settings.auto_minimize = value.as_bool().ok_or("无效的值类型")?;
    //         },
    //         _ => return Err("未知的设置项".to_string())
    //     }

    //     // 保存设置
    //     self.save_to_file(&settings)?;
        
    //     // 通知前端
    //     self.notify_settings_changed(&settings);
        
    //     // 返回更新后的设置的克隆
    //     Ok(settings.clone())
    // }

    // pub fn update_settings_transaction<F>(&mut self, update_fn: F) -> Result<(), String>
    // where
    //     F: FnOnce(&mut Settings) -> Result<(), String>
    // {
    //     let mut settings = SETTINGS_MUTEX.lock()
    //         .map_err(|_| "获取设置锁失败".to_string())?;
        
    //     // 保存旧设置用于回滚
    //     let old_settings = settings.clone();
        
    //     // 执行更新
    //     if let Err(e) = update_fn(&mut settings) {
    //         // 更新失败，回滚
    //         *settings = old_settings;
    //         return Err(e);
    //     }
        
    //     // 保存到文件
    //     if let Err(e) = self.save_to_file(&settings) {
    //         // 保存失败，回滚
    //         *settings = old_settings;
    //         return Err(e);
    //     }
        
    //     // 通知前端
    //     self.notify_settings_changed(&settings);
        
    //     Ok(())
    // }

    // 添加一个通用的 setter 方法
    pub fn set_setting<T: serde::Serialize>(&self, key: &str, value: T) -> Result<(), String> {
        let mut settings = SETTINGS_MUTEX.lock()
            .map_err(|_| "获取设置锁失败".to_string())?;
        
        // 将值序列化为 JSON Value
        let value = serde_json::to_value(value)
            .map_err(|_| "序列化值失败".to_string())?;
        
        // 根据字段名更新对应的值
        match key {
            "auto_start" => {
                settings.auto_start = value.as_bool()
                    .ok_or("无效的值类型")?;
            },
            "auto_minimize" => {
                settings.auto_minimize = value.as_bool()
                    .ok_or("无效的值类型")?;
            },
            "refresh_interval" => {
                settings.refresh_interval = value.as_u64()
                    .ok_or("无效的值类型")?;
            },
            "frequency_threshold" => {
                settings.frequency_threshold = value.as_f64()
                    .ok_or("无效的值类型")?;
            },
            "frequency_mode" => {
                settings.frequency_mode = value.as_str()
                    .ok_or("无效的值类型")?
                    .to_string();
            },
            "auto_switch_enabled" => {
                settings.auto_switch_enabled = value.as_bool()
                    .ok_or("无效的值类型")?;
            },
            "auto_switch_threshold" => {

                //检查是否小于5，如果小于5，则返回错误
                if value.as_u64().unwrap_or(0) < 5 {
                    return Err("自动切换阈值必须大于5".to_string());
                }

                settings.auto_switch_threshold = value.as_u64()
                    .ok_or("无效的值类型")?;
            },
            "trigger_action_enabled" => {
                //模拟当切换为true时出故障

                // if value.as_bool().unwrap_or(false) {
                //     return Err("模拟出故障".to_string());
                // }

                settings.trigger_action_enabled = value.as_bool()
                    .ok_or("无效的值类型")?;
            },
            "frequency_detection_enabled" => {
                settings.frequency_detection_enabled = value.as_bool()
                    .ok_or("无效的值类型")?;
            },
            "alert_debounce_seconds" => {
                //必须大于0
                if value.as_u64().unwrap_or(0) <= 0 {
                    return Err("提醒防抖时间必须大于0".to_string());
                }

                settings.alert_debounce_seconds = value.as_u64()
                    .ok_or("无效的值类型")?;
            },
            _ => return Err(format!("未知的设置项: {}", key))
        }

        // 保存并通知
        self.save_to_file(&settings)?;
        self.notify_settings_changed(&settings);
        
        Ok(())
    }

    // pub fn set_trigger_action_enabled(&self, enabled: bool) -> Result<(), String> {
    //     self.set_setting("trigger_action_enabled", enabled)
    // }

    // pub fn set_frequency_mode(&self, mode: String) -> Result<(), String> {
    //     self.set_setting("frequency_mode", mode)
    // }

    // pub fn set_auto_switch(&self, enabled: bool, threshold: u64) -> Result<(), String> {
    //     // 对于需要同时设置多个值的情况，可以这样处理
    //     self.set_setting("auto_switch_enabled", enabled)?;
    //     self.set_setting("auto_switch_threshold", threshold)
    // }

    // validate_and_update_setting 方法可以保持不变，用于需要验证的场景
    pub async fn validate_and_update_setting(&self, key: &str, value: serde_json::Value) -> Result<(), String> {
        // let mut settings = SETTINGS_MUTEX.lock()
        //     .map_err(|_| "获取设置锁失败".to_string())?;

        info!("验证和更新设置: {} = {}", key, value);
        self.set_setting(key, value.clone())?;

        // 在更新前触发钩子
        self.trigger_hooks(key, &value);

        // ... 现有的更新逻辑 ...

        Ok(())
    }

    // 添加一个通用的 getter 方法
    pub fn get_setting(&self, key: &str) -> Result<serde_json::Value, String> {
        let settings = SETTINGS_MUTEX.lock()
            .map_err(|_| "获取设置锁失败".to_string())?;
        
        match key {
            "auto_start" => Ok(serde_json::Value::Bool(settings.auto_start)),
            "auto_minimize" => Ok(serde_json::Value::Bool(settings.auto_minimize)),
            "refresh_interval" => Ok(serde_json::Value::Number(settings.refresh_interval.into())),
            "frequency_threshold" => Ok(serde_json::Value::Number(serde_json::Number::from_f64(settings.frequency_threshold)
                .ok_or("转换频率阈值失败")?)),
            "frequency_mode" => Ok(serde_json::Value::String(settings.frequency_mode.clone())),
            "auto_switch_enabled" => Ok(serde_json::Value::Bool(settings.auto_switch_enabled)),
            "auto_switch_threshold" => Ok(serde_json::Value::Number(settings.auto_switch_threshold.into())),
            "trigger_action_enabled" => Ok(serde_json::Value::Bool(settings.trigger_action_enabled)),
            "frequency_detection_enabled" => Ok(serde_json::Value::Bool(settings.frequency_detection_enabled)),
            "alert_debounce_seconds" => Ok(serde_json::Value::Number(settings.alert_debounce_seconds.into())),
            _ => Err(format!("未知的设置项: {}", key))
        }
    }

    // 添加控制持久化的方法
    pub fn set_skip_frequency_mode_persist(&self, skip: bool) {
        SKIP_FREQUENCY_MODE_PERSIST.store(skip, Ordering::SeqCst);
    }

    // 添加注册钩子的方法
    pub fn add_setting_hook<F>(&self, key: &str, hook: F) 
    where
        F: Fn(&str, &serde_json::Value) + Send + 'static
    {
        let mut hooks = SETTING_HOOKS.lock().unwrap();
        hooks.entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(hook));
    }

    // 在更新设置时调用钩子
    fn trigger_hooks(&self, key: &str, value: &serde_json::Value) {
        if let Ok(hooks) = SETTING_HOOKS.lock() {
            if let Some(key_hooks) = hooks.get(key) {
                for hook in key_hooks {
                    hook(key, value);
                }
            }
        }
    }
}

// 初始化函数

pub fn init_settings_store(app: AppHandle) -> Result<(), String> {
    let store = SettingsStore::new(app);
    SETTINGS_STORE.set(store)
        .map_err(|_| "设置存储已经初始化".to_string())
}

// 获取存储实例的辅助函数
fn get_store() -> Result<&'static SettingsStore, String> {
    SETTINGS_STORE.get()
        .ok_or("设置存储未初始化".to_string())
}

// 公开的包装函数
#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    let settings = SETTINGS_MUTEX.lock()
        .map_err(|_| "获取设置锁失败".to_string())?;
    Ok(settings.clone())
}


#[tauri::command]
pub fn update_settings(new_settings: Settings) -> Result<(), String> {
    info!("更新设置: {:?}", new_settings);
    let store = get_store()?;
    let mut settings = SETTINGS_MUTEX.lock()
        .map_err(|_| "获取设置锁失败".to_string())?;
    *settings = new_settings;
    store.save_to_file(&settings)
}

#[tauri::command]
pub fn set_trigger_action_master_switch(enabled: bool) -> Result<(), String> {
    let store = get_store()?;
    store.set_trigger_action_master_switch(enabled)
}

// 其他包装函数...

#[tauri::command]
pub fn is_trigger_action_master_switch_enabled() -> Result<bool, String> {
    let store = get_store()?;
    store.get_trigger_action_master_switch()
}

pub fn get_frequency_threshold() -> f64 {
    match get_store().and_then(|store| store.get_setting("frequency_threshold")) {
        Ok(settings) => settings.as_f64().unwrap_or(3.0),
        Err(_) => 3.0,
    }
}

pub fn get_auto_switch_enabled()-> bool{
    match get_store().and_then(|store| store.get_setting("auto_switch_enabled")) {
        Ok(settings) => settings.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

pub fn get_auto_switch_threshold()-> u64{
    match get_store().and_then(|store| store.get_setting("auto_switch_threshold")) {
        Ok(settings) => settings.as_u64().unwrap_or(0),
        Err(_) => 0,
    }
}

pub fn get_refresh_interval()-> u64{
    match get_store().and_then(|store| store.get_setting("refresh_interval")) {
        Ok(settings) => settings.as_u64().unwrap_or(1000),
        Err(_) => 1000,
    }
}

pub fn get_trigger_action_enabled() -> Result<bool, String>{
    is_trigger_action_master_switch_enabled()
}

pub fn get_alert_debounce_seconds()-> u64{
    match get_store().and_then(|store| store.get_setting("alert_debounce_seconds")) {
        Ok(settings) => settings.as_u64().unwrap_or(15),
        Err(_) => 15,
    }
}

pub fn get_frequency_detection_enabled()-> bool{
    match get_store().and_then(|store| store.get_setting("frequency_detection_enabled")) {
        Ok(settings) => settings.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

pub fn get_frequency_mode()-> String{
    match get_store().and_then(|store| store.get_setting("frequency_mode")) {
        Ok(settings) => settings.as_str().unwrap_or("1").to_string(),
        Err(_) => "1".to_string(),
    }
}
// pub fn get_frequency_mode() -> Result<String, String> {
//     let store = get_store()?;
//     store.get_frequency_mode()
// }

#[tauri::command]
pub async fn update_setting(key: String, value: serde_json::Value) -> Result<(), String> {
    info!("Received update_setting command with key: {}, value: {}", key, value);
    let store = get_store()?;
    store.validate_and_update_setting(&key, value).await
}

#[tauri::command]
pub fn get_setting(key: String) -> Result<serde_json::Value, String> {
    let store = get_store()?;
    store.get_setting(&key)
}

// 添加公开的命令来控制持久化
#[tauri::command]
pub fn set_skip_frequency_mode_persist(skip: bool) -> Result<(), String> {
    let store = get_store()?;
    store.set_skip_frequency_mode_persist(skip);
    Ok(())
}

// 在现有的公开函数部分添加
pub fn add_setting_hook<F>(key: &str, hook: F) -> Result<(), String>
where
    F: Fn(&str, &serde_json::Value) + Send + 'static
{
    let store = get_store()?;
    store.add_setting_hook(key, hook);
    Ok(())
}