use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

use crate::agent::{build_context, generate_greeting, AgentEngine};
use crate::memory::{Embedder, MemoryStore};
use crate::observability::{EventBus, JarvisEvent};
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

    // Greeting pre-step. Two cases:
    //   a) due + the message is itself a pure greeting → skip the extra API
    //      call (the agent's own reply will cover the hello), but still mark
    //      greeted so we don't re-fire on the very next turn.
    //   b) due + real task message → generate a contextual greeting.
    // In both branches we call `mark_greeted` even on failure so a flaky
    // Anthropic call can't cause a greeting-per-turn storm.
    if tracker.should_greet() {
        if is_pure_greeting(&message) {
            tracker.mark_greeted();
        } else {
            tracker.mark_greeted();
            let greeting_ctx = build_context(&mem_store, &embedder, &tracker, None);
            let api_key = {
                let e = engine.lock().map_err(|e| e.to_string())?;
                e.api_key().to_string()
            };
            if !api_key.is_empty() {
                match generate_greeting(&api_key, &greeting_ctx).await {
                    Ok(text) => {
                        if let Ok(bus) = event_bus.lock() {
                            bus.emit(JarvisEvent::JarvisGreeting { text: text.clone() });
                        }
                        // Speak the greeting too so behavior matches voice path.
                        let _ = app.emit("jarvis-greeting-speak", text);
                    }
                    Err(e) => eprintln!("[Chat] greeting skipped: {e}"),
                }
            }
        }
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

/// Simple heuristic: is this just a hello-style utterance with no task?
fn is_pure_greeting(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    if t.len() > 30 {
        return false;
    }
    matches!(
        t.as_str(),
        "hi" | "hello" | "hey" | "jarvis" | "hi jarvis" | "hello jarvis"
            | "hey jarvis" | "good morning" | "good afternoon" | "good evening"
            | "are you there" | "are you there jarvis" | "jarvis are you there"
    )
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