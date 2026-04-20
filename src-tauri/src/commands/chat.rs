use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

use crate::agent::{build_context, try_greet, AgentEngine};
use crate::memory::{Embedder, MemoryStore};
use crate::observability::EventBus;
use crate::session::SessionTracker;

#[tauri::command]
pub async fn send_message(
    message: String,
    app: AppHandle,
    engine: State<'_, Mutex<AgentEngine>>,
    mem_store: State<'_, Mutex<MemoryStore>>,
    embedder: State<'_, Embedder>,
    tracker: State<'_, SessionTracker>,
    event_bus: State<'_, Arc<Mutex<EventBus>>>,
) -> Result<String, String> {
    eprintln!("[Chat] send_message invoked, text={:?}", message);

    // Greeting pre-step — shared with the voice path. `try_greet` handles
    // the should_greet / is_pure_greeting decision tree and mark_greeted
    // bookkeeping. We only differ in how we speak the returned text: the
    // chat path emits a Tauri event so the frontend can invoke TTS.
    let greeting_api_key = {
        let e = engine.lock().map_err(|e| e.to_string())?;
        e.api_key().to_string()
    };
    if let Some(text) = try_greet(
        &greeting_api_key,
        &message,
        &mem_store,
        &embedder,
        &tracker,
        event_bus.as_ref(),
    )
    .await
    {
        let _ = app.emit("jarvis-greeting-speak", text);
    }

    tracker.mark_interaction();

    let (api_key, history) = {
        let e = engine.lock().map_err(|e| e.to_string())?;
        (e.api_key().to_string(), e.history().to_vec())
    };

    if api_key.is_empty() {
        return Err("ANTHROPIC_API_KEY is not configured. Add it to .env or the environment and restart.".to_string());
    }

    let ctx = build_context(&mem_store, &embedder, &tracker, Some(&message));
    let bus = event_bus.lock().map_err(|e| e.to_string())?.clone();

    let result = AgentEngine::send_with(
        &api_key,
        &history,
        &message,
        &ctx,
        &mem_store,
        &embedder,
        &bus,
    )
    .await;
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
