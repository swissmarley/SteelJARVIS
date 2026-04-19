use std::sync::{Arc, Mutex};
use tauri::State;

use crate::agent::AgentEngine;
use crate::memory::MemoryStore;
use crate::observability::EventBus;

#[tauri::command]
pub async fn send_message(
    message: String,
    engine: State<'_, Mutex<AgentEngine>>,
    event_bus: State<'_, Arc<Mutex<EventBus>>>,
) -> Result<String, String> {
    let (api_key, history) = {
        let e = engine.lock().map_err(|e| e.to_string())?;
        (e.api_key().to_string(), e.history().to_vec())
    };

    let bus = event_bus.lock().map_err(|e| e.to_string())?.clone();

    let (response, new_messages) = AgentEngine::send_with(&api_key, &history, &message, &bus).await?;

    // Update engine history with the full conversation
    {
        let mut e = engine.lock().map_err(|e| e.to_string())?;
        e.set_history(new_messages);
    }

    Ok(response)
}

#[tauri::command]
pub async fn check_health(
    engine: State<'_, Mutex<AgentEngine>>,
    mem_store: State<'_, Mutex<MemoryStore>>,
) -> Result<serde_json::Value, String> {
    let provider_connected = {
        let e = engine.lock().map_err(|e| e.to_string())?;
        !e.api_key().is_empty()
    };

    let db_connected = {
        let store = mem_store.lock().map_err(|e| e.to_string())?;
        store.health_check()
    };

    let audio_available = std::process::Command::new("say")
        .arg("-v")
        .arg("?")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    Ok(serde_json::json!({
        "providerConnected": provider_connected,
        "dbConnected": db_connected,
        "audioAvailable": audio_available,
    }))
}