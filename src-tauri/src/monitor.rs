use crate::notification::{send_notification, send_notification_with_handle};
use crate::power_plan::set_active_plan;
use crate::settings::Settings;
use crate::trigger_action::{load_trigger_actions, TriggerAction};
use calcmhz;
use log::{error, info, warn};
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tauri::async_runtime;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::{Manager, WebviewWindow};
use tokio::sync::Mutex;
use tokio::time::{interval as tokio_interval, Duration, Instant};
use crate::settings_store;
use tokio::task::JoinHandle;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::trigger_action;
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
    mode_auto_switched: Arc<Mutex<bool>>,
    monitor_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    timer_version: Arc<AtomicU64>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MonitorState::default())),
            settings: Arc::new(Mutex::new(Settings::default())),
            running: Arc::new(tokio::sync::RwLock::new(false)),
            window: None,
            last_alert_time: Arc::new(Mutex::new(0)),
            mode_auto_switched: Arc::new(Mutex::new(false)),
            monitor_task: Arc::new(Mutex::new(None)),
            timer_version: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn set_window(&mut self, window: WebviewWindow) {
        self.window = Some(window.clone());

        // 注册设置钩子
        let monitor = self.clone();
        
        // 监听频率检测开关
        if let Ok(()) = settings_store::add_setting_hook("frequency_detection_enabled", move |_, value| {
            info!("钩子-频率检测开关变化: {}", value);
            let enabled = value.as_bool().unwrap_or(true);
            let monitor = monitor.clone();
            
            // 直接启动，不等待
            if !enabled {
                monitor.timer_version.store(0, Ordering::SeqCst);
            } else {
                tauri::async_runtime::spawn(async move {
                    monitor.start();
                });
            }
        }) {
            info!("已注册频率检测开关钩子");
        }

        // 监听刷新间隔变化
        let monitor = self.clone();
        if let Ok(()) = settings_store::add_setting_hook("refresh_interval", move |_, value| {
            if let Some(interval) = value.as_u64() {
                let monitor = monitor.clone();
                tauri::async_runtime::spawn(async move {
                    info!("钩子-刷新间隔变化: {}", interval);
                    // 直接重启监控
                    monitor.start();
                });
            }
        }) {
            info!("已注册刷新间隔钩子");
        }

        //监听频率模式变化
        let monitor = self.clone();
        if let Ok(()) = settings_store::add_setting_hook("frequency_mode", move |_, value| {
            if let Some(mode_str) = value.as_str() {
                let mode = mode_str.to_string(); // 转换为拥有所有权的字符串
                let monitor = monitor.clone();
                tauri::async_runtime::spawn(async move {
                    info!("钩子-频率模式变化: {}", mode);
                    monitor.start();
                });
            }
        }) {
            info!("已注册频率模式钩子");
        }

        // 在设置窗口时初始化监控器的设置
        let window_clone = window.clone();
        let settings_clone = self.settings.clone();

        tauri::async_runtime::spawn(async move {
            // 从文件加载设置
            let settings_path = window_clone
                .app_handle()
                .path()
                .app_data_dir()
                .unwrap()
                .join("settings.json");
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

    // pub async fn update_settings(&self, new_settings: Settings) {
    //     let mut settings = self.settings.lock().await;
        
    //     // 检查关键设置变化
    //     let refresh_interval_changed = settings.refresh_interval != new_settings.refresh_interval;
    //     let frequency_mode_changed = settings.frequency_mode != new_settings.frequency_mode;
    //     let auto_switch_changed = settings.auto_switch_enabled != new_settings.auto_switch_enabled;
        
    //     // 更新设置
    //     *settings = new_settings;
    //     drop(settings);

    //     // 如果关键设置发生变化，重置相关状态
    //     if refresh_interval_changed || frequency_mode_changed || auto_switch_changed {
    //         let mut state = self.state.lock().await;
    //         state.last_update_count = 0;
    //         if frequency_mode_changed {
    //             state.frequencies = Vec::new();
    //         }
            
    //         // 通知前端状态已更新
    //         if let Some(window) = &self.window {
    //             let _ = window.emit("monitor-state-updated", &*state);
    //         }
    //     }
    // }

    pub async fn get_state(&self) -> MonitorState {
        self.state.lock().await.clone()
    }

    pub async fn get_current_state(&self) -> MonitorState {
        let state = self.state.lock().await;
        state.clone()
    }

    pub fn start(&self) {
        //如果开启状态为关，那也不用启动
        if !settings_store::get_frequency_detection_enabled() {
            info!("频率检测开关为关，不启动监控");
            return;
        }

        // 更新版本号
        let current_version = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.timer_version.store(current_version, Ordering::SeqCst);
        
        let state = self.state.clone();
        let settings = self.settings.clone();
        let window = self.window.clone();
        let last_alert_time = self.last_alert_time.clone();
        let version = self.timer_version.clone();
        let monitor = self.clone();

        // 不使用 await，直接 spawn 新任务
        tauri::async_runtime::spawn(async move {
            let mut interval_timer = tokio_interval(Duration::from_millis(settings_store::get_refresh_interval()));
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            info!("新的监控器启动，版本号: {}，刷新间隔: {}ms", current_version, settings_store::get_refresh_interval());
            
            let current_version = version.load(Ordering::SeqCst);

            let mut last_frequencies: Vec<u64> = Vec::new();
            let mut unchanged_count = 0;

            let mut skip_interval_for_first_time = true;

            while version.load(Ordering::SeqCst) == current_version && settings_store::get_frequency_detection_enabled() {
                if skip_interval_for_first_time {
                    skip_interval_for_first_time = false;
                } else {
                    interval_timer.tick().await;
                }
                if (version.load(Ordering::SeqCst) != current_version) || (settings_store::get_frequency_detection_enabled() == false) {
                    info!("版本不对或者频率检测开关为关，不往下了");
                    break;
                }
                // 获取当前设置
                // let settings_guard = settings.lock().await;
                // let frequency_mode = settings_guard.frequency_mode.clone();
                let frequency_mode = settings_store::get_frequency_mode();
                // let frequency_threshold = settings_guard.frequency_threshold;
                let frequency_threshold = settings_store::get_frequency_threshold();
                let trigger_action_enabled = settings_store::get_trigger_action_enabled().unwrap_or(false);
                // let auto_switch_enabled = settings_guard.auto_switch_enabled;
                let auto_switch_enabled = settings_store::get_auto_switch_enabled();
                // let refresh_interval = settings_guard.refresh_interval;
                let refresh_interval = settings_store::get_refresh_interval();
                // drop(settings_guard);

                // 检查是否需要更新定时器间隔
                // if interval_timer.period() != Duration::from_millis(refresh_interval) {
                //     interval_timer = tokio_interval(Duration::from_millis(refresh_interval));
                //     interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                //     info!("更新刷新间隔为: {}ms", refresh_interval);
                // }

                // 获取频率数据
                let frequencies = Self::get_frequencies(&frequency_mode).await;

                //如果版本不对，那就不用往下了
                if (version.load(Ordering::SeqCst) != current_version) || (settings_store::get_frequency_detection_enabled() == false) {
                    info!("版本不对或者频率检测开关为关，不往下了");
                    break;
                }

                // 只在启用自动切换时进行频率变化检测
                if auto_switch_enabled && frequency_mode == "1" {
                    // 检查频率是否有变化
                    let has_changed = if !frequencies.is_empty() 
                        && !last_frequencies.is_empty() 
                        && frequencies.len() == last_frequencies.len() 
                    {
                        frequencies.iter().zip(last_frequencies.iter()).any(|(a, b)| a != b)
                    } else {
                        false
                    };

                    if has_changed {
                        unchanged_count = 0;
                    } else {
                        unchanged_count += 1;
                        // 获取阈值用于日志
                        let threshold = settings_store::get_auto_switch_threshold();
                        info!("频率未更新，计数: {}/{}", unchanged_count, threshold);
                    }

                    // 更新前端显示的计数
                    let mut state = state.lock().await;
                    state.last_update_count = unchanged_count;
                    if let Some(window) = &window {
                        let _ = window.emit("monitor-state-updated", &*state);
                    }

                    // 检查是否需要切换模式
                    let threshold = settings_store::get_auto_switch_threshold();
                    if unchanged_count >= threshold {
                        monitor.set_mode_auto_switched(true).await;
                        info!("触发自动切换到 CalcMhz 模式");
                        if let Some(window) = &window {
                            send_notification_with_handle(&window.app_handle(), "CPU频率检测模式自动变更", "由于在Sysinfo模式下频率长时间未更新，我们认为这是有问题的，自动切换到 CalcMhz 模式");
                            let _ = window.emit(
                                "mode-switched",
                                json!({
                                    "mode": "2",
                                    "auto_switch_disabled": false,
                                    "unchanged_count": unchanged_count
                                }),
                            );
                        }
                        drop(state);

                        // 更新设置
                        let mut settings = settings.lock().await;
                        settings.frequency_mode = "2".to_string();
                        drop(settings);
                        unchanged_count = 0;
                        continue;
                    }
                } else {
                    // 如果自动切换被禁用，立即重置计数器和状态
                    if unchanged_count > 0 {
                        unchanged_count = 0;
                        let mut state = state.lock().await;
                        state.last_update_count = 0;
                        if let Some(window) = &window {
                            let _ = window.emit("monitor-state-updated", &*state);
                        }
                    }
                }

                // 更新 last_frequencies
                last_frequencies = frequencies.clone();

                // 更新状态
                let mut state = state.lock().await;
                state.frequencies = frequencies.clone();
                
                // 检查是否需要执行触发动作
                if let Some(window) = &window {
                    let settings = settings.lock().await;
                    
                    // 只有在总开关打开时才检查频率并执行动作
                    if trigger_action_enabled {
                        for (i, &freq) in frequencies.iter().enumerate() {
                            let freq_ghz = freq as f64 / 1000.0;
                            if freq_ghz > frequency_threshold {
                                // 检查是否需要发送通知（防抖）
                                let current_time = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                
                                let mut last_alert = last_alert_time.lock().await;
                                if current_time - *last_alert >= settings.alert_debounce_seconds {
                                    *last_alert = current_time;
                                    
                                    // 执行触发动作
                                    if let Ok(actions) = load_trigger_actions(window.app_handle().clone()).await {
                                        if let Some(active_action) = actions.iter().find(|a| a.enabled) {
                                            info!("发现活动的触发动作，准备执行: {}", active_action.name);
                                            trigger_action::execute_trigger_action(
                                                active_action,
                                            ).await;
                                        } else {
                                            warn!("未找到已启用的触发动作");
                                        }
                                    } else {
                                        error!("加载触发动作失败");
                                    }
                                }
                                break; // 找到一个超过阈值的就足够了
                            }
                        }
                    }
                    drop(settings);

                    // 发送状态更新到前端
                    let _ = window.emit("monitor-state-updated", &*state);
                }
                
                // 检查频率阈值和触发动作
                if let Some(window) = &window {
                    Self::check_frequency_threshold(
                        &frequencies,
                        frequency_threshold,
                        trigger_action_enabled,
                        window,
                        last_alert_time.clone(),
                        // settings.lock().await.alert_debounce_seconds,
                        settings_store::get_alert_debounce_seconds()
                    )
                    .await;
                }
                
            }
            
            info!("监控器停止: version={}/{}", 
                version.load(Ordering::SeqCst),
                current_version
            );
        });
    }

    pub async fn stop(&self) {
        // let mut running = self.running.write().await;
        // if *running {
        //     *running = false;

        //     // 重置状态
        //     let mut state_guard = self.state.lock().await;
        //     state_guard.last_update_count = 0;
        //     state_guard.frequencies = Vec::new();
        //     state_guard.is_refreshing = false;

        //     // 如果有窗口，通知前端状态已重置
        //     if let Some(window) = &self.window {
        //         let _ = window.emit("monitor-state-updated", &*state_guard);
        //     }

        //     info!("停止监控器");
        // }

        // 直接修改全局
        self.timer_version.store(0, Ordering::SeqCst);
        info!("停止监控器");
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
                Err(_) => Vec::new(),
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
                drop(last_time); // 提前释放锁，避免死锁

                // 发送通知
                let _ = window.emit(
                    "threshold-exceeded",
                    json!({
                        "total_cores": frequencies.len(),
                        "exceeded_count": exceeded_count,
                        "exceeded_cores": exceeded_cores,
                        "threshold": threshold
                    }),
                );

                // 发送系统通知
                if exceeded_count == frequencies.len() {
                    send_notification("CPU 频率警告", &format!("所有核心频率均超过 {:.1} GHz", threshold));
                } else {
                    send_notification("CPU 频率警告", &format!("{} 个核心频率超过 {:.1} GHz", exceeded_count, threshold));
                }

                // 如果启用了触发动作，立即执行
                if trigger_action_enabled {
                    if let Ok(actions) = trigger_action::load_trigger_actions(window.app_handle().clone()).await {
                        if let Some(active_action) = actions.iter().find(|a| a.enabled) {
                            info!("发现活动的触发动作，准备执行: {}", active_action.name);
                            trigger_action::execute_trigger_action(
                                active_action,
                            ).await;
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
        let _ = window.emit(
            "indicator-status-changed",
            if exceeded_count == frequencies.len() {
                "danger"
            } else if exceeded_count > 0 {
                "warning"
            } else {
                "normal"
            },
        );
    }

    // pub async fn update_frequency_mode(&self, mode: String) {
    //     self.set_mode_auto_switched(false).await;
    //     // 先获取旧的模式
    //     let old_mode = {
    //         let settings = self.settings.lock().await;
    //         settings.frequency_mode.clone()
    //     };

    //     // 如果模式没有变化，直接返回
    //     if old_mode == mode {
    //         return;
    //     }

    //     info!("切换频率检测模式: {} -> {}", old_mode, mode);

    //     // 更新设置
    //     {
    //         let mut settings = self.settings.lock().await;
    //         settings.frequency_mode = mode.clone();
    //     }

    //     // 重置状态，与频率更新时的逻辑保持一致
    //     if let Some(window) = &self.window {
    //         let mut state = self.state.lock().await;
    //         state.frequencies = Vec::new();
    //         state.last_update_count = 0;
    //         state.is_refreshing = true;
    //         let _ = window.emit("monitor-state-updated", &*state);
    //         drop(state);

    //         // 使用短延迟重置刷新状态
    //         let window_clone = window.clone();
    //         let state_clone = self.state.clone();
    //         tokio::spawn(async move {
    //             tokio::time::sleep(Duration::from_millis(200)).await;
    //             let mut state = state_clone.lock().await;
    //             state.is_refreshing = false;
    //             let _ = window_clone.emit("monitor-state-updated", &*state);
    //         });
    //     }

    //     // 发送模式切换通知
    //     if let Some(window) = &self.window {
    //         let mode_name = if mode == "1" { "SysInfo" } else { "CalcMhz" };
    //         send_notification_with_handle(&window.app_handle(), "模式已切换", &format!("已切换到 {} 模式", mode_name));
    //     }

    //     info!("模式切换完成: {}", mode);
    // }

    pub async fn has_active_trigger_action(&self, window: &WebviewWindow) -> bool {
        if let Ok(actions) =
            trigger_action::load_trigger_actions(window.app_handle().clone()).await
        {
            actions.iter().any(|a| a.enabled)
        } else {
            false
        }
    }

    // pub async fn update_auto_switch(&self, enabled: bool, threshold: u64) {
    //     let mut settings = self.settings.lock().await;
    //     settings.auto_switch_enabled = enabled;
        
    //     // 根据启用状态设置阈值
    //     settings.auto_switch_threshold = if enabled {
    //         threshold.max(5) // 确保最小值为 5
    //     } else {
    //         20 // 关闭时设置为默认值 20
    //     };
    //     drop(settings);

    //     // 立即重置计数器
    //     if let Some(window) = &self.window {
    //         let mut state = self.state.lock().await;
    //         state.last_update_count = 0;
    //         let _ = window.emit("monitor-state-updated", &*state);
    //     }
        
    //     info!("自动切换状态更新: enabled={}, threshold={}", enabled, threshold);
    // }

    pub async fn refresh_now(&self) {
        info!("立即刷新频率");
        let frequency_mode = {
            let settings = self.settings.lock().await;
            settings.frequency_mode.clone()
        };

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

    pub async fn set_mode_auto_switched(&self, switched: bool) {
        let mut flag = self.mode_auto_switched.lock().await;
        *flag = switched;
    }

    pub async fn is_mode_auto_switched(&self) -> bool {
        *self.mode_auto_switched.lock().await
    }
}

// 创建一个全局监控实例
lazy_static::lazy_static! {
    pub static ref MONITOR: Monitor = Monitor::new();
}

// 添加一个 tauri 命令
#[tauri::command]
pub async fn get_monitor_state() -> Result<MonitorState, String> {
    Ok(MONITOR.get_current_state().await)
}
