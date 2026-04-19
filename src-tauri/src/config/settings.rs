use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub anthropic_api_key: String,
    pub model: String,
    pub voice_enabled: bool,
    pub clap_detection_enabled: bool,
    pub clap_sensitivity: u8,
    pub activation_method: String,
    pub auto_memory: bool,
    pub execution_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            anthropic_api_key: String::new(),
            model: "claude-sonnet-4-6".to_string(),
            voice_enabled: false,
            clap_detection_enabled: false,
            clap_sensitivity: 5,
            activation_method: "hotkey".to_string(),
            auto_memory: true,
            execution_mode: "assistive".to_string(),
        }
    }
}