pub struct AppConstants;

impl AppConstants {
    pub const GITHUB_REPO_URL: &'static str = "https://github.com/jell0w/amd-freq-guard";
    pub const GITHUB_API_REPO: &'static str = "jell0w/amd-freq-guard";
    pub const CURRENT_TERMS_OF_SERVICE_VERSION: u32 = 1;
}

#[tauri::command]
pub fn get_constants() -> serde_json::Value {
    serde_json::json!({
        "GITHUB_REPO_URL": AppConstants::GITHUB_REPO_URL,
        "GITHUB_API_REPO": AppConstants::GITHUB_API_REPO,
        "CURRENT_TERMS_OF_SERVICE_VERSION": AppConstants::CURRENT_TERMS_OF_SERVICE_VERSION,
    })
} 