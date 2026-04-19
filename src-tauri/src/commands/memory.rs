use std::sync::{Arc, Mutex};
use tauri::State;

use crate::memory::{MemoryStore, MemoryEntry, MemoryCategory};
use crate::observability::{EventBus, JarvisEvent};

#[tauri::command]
pub fn save_memory(
    content: String,
    category: String,
    source: String,
    store: State<'_, Mutex<MemoryStore>>,
    event_bus: State<'_, Arc<Mutex<EventBus>>>,
) -> Result<MemoryEntry, String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;
    let cat = MemoryCategory::from_str(&category);
    let entry = store.save(&content, cat, &source).map_err(|e| e.to_string())?;

    let event_bus = event_bus.lock().map_err(|e| e.to_string())?;
    event_bus.emit(JarvisEvent::MemorySaved {
        id: entry.id.clone(),
        category: entry.category.clone(),
        preview: content.chars().take(100).collect::<String>(),
    });

    Ok(entry)
}

#[tauri::command]
pub fn search_memories(
    query: String,
    limit: Option<u32>,
    store: State<'_, Mutex<MemoryStore>>,
) -> Result<Vec<MemoryEntry>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    store.search(&query, limit.unwrap_or(10)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_memories(
    category: Option<String>,
    limit: Option<u32>,
    store: State<'_, Mutex<MemoryStore>>,
) -> Result<Vec<MemoryEntry>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    let cat = category.as_ref().and_then(|c| Some(MemoryCategory::from_str(c)));
    store.list(cat, limit.unwrap_or(50)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memory(
    id: String,
    store: State<'_, Mutex<MemoryStore>>,
) -> Result<(), String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;
    store.delete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_memory(
    id: String,
    pinned: bool,
    store: State<'_, Mutex<MemoryStore>>,
) -> Result<(), String> {
    let mut store = store.lock().map_err(|e| e.to_string())?;
    store.set_pinned(&id, pinned).map_err(|e| e.to_string())
}