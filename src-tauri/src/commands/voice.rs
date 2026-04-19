use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

use crate::observability::{EventBus, JarvisEvent};
use crate::voice::{SpeechManager, ClapDetector, ActivationManager, AudioCapture, SpeechRecognizer};

/// Lightweight bridge so the frontend can write to the dev server stderr.
/// Lets us trace UI-side flow without asking the user to open devtools.
#[tauri::command]
pub fn log_debug(tag: String, message: String) {
    eprintln!("[UI:{}] {}", tag, message);
}

#[tauri::command]
pub fn speak(
    text: String,
    app: AppHandle,
    speech: State<'_, Mutex<SpeechManager>>,
    event_bus: State<'_, Arc<Mutex<EventBus>>>,
) -> Result<(), String> {
    let speech = speech.lock().map_err(|e| e.to_string())?;
    let event_bus = event_bus.lock().map_err(|e| e.to_string())?;
    event_bus.emit(JarvisEvent::VoiceStateChanged { state: "speaking".to_string() });
    speech.speak_async(&text, &app)
}

#[tauri::command]
pub fn stop_speaking(
    speech: State<'_, Mutex<SpeechManager>>,
    event_bus: State<'_, Arc<Mutex<EventBus>>>,
) -> Result<(), String> {
    let speech = speech.lock().map_err(|e| e.to_string())?;
    let event_bus = event_bus.lock().map_err(|e| e.to_string())?;
    event_bus.emit(JarvisEvent::VoiceStateChanged { state: "idle".to_string() });
    speech.stop_speaking()
}

#[tauri::command]
pub fn set_voice(
    voice_name: String,
    speech: State<'_, Mutex<SpeechManager>>,
) -> Result<(), String> {
    let mut speech = speech.lock().map_err(|e| e.to_string())?;
    speech.set_voice(&voice_name)
}

#[tauri::command]
pub fn set_speech_rate(
    rate: u32,
    speech: State<'_, Mutex<SpeechManager>>,
) -> Result<(), String> {
    let mut speech = speech.lock().map_err(|e| e.to_string())?;
    speech.set_rate(rate);
    Ok(())
}

#[tauri::command]
pub fn list_voices(
    speech: State<'_, Mutex<SpeechManager>>,
) -> Result<Vec<String>, String> {
    let speech = speech.lock().map_err(|e| e.to_string())?;
    Ok(speech.available_voices().to_vec())
}

#[tauri::command]
pub fn get_voice_config(
    speech: State<'_, Mutex<SpeechManager>>,
) -> Result<serde_json::Value, String> {
    let speech = speech.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "voice": speech.get_voice(),
        "rate": speech.get_rate(),
    }))
}

#[tauri::command]
pub fn set_clap_enabled(
    enabled: bool,
    activation: State<'_, Mutex<ActivationManager>>,
) -> Result<(), String> {
    let mut activation = activation.lock().map_err(|e| e.to_string())?;
    activation.set_clap_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub fn get_activation_config(
    activation: State<'_, Mutex<ActivationManager>>,
) -> Result<serde_json::Value, String> {
    let activation = activation.lock().map_err(|e| e.to_string())?;
    let config = crate::voice::activation::ActivationConfig::from(&*activation);
    Ok(serde_json::to_value(config).unwrap_or_default())
}

#[tauri::command]
pub fn set_clap_sensitivity(
    sensitivity: u8,
    clap_detector: State<'_, Arc<Mutex<ClapDetector>>>,
) -> Result<(), String> {
    let mut detector = clap_detector.lock().map_err(|e| e.to_string())?;
    detector.set_sensitivity(sensitivity);
    Ok(())
}

#[tauri::command]
pub fn start_clap_detection(
    audio_capture: State<'_, Mutex<AudioCapture>>,
) -> Result<(), String> {
    let capture = audio_capture.lock().map_err(|e| e.to_string())?;
    capture.start()
}

#[tauri::command]
pub fn stop_clap_detection(
    audio_capture: State<'_, Mutex<AudioCapture>>,
) -> Result<(), String> {
    let capture = audio_capture.lock().map_err(|e| e.to_string())?;
    capture.stop()
}

#[tauri::command]
pub fn start_listening(
    stt: State<'_, Mutex<SpeechRecognizer>>,
) -> Result<(), String> {
    // CPAL (clap detection) and AVAudioEngine (STT) can share the default
    // input device on macOS — both just tap the node concurrently. Keeping
    // CPAL running means clap detection stays alive during speech capture.
    eprintln!("[Voice] start_listening invoked");
    let mut stt = stt.lock().map_err(|e| e.to_string())?;
    stt.start()
}

#[tauri::command]
pub fn stop_listening(
    stt: State<'_, Mutex<SpeechRecognizer>>,
) -> Result<(), String> {
    eprintln!("[Voice] stop_listening invoked");
    let mut stt = stt.lock().map_err(|e| e.to_string())?;
    stt.stop()
}