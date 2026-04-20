use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::MemoryError;

/// Serialize an f32 vector as little-endian bytes for SQLite BLOB storage.
pub(crate) fn encode_embedding(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

/// Decode an f32 vector from little-endian bytes. Returns `None` if the
/// byte length isn't a multiple of 4.
pub(crate) fn decode_embedding(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Some(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryCategory {
    Profile,
    Preferences,
    Facts,
    TaskHistory,
    Workflows,
    AppPreferences,
    Recruiting,
    Relationships,
    Notes,
}

impl MemoryCategory {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "profile" => Self::Profile,
            "preferences" => Self::Preferences,
            "facts" => Self::Facts,
            "task_history" => Self::TaskHistory,
            "workflows" => Self::Workflows,
            "app_preferences" => Self::AppPreferences,
            "recruiting" => Self::Recruiting,
            "relationships" => Self::Relationships,
            _ => Self::Notes,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Profile => "profile",
            Self::Preferences => "preferences",
            Self::Facts => "facts",
            Self::TaskHistory => "task_history",
            Self::Workflows => "workflows",
            Self::AppPreferences => "app_preferences",
            Self::Recruiting => "recruiting",
            Self::Relationships => "relationships",
            Self::Notes => "notes",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemorySource {
    Explicit,
    AutoExtracted,
    ToolResult,
}

impl MemorySource {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "explicit" => Self::Explicit,
            "auto_extracted" => Self::AutoExtracted,
            "tool_result" => Self::ToolResult,
            _ => Self::Explicit,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Explicit => "explicit",
            Self::AutoExtracted => "auto_extracted",
            Self::ToolResult => "tool_result",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: String,
    pub confidence: f64,
    pub source: String,
    pub privacy_label: String,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
    pub access_count: i32,
}

pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn new(path: &Path) -> Result<Self, MemoryError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;

        // Idempotent migration: older DBs may not have `embedding`. SQLite
        // has no ADD COLUMN IF NOT EXISTS, so probe first.
        let has_embedding: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('memories') WHERE name = 'embedding'")
            .and_then(|mut s| s.exists([]))?;
        if !has_embedding {
            conn.execute("ALTER TABLE memories ADD COLUMN embedding BLOB", [])?;
        }

        Ok(Self { conn })
    }

    pub fn save(
        &mut self,
        content: &str,
        category: MemoryCategory,
        source: &str,
    ) -> Result<MemoryEntry, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let src = MemorySource::from_str(source);

        self.conn.execute(
            "INSERT INTO memories (id, content, category, confidence, source, privacy_label, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, content, category.as_str(), 1.0, src.as_str(), "normal", now, now],
        )?;

        Ok(MemoryEntry {
            id,
            content: content.to_string(),
            category: category.as_str().to_string(),
            confidence: 1.0,
            source: src.as_str().to_string(),
            privacy_label: "normal".to_string(),
            pinned: false,
            created_at: now.clone(),
            updated_at: now,
            access_count: 0,
        })
    }

    pub fn search(&self, query: &str, limit: u32) -> Result<Vec<MemoryEntry>, MemoryError> {
        let pattern = format!("%{}%", query.replace('%', "\\%"));
        let mut stmt = self.conn.prepare(
            "SELECT id, content, category, confidence, source, privacy_label, pinned, created_at, updated_at, access_count
             FROM memories
             WHERE content LIKE ?1
             ORDER BY updated_at DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![pattern, limit], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    category: row.get(2)?,
                    confidence: row.get(3)?,
                    source: row.get(4)?,
                    privacy_label: row.get(5)?,
                    pinned: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    access_count: row.get(9)?,
                })
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    pub fn list(
        &self,
        category: Option<MemoryCategory>,
        limit: u32,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let (query, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(cat) = category {
            (
                "SELECT id, content, category, confidence, source, privacy_label, pinned, created_at, updated_at, access_count
                 FROM memories WHERE category = ?1 ORDER BY updated_at DESC LIMIT ?2",
                vec![Box::new(cat.as_str().to_string()), Box::new(limit)],
            )
        } else {
            (
                "SELECT id, content, category, confidence, source, privacy_label, pinned, created_at, updated_at, access_count
                 FROM memories ORDER BY updated_at DESC LIMIT ?1",
                vec![Box::new(limit)],
            )
        };

        let mut stmt = self.conn.prepare(query)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let entries = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    category: row.get(2)?,
                    confidence: row.get(3)?,
                    source: row.get(4)?,
                    privacy_label: row.get(5)?,
                    pinned: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    access_count: row.get(9)?,
                })
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    pub fn delete(&mut self, id: &str) -> Result<(), MemoryError> {
        let changed = self.conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(MemoryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn set_pinned(&mut self, id: &str, pinned: bool) -> Result<(), MemoryError> {
        let changed = self.conn.execute(
            "UPDATE memories SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
            params![pinned, chrono::Utc::now().to_rfc3339(), id],
        )?;
        if changed == 0 {
            return Err(MemoryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn health_check(&self) -> bool {
        self.conn
            .execute_batch("SELECT 1;")
            .is_ok()
    }

    /// Like `save()`, but also stores an optional embedding.
    pub fn save_with_embedding(
        &mut self,
        content: &str,
        category: MemoryCategory,
        source: &str,
        embedding: Option<&[f32]>,
    ) -> Result<MemoryEntry, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let src = MemorySource::from_str(source);
        let emb_bytes = embedding.map(encode_embedding);

        self.conn.execute(
            "INSERT INTO memories (id, content, category, confidence, source, privacy_label, created_at, updated_at, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                content,
                category.as_str(),
                1.0,
                src.as_str(),
                "normal",
                now,
                now,
                emb_bytes,
            ],
        )?;

        Ok(MemoryEntry {
            id,
            content: content.to_string(),
            category: category.as_str().to_string(),
            confidence: 1.0,
            source: src.as_str().to_string(),
            privacy_label: "normal".to_string(),
            pinned: false,
            created_at: now.clone(),
            updated_at: now,
            access_count: 0,
        })
    }

    /// Linear-scan semantic search. Decodes each row's embedding, computes
    /// cosine similarity against `query`, drops rows below `min_sim`, sorts
    /// descending, truncates to `limit`. Rows with NULL embedding are skipped.
    pub fn semantic_search(
        &self,
        query: &[f32],
        limit: u32,
        min_sim: f32,
    ) -> Result<Vec<(MemoryEntry, f32)>, MemoryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, category, confidence, source, privacy_label, pinned, created_at, updated_at, access_count, embedding
             FROM memories
             WHERE embedding IS NOT NULL",
        )?;

        // Single-pass: decode + score + threshold-filter each row as we stream
        // it from SQLite. Avoids allocating a `Vec<(MemoryEntry, Vec<u8>)>`
        // just to throw most of it away, which matters once the user has
        // thousands of memories. The only Vec we keep is `scored`, which is
        // already bounded by `min_sim`.
        let rows = stmt.query_map([], |row| {
            let entry = MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                category: row.get(2)?,
                confidence: row.get(3)?,
                source: row.get(4)?,
                privacy_label: row.get(5)?,
                pinned: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                access_count: row.get(9)?,
            };
            let emb_bytes: Vec<u8> = row.get(10)?;
            Ok((entry, emb_bytes))
        })?;

        let mut scored: Vec<(MemoryEntry, f32)> = Vec::new();
        for row in rows {
            // Genuine DB errors propagate; codec-level corruption (empty /
            // odd-length blob, dim mismatch) is logged and skipped so one bad
            // row can't poison the whole query.
            let (entry, bytes) = row?;
            let emb = match decode_embedding(&bytes) {
                Some(v) => v,
                None => {
                    eprintln!(
                        "[semantic_search] skipping memory {} — malformed embedding blob ({} bytes)",
                        entry.id,
                        bytes.len()
                    );
                    continue;
                }
            };
            let sim = crate::memory::Embedder::cosine(query, &emb);
            if sim >= min_sim {
                scored.push((entry, sim));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit as usize);
        Ok(scored)
    }

    /// Returns the top `limit` pinned memories ordered by update recency.
    /// Used for the greeting's "always relevant" context block.
    #[allow(dead_code)]
    pub fn list_pinned(&self, limit: u32) -> Result<Vec<MemoryEntry>, MemoryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, category, confidence, source, privacy_label, pinned, created_at, updated_at, access_count
             FROM memories WHERE pinned = 1 ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(params![limit], |row| {
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    category: row.get(2)?,
                    confidence: row.get(3)?,
                    source: row.get(4)?,
                    privacy_label: row.get(5)?,
                    pinned: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    access_count: row.get(9)?,
                })
            })?
            .filter_map(|e| e.ok())
            .collect();
        Ok(entries)
    }
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    category TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    source TEXT NOT NULL DEFAULT 'explicit',
    privacy_label TEXT DEFAULT 'normal',
    pinned BOOLEAN DEFAULT FALSE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed TEXT,
    access_count INTEGER DEFAULT 0,
    conversation_id TEXT,
    metadata TEXT,
    embedding BLOB
);

CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_updated ON memories(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_pinned ON memories(pinned);
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_roundtrip_preserves_values() {
        let v = vec![0.1f32, -0.2, 3.14, -123.456];
        let bytes = encode_embedding(&v);
        let back = decode_embedding(&bytes).expect("decode");
        assert_eq!(back.len(), v.len());
        for (a, b) in v.iter().zip(back.iter()) {
            assert!((a - b).abs() < 1e-6, "{a} != {b}");
        }
    }

    #[test]
    fn decode_rejects_odd_length_bytes() {
        let bad = vec![1u8, 2, 3];
        assert!(decode_embedding(&bad).is_none());
    }

    fn temp_store() -> MemoryStore {
        let dir = std::env::temp_dir().join(format!("jarvis-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("jarvis.db");
        MemoryStore::new(&path).unwrap()
    }

    #[test]
    fn semantic_search_ranks_closest_first() {
        let mut store = temp_store();
        // Fake 2-dim embeddings so we can hand-pick similarity.
        let emb_a = vec![1.0, 0.0]; // "coffee"
        let emb_b = vec![0.9, 0.1]; // "espresso" — near a
        let emb_c = vec![0.0, 1.0]; // "paris" — orthogonal
        store
            .save_with_embedding("coffee preference", MemoryCategory::Preferences, "explicit", Some(&emb_a))
            .unwrap();
        store
            .save_with_embedding("espresso preference", MemoryCategory::Preferences, "explicit", Some(&emb_b))
            .unwrap();
        store
            .save_with_embedding("france fact", MemoryCategory::Facts, "explicit", Some(&emb_c))
            .unwrap();

        // Query that should match the coffee/espresso cluster.
        let query = vec![0.95, 0.05];
        let results = store.semantic_search(&query, 2, 0.0).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].0.content.contains("preference"));
        // First two results should be the preference rows, not the france fact.
        assert!(results.iter().all(|(e, _)| e.content.contains("preference")));
    }

    #[test]
    fn semantic_search_skips_null_embeddings() {
        let mut store = temp_store();
        store
            .save_with_embedding("unembedded", MemoryCategory::Notes, "explicit", None)
            .unwrap();
        let query = vec![1.0, 0.0];
        let results = store.semantic_search(&query, 5, 0.0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn semantic_search_respects_threshold() {
        let mut store = temp_store();
        let emb = vec![0.0, 1.0];
        store
            .save_with_embedding("unrelated", MemoryCategory::Notes, "explicit", Some(&emb))
            .unwrap();
        let query = vec![1.0, 0.0]; // orthogonal, sim = 0.0
        let results = store.semantic_search(&query, 5, 0.3).unwrap();
        assert!(results.is_empty(), "threshold should filter out sim=0 match");
    }

    #[test]
    fn semantic_search_truncates_to_limit() {
        let mut store = temp_store();
        // Three rows, all near the query.
        for (i, label) in ["first", "second", "third"].iter().enumerate() {
            let emb = vec![1.0 - 0.01 * i as f32, 0.01 * i as f32];
            store
                .save_with_embedding(label, MemoryCategory::Notes, "explicit", Some(&emb))
                .unwrap();
        }
        let query = vec![1.0, 0.0];
        let results = store.semantic_search(&query, 1, 0.0).unwrap();
        assert_eq!(results.len(), 1, "limit=1 must return exactly one row");
        assert_eq!(results[0].0.content, "first");
    }
}
