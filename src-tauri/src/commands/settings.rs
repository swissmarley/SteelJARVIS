use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub anthropic_api_key: String,
    pub openai_api_key: String,
    pub model: String,
    pub voice_enabled: bool,
    pub clap_detection_enabled: bool,
    pub clap_sensitivity: u8,
    pub activation_method: String,
    pub auto_memory: bool,
    pub execution_mode: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            anthropic_api_key: String::new(),
            openai_api_key: String::new(),
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

#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
    Ok(AppSettings::default())
}

#[tauri::command]
pub async fn update_settings(
    settings: AppSettings,
) -> Result<AppSettings, String> {
    Ok(settings)
}