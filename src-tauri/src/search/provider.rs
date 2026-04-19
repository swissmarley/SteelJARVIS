use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub struct SearchProvider {
    client: Client,
    pub api_key: Option<String>,
    pub engine_id: Option<String>,
}

impl SearchProvider {
    pub fn new(api_key: Option<String>, engine_id: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            engine_id,
        }
    }

    /// Search using Google Custom Search API
    /// Falls back to a simple response if no API key is configured
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>, super::SearchError> {
        match (&self.api_key, &self.engine_id) {
            (Some(key), Some(id)) if !key.is_empty() && !id.is_empty() => {
                self.google_search(query, limit, key, id).await
            }
            _ => {
                // No search API configured — return a placeholder
                Ok(vec![SearchResult {
                    title: "Search not configured".to_string(),
                    url: format!("https://www.google.com/search?q={}", urlencoding(query)),
                    snippet: "Set SEARCH_API_KEY and SEARCH_ENGINE_ID in .env to enable web search.".to_string(),
                }])
            }
        }
    }

    async fn google_search(
        &self,
        query: &str,
        limit: u32,
        api_key: &str,
        engine_id: &str,
    ) -> Result<Vec<SearchResult>, super::SearchError> {
        let url = format!(
            "https://www.googleapis.com/customsearch/v1?key={}&cx={}&q={}&num={}",
            api_key, engine_id, urlencoding(query), limit
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(super::SearchError::ApiError(body));
        }

        let data: serde_json::Value = response.json().await?;

        let items = data.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();

        let results: Vec<SearchResult> = items
            .iter()
            .take(limit as usize)
            .filter_map(|item| {
                Some(SearchResult {
                    title: item.get("title")?.as_str()?.to_string(),
                    url: item.get("link")?.as_str()?.to_string(),
                    snippet: item.get("snippet")?.as_str()?.to_string(),
                })
            })
            .collect();

        Ok(results)
    }
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "+")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('?', "%3F")
        .replace('#', "%23")
}