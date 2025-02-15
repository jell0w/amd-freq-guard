// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use calcmhz;
use env_logger;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::thread;
use std::time::Duration;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};
use sysinfo::{CpuRefreshKind, System};
// use tauri::api::shell;
use tauri::async_runtime;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WebviewWindow,
    Emitter
};
use tauri_plugin_shell::ShellExt;
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsWindow, PostMessageW, SetForegroundWindow, ShowWindow, SW_SHOW, WM_CLOSE,
};

// 在文件顶部添加模块声明
mod power_plan;
use power_plan::{
    check_if_scheme_is_valid, get_power_plans, get_power_plans_json_by_scheme_guid,
    set_active_plan, PowerPlan,
};

mod trigger_action;
pub use trigger_action::{delete_trigger_action, load_trigger_actions, save_trigger_action, set_trigger_action_enabled, get_trigger_action_by_id};

mod monitor;
pub use monitor::MONITOR;

mod settings;
mod settings_store;
pub use settings::Settings;
use settings_store::{
    get_settings, 
    update_settings, 
    init_settings_store, 
    set_trigger_action_master_switch,
    update_setting,
    get_setting,
};

mod notification;
use notification::{init_notification_manager, send_notification};

mod autostart;
use autostart::{disable_autostart, enable_autostart, setup_autostart};

mod updater;
use updater::check_update;

mod constants;
use constants::get_constants;

mod PowerPlanUtils;
use PowerPlanUtils::GetPowerPlans::write_value_set;

// 添加新模块
mod power_settings_preferences_store;
use power_settings_preferences_store::{get_liked_power_settings, toggle_power_setting_liked};

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

use std::env;

fn get_settings_path(app: &tauri::AppHandle) -> PathBuf {
    let mut path = env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    path.push("settings.json");
    path
}

// #[tauri::command]
// fn greet(name: &str) -> String {
//     format!("Hello, {}! You've been greeted from Rust!", name)
// }

#[tauri::command]
async fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    update_settings(settings)
}

#[tauri::command]
async fn load_settings(_app: tauri::AppHandle) -> Result<Settings, String> {
    get_settings()
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
fn get_cpu_frequency_sysinfo(_state: State<SystemState>) -> Vec<u64> {
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
fn get_power_plans_json_by_scheme_guid_command(guid: &str) -> Result<String, String> {
    get_power_plans_json_by_scheme_guid(guid)
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

// 添加缺失的命令
// #[tauri::command]
// async fn update_monitor_settings(settings: Settings) -> Result<(), String> {
//     MONITOR.update_settings(settings).await;
//     Ok(())
// }

#[tauri::command]
async fn check_active_trigger_action(window: tauri::WebviewWindow) -> Result<bool, String> {
    Ok(MONITOR.has_active_trigger_action(&window).await)
}

// #[tauri::command]
// async fn update_frequency_mode(mode: String) -> Result<(), String> {
//     MONITOR.update_frequency_mode(mode).await;
//     Ok(())
// }

// #[tauri::command]
// async fn update_auto_switch(enabled: bool, threshold: u64) -> Result<(), String> {
//     MONITOR.update_auto_switch(enabled, threshold).await;
//     Ok(())
// }

#[tauri::command]
async fn refresh_frequencies() -> Result<(), String> {
    MONITOR.refresh_now().await;
    Ok(())
}

#[tauri::command]
fn check_admin_privileges() -> bool {
    unsafe {
        // IsUserAnAdmin() != 0
        let is_admin = IsUserAnAdmin();
        info!("is_under_admin_privileges: {}", is_admin);
        is_admin != 0
    }
}

#[tauri::command]
async fn request_admin_privileges(app: tauri::AppHandle) -> Result<(), String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

    // 添加管理员重启参数
    let params = "--admin-restart\0".encode_utf16().collect::<Vec<u16>>();
    let path = current_exe
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let operation = "runas\0".encode_utf16().collect::<Vec<u16>>();

    unsafe {
        let result = ShellExecuteW(
            0,                  // hwnd
            operation.as_ptr(), // operation ("runas")
            path.as_ptr(),      // file
            params.as_ptr(),    // parameters (添加了重启参数)
            std::ptr::null(),   // directory
            SW_SHOW,            // show command
        );

        if result > 32 {
            app.exit(0);
            Ok(())
        } else {
            Err(format!("启动失败，错误码: {}", result))
        }
    }
}

#[tauri::command]
async fn open_external_link(url: String, app: tauri::AppHandle) -> Result<(), String> {
    let url_wide: Vec<u16> = format!("{}\0", url).encode_utf16().collect();
    let operation = "open\0".encode_utf16().collect::<Vec<u16>>();

    unsafe {
        let result = ShellExecuteW(
            0,                  // hwnd
            operation.as_ptr(), // operation
            url_wide.as_ptr(),  // file
            std::ptr::null(),   // parameters
            std::ptr::null(),   // directory
            SW_SHOW,            // show command
        );

        if result > 32 {
            Ok(())
        } else {
            Err(format!("打开链接失败，错误码: {}", result))
        }
    }
}

#[tauri::command]
async fn toggle_autostart(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("获取程序路径失败: {}", e))?
            .to_string_lossy()
            .to_string();

        setup_autostart(exe_path)?;
        enable_autostart()?;
    } else {
        disable_autostart()?;
    }
    Ok(())
}

#[tauri::command]
async fn write_value_set_command(
    guid: &str,
    subgroup_guid: &str,
    setting_guid: &str,
    ac_value: u32,
    dc_value: u32,
) -> Result<(), String> {
    info!("write_value_set_command: guid: {}, subgroup_guid: {}, setting_guid: {}, ac_value: {}, dc_value: {}", guid, subgroup_guid, setting_guid, ac_value, dc_value);
    write_value_set(guid, subgroup_guid, setting_guid, ac_value, dc_value)
}

