// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use calcmhz;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use std::{fs, path::PathBuf, process::Command, sync::Mutex};
use sysinfo::{CpuRefreshKind, System};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};
use std::fmt;

// 在文件顶部添加模块声明
mod power_plan;
use power_plan::{get_power_plans, set_active_plan, PowerPlan};

mod trigger_action;
pub use trigger_action::{
    save_trigger_action,
    delete_trigger_action,
    load_trigger_actions,
};

// 创建一个全局状态来存储System实例
struct SystemState(Mutex<System>);

// 修改频率获取模式枚举
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FrequencyMode {
    #[serde(rename = "1")]
    SysInfo = 1,
    #[serde(rename = "2")]
    CalcMhz = 2,
}

// 为 FrequencyMode 实现 Display trait
impl fmt::Display for FrequencyMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrequencyMode::SysInfo => write!(f, "1"),
            FrequencyMode::CalcMhz => write!(f, "2"),
        }
    }
}

impl Default for FrequencyMode {
    fn default() -> Self {
        FrequencyMode::SysInfo
    }
}

// 修改设置结构体
#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    auto_start: bool,
    auto_minimize: bool,
    refresh_interval: u64,
    frequency_threshold: f64,
    frequency_mode: String,
    auto_switch_enabled: bool,
    auto_switch_threshold: u64,
    trigger_action_enabled: bool,
    frequency_detection_enabled: bool,  // 添加频率检测开关
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_start: false,
            auto_minimize: false,
            refresh_interval: 1000,
            frequency_threshold: 3.5,
            frequency_mode: FrequencyMode::default().to_string(),
            auto_switch_enabled: false,
            auto_switch_threshold: 25,
            trigger_action_enabled: false,
            frequency_detection_enabled: true,  // 默认开启
        }
    }
}

fn get_settings_path(app: &tauri::AppHandle) -> PathBuf {
    let mut path = app.path().app_data_dir().unwrap();
    path.push("settings.json");
    path
}

// #[tauri::command]
// fn greet(name: &str) -> String {
//     format!("Hello, {}! You've been greeted from Rust!", name)
// }

#[tauri::command]
async fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let settings_path = get_settings_path(&app);

    // 确保目录存在
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // 将设置序列化为JSON并保存到文件
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(settings_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn load_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let settings_path = get_settings_path(&app);

    // 如果文件不存在，返回默认设置
    if !settings_path.exists() {
        return Ok(Settings::default());
    }

    // 读取并解析设置文件
    let content = fs::read_to_string(settings_path).map_err(|e| e.to_string())?;
    
    // 先解析为 serde_json::Value，这样我们可以处理缺失的字段
    let stored_settings: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("解析设置失败: {}", e))?;
    
    // 获取默认设置
    let default_settings = Settings::default();
    
    // 构建完整的设置，使用存储的值或默认值
    let settings = Settings {
        auto_start: stored_settings.get("auto_start")
            .and_then(|v| v.as_bool())
            .unwrap_or(default_settings.auto_start),
            
        auto_minimize: stored_settings.get("auto_minimize")
            .and_then(|v| v.as_bool())
            .unwrap_or(default_settings.auto_minimize),
            
        refresh_interval: stored_settings.get("refresh_interval")
            .and_then(|v| v.as_u64())
            .unwrap_or(default_settings.refresh_interval),
            
        frequency_threshold: stored_settings.get("frequency_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(default_settings.frequency_threshold),
            
        frequency_mode: stored_settings.get("frequency_mode")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_settings.frequency_mode)
            .to_string(),
            
        auto_switch_enabled: stored_settings.get("auto_switch_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(default_settings.auto_switch_enabled),
            
        auto_switch_threshold: stored_settings.get("auto_switch_threshold")
            .and_then(|v| v.as_u64())
            .unwrap_or(default_settings.auto_switch_threshold),
            
        trigger_action_enabled: stored_settings.get("trigger_action_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(default_settings.trigger_action_enabled),
            
        frequency_detection_enabled: stored_settings.get("frequency_detection_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(default_settings.frequency_detection_enabled),
    };

    Ok(settings)
}

// 修改 calcmhz 的频率获取函数
#[tauri::command]
async fn get_cpu_frequency_calcmhz() -> Result<Vec<u64>, String> {
    // 使用 tokio 的 spawn_blocking
    match tauri::async_runtime::spawn_blocking(|| calcmhz::mhz().map(|freq| vec![freq as u64]))
        .await
    {
        Ok(result) => result.map_err(|e| format!("获取CPU频率失败: {}", e)),
        Err(_) => Err("获取频率失败".to_string()),
    }
}

// 修改原有的频率获取函数名称以区分
#[tauri::command]
fn get_cpu_frequency_sysinfo(state: State<SystemState>) -> Vec<u64> {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.cpus().iter().map(|cpu| cpu.frequency()).collect()
}

#[tauri::command]
fn trigger_clock_exceed(core_id: usize, frequency: f64) {
    println!("CPU核心 {} 超频警告: {:.2} GHz", core_id, frequency);
}

#[tauri::command]
fn get_power_plans_command() -> Result<Vec<PowerPlan>, String> {
    get_power_plans()
}

#[tauri::command]
fn set_active_plan_command(guid: String) -> Result<(), String> {
    set_active_plan(&guid)
}

// 重新导出命令
pub use power_plan::{
    delete_power_plan_command, duplicate_power_plan_command, export_power_plan_command,
    import_power_plan_command, rename_power_plan_command,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let system = SystemState(Mutex::new(System::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 获取主窗口
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();

            // 处理窗口关闭事件
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // 阻止窗口关闭
                    api.prevent_close();
                    // 隐藏窗口
                    window_clone.hide().unwrap();
                }
            });

            let show_i = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let win = tray
                            .app_handle()
                            .get_webview_window("main")
                            .expect("failed to get window");
                        if let Ok(visible) = win.is_visible() {
                            if !visible {
                                let _ = win.show();
                            }
                        }
                        let _ = win.set_focus();
                    }
                    _ => {}
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let win = app.get_webview_window("main").unwrap();
                        if let Ok(visible) = win.is_visible() {
                            if !visible {
                                let _ = win.show();
                            }
                        }
                        let _ = win.set_focus();
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .manage(system)
        .invoke_handler(tauri::generate_handler![
            get_cpu_frequency_sysinfo,
            get_cpu_frequency_calcmhz,
            save_settings,
            load_settings,
            trigger_clock_exceed,
            get_power_plans_command,
            set_active_plan_command,
            duplicate_power_plan_command,
            delete_power_plan_command,
            rename_power_plan_command,
            export_power_plan_command,
            import_power_plan_command,
            save_trigger_action,
            delete_trigger_action,
            load_trigger_actions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
