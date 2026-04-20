use std::sync::{Arc, Mutex};
use tauri::State;

use crate::agent::{AgentContext, AgentEngine};
use crate::memory::{Embedder, MemoryStore};
use crate::observability::EventBus;

#[tauri::command]
pub async fn send_message(
    message: String,
    engine: State<'_, Mutex<AgentEngine>>,
    mem_store: State<'_, Mutex<MemoryStore>>,
    embedder: State<'_, Embedder>,
    event_bus: State<'_, Arc<Mutex<EventBus>>>,
) -> Result<String, String> {
    eprintln!("[Chat] send_message invoked, text={:?}", message);
    let (api_key, history) = {
        let e = engine.lock().map_err(|e| e.to_string())?;
        (e.api_key().to_string(), e.history().to_vec())
    };

    if api_key.is_empty() {
        eprintln!("[Chat] ERROR: ANTHROPIC_API_KEY is not configured");
        return Err("ANTHROPIC_API_KEY is not configured. Add it to .env or the environment and restart.".to_string());
    }
    eprintln!("[Chat] api_key len={}, history len={}", api_key.len(), history.len());

    let bus = event_bus.lock().map_err(|e| e.to_string())?.clone();

    // TODO(Task 10): replace with build_context(...) once memory + session plumbing lands.
    let ctx = AgentContext::default();
    let result = AgentEngine::send_with(
        &api_key,
        &history,
        &message,
        &ctx,
        &*mem_store,
        &*embedder,
        &bus,
    )
    .await;
    match &result {
        Ok((response, _)) => eprintln!("[Chat] agent response ({} chars)", response.len()),
        Err(e) => eprintln!("[Chat] agent error: {}", e),
    }
    let (response, new_messages) = result?;

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