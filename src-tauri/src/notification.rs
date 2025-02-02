use log::error;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn send_notification(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        error!("发送通知失败: {}", e);
    }
}
