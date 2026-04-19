pub mod store;

pub use store::{MemoryStore, MemoryEntry, MemoryCategory};

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
}