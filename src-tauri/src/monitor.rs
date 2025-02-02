use std::sync::Arc;
use tokio::sync::Mutex;
use serde::Serialize;
use crate::settings::Settings;
use crate::trigger_action::TriggerAction;
use crate::power_plan::set_active_plan;
use log::{info, warn, error};
use tokio::time::{Duration, interval as tokio_interval, Instant};
use sysinfo::System;
use calcmhz;
use serde_json::json;
use tauri::{Manager, WebviewWindow};
use tauri::async_runtime;
use tauri::Emitter;
use crate::notification::send_notification;
use tauri::AppHandle;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;

#[derive(Clone, Serialize)]
pub struct MonitorState {
    pub frequencies: Vec<u64>,
    pub is_refreshing: bool,
    pub indicator_status: String,
    pub last_update_count: u64,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            frequencies: Vec::new(),
            is_refreshing: false,
            indicator_status: "normal".to_string(),
            last_update_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct Monitor {
    state: Arc<Mutex<MonitorState>>,
    settings: Arc<Mutex<Settings>>,
    running: Arc<tokio::sync::RwLock<bool>>,
    window: Option<WebviewWindow>,
    last_alert_time: Arc<Mutex<u64>>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MonitorState::default())),
            settings: Arc::new(Mutex::new(Settings::default())),
            running: Arc::new(tokio::sync::RwLock::new(false)),
            window: None,
            last_alert_time: Arc::new(Mutex::new(0)),
        }
    }

    pub fn set_window(&mut self, window: WebviewWindow) {
        self.window = Some(window.clone());
        
        // 在设置窗口时初始化监控器的设置
        let window_clone = window.clone();
        let settings_clone = self.settings.clone();
        
        tauri::async_runtime::spawn(async move {
            // 从文件加载设置
            let settings_path = window_clone.app_handle().path().app_data_dir().unwrap().join("settings.json");
            if settings_path.exists() {
                if let Ok(content) = fs::read_to_string(&settings_path) {
                    if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                        let mut current_settings = settings_clone.lock().await;
                        *current_settings = settings;
                        info!("已从文件加载设置");
                    }
                }
            }
        });
    }

    pub async fn update_settings(&self, new_settings: Settings) {
        let mut settings = self.settings.lock().await;
        
        // 检查是否有关键设置发生变化
        let refresh_interval_changed = settings.refresh_interval != new_settings.refresh_interval;
        let frequency_mode_changed = settings.frequency_mode != new_settings.frequency_mode;
        let auto_switch_changed = settings.auto_switch_enabled != new_settings.auto_switch_enabled;
        
        // 更新设置
        *settings = new_settings;
        drop(settings);

        // 如果关键设置发生变化，需要特殊处理
        if refresh_interval_changed || frequency_mode_changed || auto_switch_changed {
            // 重置相关状态
            let mut state = self.state.lock().await;
            state.last_update_count = 0;
            if frequency_mode_changed {
                state.frequencies = Vec::new();
            }
            drop(state);
        }
    }

    pub async fn get_state(&self) -> MonitorState {
        self.state.lock().await.clone()
    }

    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            info!("监控器已经在运行中");
            return;
        }
        *running = true;
        info!("启动监控器");
        drop(running);

        let state = self.state.clone();
        let settings = self.settings.clone();
        let running = self.running.clone();
        let window = self.window.clone();
        let last_alert_time = self.last_alert_time.clone();

        // 初始化状态
        {
            let mut state_guard = state.lock().await;
            state_guard.last_update_count = 0;  // 确保计数从0开始
            
            // 检查是否需要启动自动切换检测
            let settings_guard = settings.lock().await;
            if settings_guard.auto_switch_enabled && settings_guard.frequency_mode == "1" {
                info!("自动切换功能已启用，开始计数");
                drop(settings_guard);
            }
        }

        tokio::spawn(async move {
            let mut current_interval = {
                let settings = settings.lock().await;
                settings.refresh_interval
            };
            
            let mut interval = tokio_interval(Duration::from_millis(current_interval));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            
            let mut last_frequencies = Vec::new();
            let mut unchanged_count = 0;

            while *running.read().await {
                interval.tick().await;

                let settings_guard = settings.lock().await;
                if !settings_guard.frequency_detection_enabled {
                    drop(settings_guard);
                    continue;
                }

                // 检查刷新间隔是否改变
                if current_interval != settings_guard.refresh_interval {
                    current_interval = settings_guard.refresh_interval;
                    interval = tokio_interval(Duration::from_millis(current_interval));
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    info!("刷新间隔已更新: {} ms", current_interval);
                }

                let refresh_interval = settings_guard.refresh_interval;
                let frequency_mode = settings_guard.frequency_mode.clone();
                let frequency_threshold = settings_guard.frequency_threshold;
                let trigger_action_enabled = settings_guard.trigger_action_enabled;
                let auto_switch_enabled = settings_guard.auto_switch_enabled;
                let auto_switch_threshold = settings_guard.auto_switch_threshold;
                let alert_debounce_seconds = settings_guard.alert_debounce_seconds;
                drop(settings_guard);

                // 获取 CPU 频率
                let start_time = Instant::now();
                let frequencies = Self::get_frequencies(&frequency_mode).await;
                let elapsed = start_time.elapsed();

                // 如果采样时间超过了间隔时间的一半，记录警告
                if elapsed.as_millis() as u64 > refresh_interval / 2 {
                    warn!("CPU频率采样耗时较长: {}ms", elapsed.as_millis());
                }

                // 处理自动切换逻辑
                if auto_switch_enabled && frequency_mode == "1" {
                    Self::handle_auto_switch(
                        &frequencies,
                        &last_frequencies,
                        auto_switch_threshold,
                        &mut unchanged_count,
                        &window,
                        settings.clone(),
                        state.clone(),
                    ).await;
                }

                // 更新状态
                {
                    let mut state_guard = state.lock().await;
                    state_guard.frequencies = frequencies.clone();
                    state_guard.is_refreshing = true;
                    state_guard.last_update_count = unchanged_count;
                    
                    if let Some(window) = &window {
                        let _ = window.emit("monitor-state-updated", &*state_guard);
                    }
                }

                // 更新 last_frequencies
                if !frequencies.is_empty() {
                    last_frequencies = frequencies.clone();
                }

                // 检查频率阈值和触发动作
                if let Some(window) = &window {
                    Self::check_frequency_threshold(
                        &frequencies,
                        frequency_threshold,
                        trigger_action_enabled,
                        window,
                        last_alert_time.clone(),
                        alert_debounce_seconds,
                    ).await;
                }
            }
            info!("监控器已停止");
        });
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        if *running {
            *running = false;
            
            // 重置状态
            let mut state_guard = self.state.lock().await;
            state_guard.last_update_count = 0;
            state_guard.frequencies = Vec::new();
            state_guard.is_refreshing = false;
            
            // 如果有窗口，通知前端状态已重置
            if let Some(window) = &self.window {
                let _ = window.emit("monitor-state-updated", &*state_guard);
            }
            
            info!("停止监控器");
        }
    }

    async fn get_frequencies(frequency_mode: &str) -> Vec<u64> {
        if frequency_mode == "1" {
            // SysInfo 模式
            let mut sys = System::new();
            sys.refresh_cpu_all();
            sys.cpus().iter().map(|cpu| cpu.frequency()).collect()
        } else {
            // CalcMhz 模式
            match calcmhz::mhz() {
                Ok(freq) => vec![freq as u64],
                Err(_) => Vec::new()
            }
        }
    }

    async fn check_frequency_threshold(
        frequencies: &[u64],
        threshold: f64,
        trigger_action_enabled: bool,
        window: &WebviewWindow,
        last_alert_time: Arc<Mutex<u64>>,
        debounce_seconds: u64,
    ) {
        let mut exceeded_count = 0;
        let mut exceeded_cores = Vec::new();

        for (index, freq) in frequencies.iter().enumerate() {
            let freq_ghz = *freq as f64 / 1000.0;
            if freq_ghz > threshold {
                exceeded_count += 1;
                warn!("CPU核心 {} 频率超限: {:.2} GHz", index, freq_ghz);
                
                exceeded_cores.push(json!({
                    "core": index,
                    "frequency": freq_ghz,
                    "threshold": threshold
                }));
            }
        }

        // 如果有核心超过阈值，检查防抖
        if exceeded_count > 0 {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let mut last_time = last_alert_time.lock().await;
            if current_time - *last_time >= debounce_seconds {
                // 更新上次报警时间
                *last_time = current_time;
                drop(last_time);  // 提前释放锁，避免死锁

                // 发送通知
                let _ = window.emit("threshold-exceeded", json!({
                    "total_cores": frequencies.len(),
                    "exceeded_count": exceeded_count,
                    "exceeded_cores": exceeded_cores,
                    "threshold": threshold
                }));

                // 发送系统通知
                if exceeded_count == frequencies.len() {
                    send_notification(
                        &window.app_handle(),
                        "CPU 频率警告",
                        &format!("所有核心频率均超过 {:.1} GHz", threshold)
                    );
                } else {
                    send_notification(
                        &window.app_handle(),
                        "CPU 频率警告",
                        &format!("{} 个核心频率超过 {:.1} GHz", exceeded_count, threshold)
                    );
                }

                // 如果启用了触发动作，立即执行
                if trigger_action_enabled {
                    if let Ok(actions) = crate::trigger_action::load_trigger_actions(window.app_handle().clone()).await {
                        if let Some(active_action) = actions.iter().find(|a| a.enabled) {
                            info!("发现活动的触发动作，准备执行: {}", active_action.name);
                            Self::execute_trigger_action(active_action, window.app_handle().clone()).await;
                        } else {
                            warn!("未找到已启用的触发动作");
                        }
                    } else {
                        error!("加载触发动作失败");
                    }
                }
            }
        }

        // 更新指示器状态
        let _ = window.emit("indicator-status-changed", if exceeded_count == frequencies.len() {
            "danger"
        } else if exceeded_count > 0 {
            "warning"
        } else {
            "normal"
        });
    }

    async fn handle_auto_switch(
        frequencies: &[u64],
        last_frequencies: &[u64],
        threshold: u64,
        unchanged_count: &mut u64,
        window: &Option<WebviewWindow>,
        settings: Arc<Mutex<Settings>>,
        state: Arc<Mutex<MonitorState>>,
    ) {
        // 只在 SysInfo 模式下进行检查
        let settings_guard = settings.lock().await;
        if settings_guard.frequency_mode != "1" {
            drop(settings_guard);
            return;
        }
        drop(settings_guard);

        // 检查频率是否有变化
        let has_changed = if !frequencies.is_empty() && !last_frequencies.is_empty() 
            && frequencies.len() == last_frequencies.len() {
            // 只要有一个核心的频率不同，就认为有变化
            frequencies.iter().zip(last_frequencies.iter())
                .any(|(current, last)| current != last)
        } else {
            false  // 如果数组为空或长度不同，视为没有变化
        };

        if has_changed {
            // 如果有变化，重置计数
            if *unchanged_count > 0 {
                info!("检测到频率变化，重置计数器");
                *unchanged_count = 0;
                
                // 更新状态并通知前端
                if let Some(window) = window {
                    let mut state_guard = state.lock().await;
                    state_guard.last_update_count = 0;
                    let _ = window.emit("monitor-state-updated", &*state_guard);
                }
            }
        } else {
            // 如果没有变化，增加计数
            *unchanged_count += 1;
            info!("频率未更新，计数: {}/{}", unchanged_count, threshold);
            
            // 更新状态
            if let Some(window) = window {
                let mut state_guard = state.lock().await;
                state_guard.last_update_count = *unchanged_count;
                let _ = window.emit("monitor-state-updated", &*state_guard);
                drop(state_guard);

                // 检查是否需要自动切换
                if *unchanged_count >= threshold {
                    info!("触发自动切换到 CalcMhz 模式");

                    send_notification(
                        &window.app_handle(),
                        "疑似 SysInfo 模式失效",
                        &format!("已自动切换到 CalcMhz 模式")
                    );
                    
                    // 更新设置
                    let mut settings_guard = settings.lock().await;
                    settings_guard.frequency_mode = "2".to_string();
                    drop(settings_guard);
                    
                    // 通知前端
                    let _ = window.emit("mode-switched", json!({
                        "mode": "2",
                        "auto_switch_disabled": false,
                        "unchanged_count": *unchanged_count
                    }));

                    // 重置计数
                    *unchanged_count = 0;
                }
            }
        }
    }

    async fn execute_trigger_action(action: &TriggerAction, app_handle: AppHandle) {
        info!("开始执行触发动作: {}", action.name);
        
        // 发送开始执行的通知
        // send_notification(
        //     &app_handle,
        //     "触发动作开始执行",
        //     &format!("正在执行触发动作: {}", action.name)
        // );
        
        // 切换到临时计划
        if let Err(e) = set_active_plan(&action.temp_plan_guid) {
            error!("切换到临时计划失败: {}", e);
            send_notification(
                &app_handle,
                "触发动作执行失败",
                &format!("切换到临时计划失败: {}", e)
            );
            return;
        }
        
        // 等待指定时间
        tokio::time::sleep(Duration::from_secs(action.pause_seconds as u64)).await;
        
        // 切换到目标计划
        if let Err(e) = set_active_plan(&action.target_plan_guid) {
            error!("切换到目标计划失败: {}", e);
            send_notification(
                &app_handle,
                "触发动作执行失败",
                &format!("切换到目标计划失败: {}", e)
            );
        } else {
            send_notification(
                &app_handle,
                "触发动作执行完成",
                &format!("成功执行触发动作: {}", action.name)
            );
        }
    }

    pub async fn update_frequency_mode(&self, mode: String) {
        // 先获取旧的模式
        let old_mode = {
            let settings = self.settings.lock().await;
            settings.frequency_mode.clone()
        };

        // 如果模式没有变化，直接返回
        if old_mode == mode {
            return;
        }

        info!("切换频率检测模式: {} -> {}", old_mode, mode);

        // 更新设置
        {
            let mut settings = self.settings.lock().await;
            settings.frequency_mode = mode.clone();
        }
        
        // 重置状态，与频率更新时的逻辑保持一致
        if let Some(window) = &self.window {
            let mut state = self.state.lock().await;
            state.frequencies = Vec::new();
            state.last_update_count = 0;
            state.is_refreshing = true;
            let _ = window.emit("monitor-state-updated", &*state);
            drop(state);

            // 使用短延迟重置刷新状态
            let window_clone = window.clone();
            let state_clone = self.state.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let mut state = state_clone.lock().await;
                state.is_refreshing = false;
                let _ = window_clone.emit("monitor-state-updated", &*state);
            });
        }

        // 发送模式切换通知
        if let Some(window) = &self.window {
            let mode_name = if mode == "1" { "SysInfo" } else { "CalcMhz" };
            send_notification(
                &window.app_handle(),
                "模式已切换",
                &format!("已切换到 {} 模式", mode_name)
            );
        }

        info!("模式切换完成: {}", mode);
    }

    pub async fn has_active_trigger_action(&self, window: &WebviewWindow) -> bool {
        if let Ok(actions) = crate::trigger_action::load_trigger_actions(window.app_handle().clone()).await {
            actions.iter().any(|a| a.enabled)
        } else {
            false
        }
    }

    pub async fn update_auto_switch(&self, enabled: bool, threshold: u64) {
        let mut settings = self.settings.lock().await;
        let old_enabled = settings.auto_switch_enabled;
        settings.auto_switch_enabled = enabled;
        settings.auto_switch_threshold = threshold;
        drop(settings);  // 先释放 settings 锁
        
        // 如果自动切换状态发生改变，重置计数器
        if old_enabled != enabled {
            // 重置状态并通知前端，与频率更新时的逻辑保持一致
            if let Some(window) = &self.window {
                let mut state = self.state.lock().await;
                state.last_update_count = 0;
                state.is_refreshing = true;
                let _ = window.emit("monitor-state-updated", &*state);
                drop(state);

                // 使用短延迟重置刷新状态
                let window_clone = window.clone();
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let mut state = state_clone.lock().await;
                    state.is_refreshing = false;
                    let _ = window_clone.emit("monitor-state-updated", &*state);
                });
            }
            
            info!("自动切换状态改变，重置计数器");
        }
    }

    pub async fn refresh_now(&self) {
        let settings = self.settings.lock().await;
        let frequency_mode = settings.frequency_mode.clone();
        drop(settings);

        // 立即获取一次频率
        let frequencies = Self::get_frequencies(&frequency_mode).await;
        
        // 更新状态
        if let Some(window) = &self.window {
            let mut state = self.state.lock().await;
            state.frequencies = frequencies;
            state.is_refreshing = true;
            let _ = window.emit("monitor-state-updated", &*state);
            
            // 使用短延迟重置刷新状态
            let window_clone = window.clone();
            let state_clone = self.state.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let mut state = state_clone.lock().await;
                state.is_refreshing = false;
                let _ = window_clone.emit("monitor-state-updated", &*state);
            });
        }
    }
}

// 创建一个全局监控实例
lazy_static::lazy_static! {
    pub static ref MONITOR: Monitor = Monitor::new();
} 