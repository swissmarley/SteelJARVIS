use std::sync::Mutex;
use tauri::State;

use crate::search::SearchProvider;

#[tauri::command]
pub async fn web_search(
    query: String,
    limit: Option<u32>,
    provider: State<'_, Mutex<SearchProvider>>,
) -> Result<Vec<serde_json::Value>, String> {
    let (api_key, engine_id) = {
        let p = provider.lock().map_err(|e| e.to_string())?;
        (p.api_key.clone(), p.engine_id.clone())
    };

    let search = SearchProvider::new(api_key, engine_id);
    let results = search.search(&query, limit.unwrap_or(5)).await.map_err(|e| e.to_string())?;

    Ok(results.into_iter().map(|r| {
        serde_json::json!({
            "title": r.title,
            "url": r.url,
            "snippet": r.snippet,
        })
    }).collect())
}