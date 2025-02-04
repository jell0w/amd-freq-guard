use serde::{Deserialize, Serialize};
use semver::Version;
use reqwest;
use log::{info, error};
use crate::constants::AppConstants;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    prerelease: bool,
}

#[derive(Serialize)]
pub struct UpdateInfo {
    pub is_success: bool,
    pub message: String,
    pub has_update: bool,
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub changelog: Option<String>,
}

#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> UpdateInfo {
    let current_version = app.package_info().version.to_string();
    
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", AppConstants::GITHUB_API_REPO);
    
    let client = match reqwest::Client::new()
        .get(api_url)
        .header("User-Agent", "AMDFreqGuard")
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                error!("检查更新失败: {}", e);
                return UpdateInfo {
                    is_success: false,
                    message: format!("网络请求失败: {}", e),
                    has_update: false,
                    latest_version: None,
                    download_url: None,
                    changelog: None,
                };
            }
        };

    let release = match client.json::<GithubRelease>().await {
        Ok(release) => release,
        Err(e) => {
            error!("解析响应失败: {}", e);
            return UpdateInfo {
                is_success: false,
                message: format!("解析响应失败: {}", e),
                has_update: false,
                latest_version: None,
                download_url: None,
                changelog: None,
            };
        }
    };
    
    // 解析版本号（移除 v 前缀如果存在）
    let remote_version = release.tag_name.trim_start_matches('v');
    
    match (Version::parse(&current_version), Version::parse(remote_version)) {
        (Ok(current), Ok(remote)) => {
            if remote > current {
                info!("发现新版本: {}", remote_version);
                UpdateInfo {
                    is_success: true,
                    message: "检查更新成功".to_string(),
                    has_update: true,
                    latest_version: Some(remote_version.to_string()),
                    download_url: Some(release.html_url),
                    changelog: release.body,
                }
            } else {
                info!("当前已是最新版本");
                UpdateInfo {
                    is_success: true,
                    message: "当前已是最新版本".to_string(),
                    has_update: false,
                    latest_version: Some(remote_version.to_string()),
                    download_url: None,
                    changelog: None,
                }
            }
        },
        _ => {
            error!("版本号解析失败");
            UpdateInfo {
                is_success: false,
                message: "版本号解析失败".to_string(),
                has_update: false,
                latest_version: None,
                download_url: None,
                changelog: None,
            }
        }
    }
} 