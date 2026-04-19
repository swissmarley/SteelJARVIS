pub mod store;
pub mod embedder;

pub use store::{MemoryStore, MemoryEntry, MemoryCategory};
#[allow(unused_imports)]
pub use embedder::Embedder;

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
}