pub mod provider;

pub use provider::SearchProvider;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Search API error: {0}")]
    ApiError(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[allow(dead_code)]
    #[error("No API key configured")]
    NoApiKey,
}