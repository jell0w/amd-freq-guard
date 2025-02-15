use log::{error, info};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use once_cell::sync::OnceCell;

// 定义全局单例
static NOTIFICATION_MANAGER: OnceCell<NotificationManager> = OnceCell::new();

pub struct NotificationManager {
    app: AppHandle,
}

impl NotificationManager {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub fn send(&self, title: &str, body: &str) {
        if let Err(e) = self.app.notification().builder()
            .title(title)
            .body(body)
            .show() 
        {
            error!("发送通知失败: {}", e);
        } else {
            info!("发送通知成功: {} - {}", title, body);
        }
    }
}

// 初始化函数
pub fn init_notification_manager(app: AppHandle) -> Result<(), String> {
    let manager = NotificationManager::new(app);
    NOTIFICATION_MANAGER
        .set(manager)
        .map_err(|_| "通知管理器已经初始化".to_string())
}

// 获取管理器实例的辅助函数
fn get_manager() -> Result<&'static NotificationManager, String> {
    NOTIFICATION_MANAGER
        .get()
        .ok_or("通知管理器未初始化".to_string())
}

// 保持原有的公开函数作为便捷方法
pub fn send_notification(title: &str, body: &str) -> Result<(), String> {
    let manager = get_manager()?;
    manager.send(title, body);
    Ok(())
}

// 如果仍然需要支持传入 AppHandle 的旧方法
pub fn send_notification_with_handle(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder()
        .title(title)
        .body(body)
        .show() 
    {
        error!("发送通知失败: {}", e);
    }
}