#[tauri::command]
async fn refresh_now_command() -> Result<(), String> {
    MONITOR.refresh_now().await;
    Ok(())
}

#[tauri::command]
async fn toggle_trigger_action(app: tauri::AppHandle, action_id: String, enabled: bool) -> Result<(), String> {
    // 如果要启用动作，先检查电源计划是否有效
    if enabled {
        // 获取动作详情
        let action = get_trigger_action_by_id(&app, &action_id)
            .await?
            .ok_or_else(|| "找不到指定的触发动作".to_string())?;
        
        // 检查临时计划和目标计划是否都存在
        if !check_if_scheme_is_valid(&action.temp_plan_guid) {
            return Err("临时电源计划不存在，无法启用此触发动作".to_string());
        }
        
        if !check_if_scheme_is_valid(&action.target_plan_guid) {
            return Err("目标电源计划不存在，无法启用此触发动作".to_string());
        }
    }
    
    // 如果检查通过或是禁用操作，则更新状态
    set_trigger_action_enabled(&app, &action_id, enabled).await
}

// 添加新函数用于检查触发动作的电源计划
async fn check_trigger_actions_power_plans(app_handle: &tauri::AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
    let actions = load_trigger_actions(app_handle.clone()).await?;
    let mut has_invalid_plan = false;
    let mut invalid_plans = Vec::new();
    let mut invalid_action_ids = Vec::new();

    // 只检查已启用的动作
    for action in actions.iter().filter(|a| a.enabled) {
        if !check_if_scheme_is_valid(&action.temp_plan_guid)
            || !check_if_scheme_is_valid(&action.target_plan_guid)
        {
            has_invalid_plan = true;
            invalid_plans.push(action.name.clone());
            invalid_action_ids.push(action.id.clone());
        }
    }

    if has_invalid_plan {
        set_trigger_action_master_switch(false)?;

        // 禁用包含无效计划的动作
        for action_id in invalid_action_ids {
            let _ = set_trigger_action_enabled(app_handle, &action_id, false).await;
        }

        // 发送通知
        let invalid_plans_str = invalid_plans.join(", ");
        let message = format!(
            "以下已启用的触发动作包含不存在的电源计划：{}。已自动关闭触发动作功能。",
            invalid_plans_str
        );
        send_notification( "触发动作已禁用", &message);
        // 发送前端通知
        window.emit("trigger-actions-disabled", message).unwrap_or_default();
    }

    Ok(())
}

#[tauri::command]
async fn set_trigger_action_master_switch_command(enabled: bool) -> Result<(), String> {
    set_trigger_action_master_switch(enabled)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // 检查命令行参数，看是否是管理员重启
    let args: Vec<String> = env::args().collect();
    let is_admin_restart = args.len() > 1 && args[1] == "--admin-restart";

    let system = SystemState(Mutex::new(System::new()));

    let mut builder = tauri::Builder::default();

    let is_autostart = args.iter().any(|arg| arg == "--autostart");

    // 添加单实例插件
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(
            move |app_handle, argv, cwd| {
                info!("检测到新实例启动，参数: {:?}, 工作目录: {:?}", argv, cwd);

                if let Some(window) = app_handle.get_webview_window("main") {
                    // 确保窗口可见
                    let _ = window.unminimize();
                    let _ = window.show();
                    // 设置焦点
                    let _ = window.set_focus();

                    // 如果窗口被最小化了，恢复它
                    if let Ok(true) = window.is_minimized() {
                        let _ = window.unminimize();
                    }

                    info!("已激活现有窗口");
                }
            },
        ));
    }

    builder
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| {
            // 初始化设置存储
            init_settings_store(app.handle().clone())?;
            // 初始化通知管理器
            init_notification_manager(app.handle().clone())?;
            
            // 获取主窗口
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();

            // 设置窗口标题，包含版本号
            let version = app.package_info().version.to_string();
            let title = format!("AMDFreqGuard Version:{}", version);
            window.set_title(&title).unwrap();

            // 读取设置
            let settings_path = get_settings_path(app.handle());
            let auto_minimize = if settings_path.exists() {
                if let Ok(content) = fs::read_to_string(&settings_path) {
                    if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                        settings.auto_minimize
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // 如果不是自动启动或者未开启自动隐藏，则显示窗口
            if !is_autostart || !auto_minimize {
                window.show().unwrap();
            }

            // 设置监控器的窗口并启动监控
            {
                let mut monitor = MONITOR.clone();
                monitor.set_window(window.clone());
                tauri::async_runtime::spawn(async move {
                    monitor.start().await;
                });
            }

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

            // 检查触发动作中的电源计划
            let app_handle = app.handle().clone();
            let window_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = check_trigger_actions_power_plans(&app_handle, &window_clone).await {
                    error!("检查触发动作电源计划失败: {}", e);
                }
            });

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
            // update_monitor_settings,
            check_active_trigger_action,
            // update_frequency_mode,
            // update_auto_switch,
            refresh_frequencies,
            check_admin_privileges,
            request_admin_privileges,
            open_external_link,
            toggle_autostart,
            check_update,
            get_constants,
            get_power_plans_json_by_scheme_guid_command,
            toggle_power_setting_liked,
            get_liked_power_settings,
            write_value_set_command,
            refresh_now_command,
            toggle_trigger_action,
            set_trigger_action_master_switch_command,
            get_settings,
            // update_settings,
            update_setting,
            get_setting,
            set_trigger_action_master_switch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
