use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub auto_start: bool,
    pub auto_minimize: bool,
    pub refresh_interval: u64,
    pub frequency_threshold: f64,
    pub frequency_mode: String,
    pub auto_switch_enabled: bool,
    pub auto_switch_threshold: u64,
    pub trigger_action_enabled: bool,
    pub frequency_detection_enabled: bool,
    pub alert_debounce_seconds: u64,
    pub accepted_terms_of_service: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_start: false,
            auto_minimize: false,
            refresh_interval: 5000,
            frequency_threshold: 2.9,
            frequency_mode: "1".to_string(),
            auto_switch_enabled: false,
            auto_switch_threshold: 15,
            trigger_action_enabled: false,
            frequency_detection_enabled: false,
            alert_debounce_seconds: 10,
            accepted_terms_of_service: 0,
        }
    }
}

