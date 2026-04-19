use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::MemoryError;

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
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_updated ON memories(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_pinned ON memories(pinned);
";