# JARVIS Persona, Long-Term Memory, Contextual Greeting — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire JARVIS's persona, local-embedding-backed memory, and contextual greeting in the Rust Tauri backend so the assistant feels like a persistent butler rather than a stateless chatbot.

**Architecture:** Add a `fastembed`-backed `Embedder` module; extend `MemoryStore` with an embedding BLOB column and `semantic_search`; introduce `SessionTracker` managed state; refactor `AgentEngine` to take an `AgentContext` (time, name, last-interaction, top memories) that feeds into a new JARVIS-persona system prompt; wire the `save_memory` / new `recall_memory` tools to the store; add a first-of-session / >30min-idle greeting flow with a dedicated Tauri event.

**Tech Stack:** Rust (Tauri 2.x), `fastembed` (ONNX local embeddings, `BAAI/bge-small-en-v1.5`, 384-dim), SQLite via `rusqlite` (bundled), React/TypeScript/Zustand frontend.

**Spec reference:** `docs/superpowers/specs/2026-04-19-jarvis-persona-memory-design.md`

---

## File Structure

### New files

| Path | Responsibility |
|---|---|
| `src-tauri/src/memory/embedder.rs` | Thin wrapper over `fastembed::TextEmbedding`; lazy init; graceful failure path |
| `src-tauri/src/session/mod.rs` | Module barrel |
| `src-tauri/src/session/tracker.rs` | `SessionTracker` state + greeting decision logic |

### Modified files

| Path | What changes |
|---|---|
| `src-tauri/Cargo.toml` | Add `fastembed = "5"` dependency |
| `src-tauri/src/memory/mod.rs` | Re-export `Embedder` |
| `src-tauri/src/memory/store.rs` | Schema migration; `embedding` column; `semantic_search` method |
| `src-tauri/src/agent/engine.rs` | `AgentContext` struct; persona prompt; `send_with(..., ctx)`; `save_memory`/`recall_memory` real impl; `generate_greeting` |
| `src-tauri/src/commands/chat.rs` | Build context before `send_with`; mark interaction; greeting pre-step |
| `src-tauri/src/voice/stt.rs` | Same, for the STT-driven agent dispatch |
| `src-tauri/src/observability/event_bus.rs` | New `JarvisGreeting` variant |
| `src-tauri/src/lib.rs` | Register `session` + `memory::embedder`; manage state |
| `src/App.tsx` | Handle `jarvis-greeting` Tauri event |

Tests live inline (`#[cfg(test)] mod tests { ... }`) following the existing pattern from `src-tauri/src/voice/clap_detector.rs`.

---

## Task 1: Add `fastembed` dependency and create `Embedder` module

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/memory/embedder.rs`
- Modify: `src-tauri/src/memory/mod.rs`

- [ ] **Step 1: Add fastembed to Cargo.toml**

Edit `src-tauri/Cargo.toml`, adding to the `[dependencies]` section after the existing `chrono` line:

```toml
fastembed = "5"
```

- [ ] **Step 2: Create embedder module with lazy-initialized model**

Create `src-tauri/src/memory/embedder.rs`:

```rust
use std::sync::OnceLock;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Wraps a `fastembed` text-embedding model behind a lazy, thread-safe
/// initializer. The model files are downloaded to the user's cache dir on
/// first use (~90MB for bge-small-en-v1.5). All embed() failures are soft —
/// callers fall back to keyword search when this returns Err.
pub struct Embedder {
    inner: OnceLock<Result<TextEmbedding, String>>,
}

impl Embedder {
    pub fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    /// Returns the 384-dim embedding for `text` as a Vec<f32>.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let model = self.inner.get_or_init(|| {
            TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::BGESmallENV15)
                    .with_show_download_progress(true),
            )
            .map_err(|e| format!("fastembed init failed: {e}"))
        });

        let model = model.as_ref().map_err(|e| e.clone())?;
        let mut vectors = model
            .embed(vec![text.to_string()], None)
            .map_err(|e| format!("fastembed embed failed: {e}"))?;
        vectors
            .pop()
            .ok_or_else(|| "fastembed returned empty result".to_string())
    }

    /// Cosine similarity between two embeddings of the same dimension.
    pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let mut dot = 0.0f32;
        let mut na = 0.0f32;
        let mut nb = 0.0f32;
        for i in 0..a.len() {
            dot += a[i] * b[i];
            na += a[i] * a[i];
            nb += b[i] * b[i];
        }
        if na == 0.0 || nb == 0.0 {
            return 0.0;
        }
        dot / (na.sqrt() * nb.sqrt())
    }
}

impl Default for Embedder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identity_is_one() {
        let v = vec![0.1, 0.2, 0.3, 0.4];
        let sim = Embedder::cosine(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5, "expected 1.0, got {sim}");
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(Embedder::cosine(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn cosine_mismatched_len_returns_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert_eq!(Embedder::cosine(&a, &b), 0.0);
    }

    // This test downloads the model on first run (~90MB) and is marked
    // `ignore` so `cargo test` stays fast by default. Run explicitly with:
    //   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored embed
    #[test]
    #[ignore]
    fn similar_sentences_have_higher_similarity() {
        let e = Embedder::new();
        let coffee_a = e.embed("I really like coffee in the morning").unwrap();
        let coffee_b = e.embed("Espresso is my favorite morning drink").unwrap();
        let unrelated = e.embed("The capital of France is Paris").unwrap();
        let sim_similar = Embedder::cosine(&coffee_a, &coffee_b);
        let sim_unrelated = Embedder::cosine(&coffee_a, &unrelated);
        assert!(
            sim_similar > sim_unrelated,
            "similar pair sim {sim_similar} should exceed unrelated {sim_unrelated}"
        );
    }
}
```

- [ ] **Step 3: Re-export Embedder from memory module**

Edit `src-tauri/src/memory/mod.rs`:

```rust
pub mod store;
pub mod embedder;

pub use store::{MemoryStore, MemoryEntry, MemoryCategory};
pub use embedder::Embedder;

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
}
```

- [ ] **Step 4: Compile check + fast tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib memory::embedder -- --skip ignored`
Expected: 3 tests pass (cosine_identity, cosine_orthogonal, cosine_mismatched). The `similar_sentences_have_higher_similarity` test is ignored and skipped.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/memory/embedder.rs src-tauri/src/memory/mod.rs
git commit -m "feat(memory): add fastembed-backed Embedder with cosine helper"
```

---

## Task 2: Add `embedding` column and storage helpers to `MemoryStore`

**Files:**
- Modify: `src-tauri/src/memory/store.rs`

- [ ] **Step 1: Write the failing test for bytes↔vec conversion**

Append a test module to `src-tauri/src/memory/store.rs` (or create one if none exists). Add near the bottom of the file:

```rust
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
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib memory::store::tests`
Expected: FAIL — `encode_embedding` / `decode_embedding` don't exist yet.

- [ ] **Step 3: Add the helpers and the schema migration**

Edit `src-tauri/src/memory/store.rs`. Add two free functions near the top of the file, just below the imports:

```rust
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
```

Then update the SCHEMA constant at the bottom of the file:

```rust
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
```

And in `MemoryStore::new`, after `conn.execute_batch(SCHEMA)?;`, add an idempotent migration for databases created before this change:

```rust
impl MemoryStore {
    pub fn new(path: &Path) -> Result<Self, MemoryError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;

        // Idempotent migration: older DBs may not have `embedding`. SQLite
        // has no ADD COLUMN IF NOT EXISTS, so probe first.
        let has_embedding: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('memories') WHERE name = 'embedding'")
            .and_then(|mut s| s.exists([]))
            .unwrap_or(false);
        if !has_embedding {
            conn.execute("ALTER TABLE memories ADD COLUMN embedding BLOB", [])?;
        }

        Ok(Self { conn })
    }
    // ... rest unchanged
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib memory::store`
Expected: both `embedding_roundtrip_preserves_values` and `decode_rejects_odd_length_bytes` pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/memory/store.rs
git commit -m "feat(memory): add embedding column with idempotent migration"
```

---

## Task 3: Add `semantic_search` and `save_with_embedding` to `MemoryStore`

**Files:**
- Modify: `src-tauri/src/memory/store.rs`

- [ ] **Step 1: Write the failing test**

Append to the existing `tests` module in `src-tauri/src/memory/store.rs`:

```rust
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
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib memory::store::tests::semantic_search`
Expected: FAIL — `save_with_embedding` and `semantic_search` don't exist.

- [ ] **Step 3: Implement `save_with_embedding` and `semantic_search`**

Edit `src-tauri/src/memory/store.rs`. Keep the existing `save()` method as a thin wrapper for callers that don't want an embedding. Add these methods inside `impl MemoryStore`:

```rust
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

        let mut scored: Vec<(MemoryEntry, f32)> = rows
            .filter_map(|r| r.ok())
            .filter_map(|(entry, bytes)| {
                let emb = decode_embedding(&bytes)?;
                let sim = crate::memory::Embedder::cosine(query, &emb);
                if sim >= min_sim {
                    Some((entry, sim))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit as usize);
        Ok(scored)
    }

    /// Returns the top `limit` pinned memories ordered by update recency.
    /// Used for the greeting's "always relevant" context block.
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib memory::store`
Expected: all 5 store tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/memory/store.rs
git commit -m "feat(memory): add save_with_embedding, semantic_search, list_pinned"
```

---

## Task 4: Create `SessionTracker` module

**Files:**
- Create: `src-tauri/src/session/mod.rs`
- Create: `src-tauri/src/session/tracker.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod session;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/session/tracker.rs`:

```rust
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Local};

/// Idle threshold beyond which JARVIS re-greets the user.
pub const IDLE_THRESHOLD: Duration = Duration::from_secs(30 * 60);

/// Tracks when the user last interacted and when JARVIS last greeted.
/// Drives the decision to play a contextual greeting on the next utterance.
pub struct SessionTracker {
    last_interaction: Mutex<Option<DateTime<Local>>>,
    last_greeting: Mutex<Option<DateTime<Local>>>,
    session_started: DateTime<Local>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            last_interaction: Mutex::new(None),
            last_greeting: Mutex::new(None),
            session_started: Local::now(),
        }
    }

    pub fn session_started(&self) -> DateTime<Local> {
        self.session_started
    }

    pub fn last_interaction(&self) -> Option<DateTime<Local>> {
        self.last_interaction.lock().ok().and_then(|g| *g)
    }

    pub fn mark_interaction(&self) {
        if let Ok(mut g) = self.last_interaction.lock() {
            *g = Some(Local::now());
        }
    }

    pub fn mark_greeted(&self) {
        if let Ok(mut g) = self.last_greeting.lock() {
            *g = Some(Local::now());
        }
    }

    /// True when JARVIS should greet before responding:
    /// * no greeting yet this session, OR
    /// * `now - last_interaction > IDLE_THRESHOLD`.
    /// Poisoned locks degrade to `false` rather than panicking.
    pub fn should_greet(&self) -> bool {
        let greeted = match self.last_greeting.lock() {
            Ok(g) => *g,
            Err(_) => return false,
        };
        if greeted.is_none() {
            return true;
        }
        let last = match self.last_interaction.lock() {
            Ok(g) => *g,
            Err(_) => return false,
        };
        match last {
            Some(ts) => {
                let elapsed = Local::now().signed_duration_since(ts);
                elapsed.to_std().map(|d| d > IDLE_THRESHOLD).unwrap_or(false)
            }
            None => true,
        }
    }
}

impl Default for SessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_should_greet() {
        let t = SessionTracker::new();
        assert!(t.should_greet(), "fresh tracker must greet");
    }

    #[test]
    fn after_greeting_without_idle_no_greet() {
        let t = SessionTracker::new();
        t.mark_greeted();
        t.mark_interaction();
        assert!(!t.should_greet());
    }

    #[test]
    fn after_greeting_then_long_idle_greets_again() {
        let t = SessionTracker::new();
        t.mark_greeted();
        // Simulate idle by back-dating last_interaction well beyond threshold.
        {
            let mut g = t.last_interaction.lock().unwrap();
            *g = Some(Local::now() - chrono::Duration::minutes(45));
        }
        assert!(t.should_greet(), "45min idle should trigger re-greet");
    }

    #[test]
    fn mark_interaction_updates_last_interaction() {
        let t = SessionTracker::new();
        assert!(t.last_interaction().is_none());
        t.mark_interaction();
        assert!(t.last_interaction().is_some());
    }
}
```

- [ ] **Step 2: Create the module barrel**

Create `src-tauri/src/session/mod.rs`:

```rust
pub mod tracker;

pub use tracker::{SessionTracker, IDLE_THRESHOLD};
```

- [ ] **Step 3: Register the module in lib.rs**

Edit `src-tauri/src/lib.rs`. Add `session` to the `mod` declarations near the top (in alphabetical-ish order — place after `search`):

```rust
mod commands;
mod agent;
mod memory;
mod desktop;
mod voice;
mod search;
mod session;
mod permissions;
mod observability;
mod config;
mod tray;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib session::`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/session/mod.rs src-tauri/src/session/tracker.rs src-tauri/src/lib.rs
git commit -m "feat(session): add SessionTracker with greeting-decision logic"
```

---

## Task 5: Register `Embedder` and `SessionTracker` as Tauri managed state

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add imports and manage state**

Edit `src-tauri/src/lib.rs`. At the top, extend the `use voice::...` block area with new imports:

```rust
use voice::{SpeechManager, ClapDetector, ActivationManager, AudioCapture, SpeechRecognizer};
use voice::clap_detector::ClapConfig;
use search::SearchProvider;
use memory::Embedder;
use session::SessionTracker;
```

Inside the `.setup(|app| { ... })` block, after the existing `MemoryStore` registration (around the `app.manage(Mutex::new(mem_store));` line) add:

```rust
            let embedder = Embedder::new();
            app.manage(embedder);

            let session_tracker = SessionTracker::new();
            app.manage(session_tracker);
```

Note: `Embedder` and `SessionTracker` already use interior `OnceLock` / `Mutex`, so we register them directly (not wrapped in an outer `Mutex`).

- [ ] **Step 2: Compile check**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check --manifest-path src-tauri/Cargo.toml`
Expected: builds clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(lib): register Embedder and SessionTracker as managed state"
```

---

## Task 6: Introduce `AgentContext` and refactor `build_system_prompt`

**Files:**
- Modify: `src-tauri/src/agent/engine.rs`

- [ ] **Step 1: Write the failing test**

Append a tests module at the bottom of `src-tauri/src/agent/engine.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn system_prompt_includes_persona_signature() {
        let ctx = AgentContext::default();
        let prompt = build_system_prompt(&ctx);
        assert!(prompt.contains("JARVIS"), "prompt must name-check JARVIS");
        assert!(prompt.to_lowercase().contains("butler") || prompt.contains("Tony Stark"));
    }

    #[test]
    fn system_prompt_renders_user_name_when_known() {
        let ctx = AgentContext {
            now: Local.with_ymd_and_hms(2026, 4, 19, 9, 30, 0).unwrap(),
            user_name: Some("Nakya".to_string()),
            last_interaction: None,
            memories: vec![],
        };
        let prompt = build_system_prompt(&ctx);
        assert!(prompt.contains("Nakya"), "known name should appear in context block");
    }

    #[test]
    fn system_prompt_falls_back_to_unknown_when_name_missing() {
        let ctx = AgentContext {
            now: Local.with_ymd_and_hms(2026, 4, 19, 9, 30, 0).unwrap(),
            user_name: None,
            last_interaction: None,
            memories: vec![],
        };
        let prompt = build_system_prompt(&ctx);
        assert!(prompt.contains("unknown") || prompt.to_lowercase().contains("sir"));
    }

    #[test]
    fn system_prompt_renders_memories_with_category() {
        let ctx = AgentContext {
            now: Local::now(),
            user_name: None,
            last_interaction: None,
            memories: vec![MemoryEntry {
                id: "id-1".into(),
                content: "User prefers espresso".into(),
                category: "preferences".into(),
                confidence: 1.0,
                source: "explicit".into(),
                privacy_label: "normal".into(),
                pinned: false,
                created_at: "".into(),
                updated_at: "".into(),
                access_count: 0,
            }],
        };
        let prompt = build_system_prompt(&ctx);
        assert!(prompt.contains("User prefers espresso"));
        assert!(prompt.contains("[preferences]"));
    }
}
```

- [ ] **Step 2: Add the `AgentContext` struct and the refactored prompt builder**

Edit `src-tauri/src/agent/engine.rs`. At the top of the file, after the existing `use` lines, add:

```rust
use chrono::{DateTime, Local};

use crate::memory::MemoryEntry;
```

Then add the struct near the top of the file, below the existing constants:

```rust
/// Per-turn context injected into the JARVIS system prompt.
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub now: DateTime<Local>,
    pub user_name: Option<String>,
    pub last_interaction: Option<DateTime<Local>>,
    pub memories: Vec<MemoryEntry>,
}

impl Default for AgentContext {
    fn default() -> Self {
        Self {
            now: Local::now(),
            user_name: None,
            last_interaction: None,
            memories: vec![],
        }
    }
}
```

Replace the existing `fn build_system_prompt() -> String { ... }` at the bottom of the file with:

```rust
pub fn build_system_prompt(ctx: &AgentContext) -> String {
    let name = ctx
        .user_name
        .as_deref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown (address as \"Sir\")".to_string());

    let last_interaction = ctx
        .last_interaction
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "first contact".to_string());

    let memories_block = if ctx.memories.is_empty() {
        "(none yet)".to_string()
    } else {
        ctx.memories
            .iter()
            .map(|m| format!("[{}] {}", m.category, m.content))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"You are JARVIS — a highly intelligent, proactive desktop assistant modeled on Tony Stark's butler AI. You address the user by their name when you know it; otherwise, "Sir". You are confident, witty, professional, and concise. Favor short, complete sentences. Occasional dry humor is welcome, but never waste the user's time. When you have evidence of a preference, habit, or prior context, factor it in and briefly state why. You never roleplay as a human, and you never deny being an AI.

Current context:
- Local time: {now}
- User: {name}
- Last interaction: {last}

<user_memories>
{memories}
</user_memories>

You have tools for taking real actions on the user's Mac. Use them when an action is required. For conversation and questions, answer directly. When the user explicitly says "remember this" or similar, use save_memory. When you need to recall something about the user, use recall_memory."#,
        now = ctx.now.format("%Y-%m-%d %H:%M"),
        name = name,
        last = last_interaction,
        memories = memories_block,
    )
}
```

- [ ] **Step 3: Update `send_with` signature to accept `AgentContext` and use it**

Still in `src-tauri/src/agent/engine.rs`, change the `send_with` signature from:

```rust
    pub async fn send_with(
        api_key: &str,
        history: &[ClaudeMessage],
        message: &str,
        event_bus: &EventBus,
    ) -> Result<(String, Vec<ClaudeMessage>), String> {
```

to:

```rust
    pub async fn send_with(
        api_key: &str,
        history: &[ClaudeMessage],
        message: &str,
        ctx: &AgentContext,
        event_bus: &EventBus,
    ) -> Result<(String, Vec<ClaudeMessage>), String> {
```

Then inside the function body, replace the existing:

```rust
                system: build_system_prompt(),
```

with:

```rust
                system: build_system_prompt(ctx),
```

(This is the only use site inside the function; callers will be updated in Task 10.)

- [ ] **Step 4: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib agent::engine::tests`
Expected: 4 tests pass.

The broader `cargo check` will now FAIL because `send_with` callers (chat.rs, stt.rs) still pass the old signature. That's expected — Task 10 fixes them. Continue.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/agent/engine.rs
git commit -m "feat(agent): introduce AgentContext and JARVIS persona prompt"
```

---

## Task 7: Wire real `save_memory` tool implementation

**Files:**
- Modify: `src-tauri/src/agent/engine.rs`

- [ ] **Step 1: Change `execute_tool` signature to accept store and embedder**

Still in `src-tauri/src/agent/engine.rs`, the current `fn execute_tool(name, input) -> String` needs access to `MemoryStore` + `Embedder`. Change its signature:

```rust
use std::sync::Mutex;
use crate::memory::{MemoryStore, MemoryCategory, Embedder};

fn execute_tool(
    name: &str,
    input: &serde_json::Value,
    store: &Mutex<MemoryStore>,
    embedder: &Embedder,
) -> String {
```

Update the `save_memory` arm (currently a stub that returns a format string) to actually persist:

```rust
        "save_memory" => {
            let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let category = input.get("category").and_then(|v| v.as_str()).unwrap_or("notes");
            if content.is_empty() {
                return "save_memory called with empty content — nothing saved.".to_string();
            }
            let cat = MemoryCategory::from_str(category);
            let embedding = embedder.embed(content).ok();
            let mut guard = match store.lock() {
                Ok(g) => g,
                Err(e) => return format!("save_memory: memory store lock poisoned: {e}"),
            };
            match guard.save_with_embedding(content, cat, "explicit", embedding.as_deref()) {
                Ok(entry) => format!("Saved to {} (id={}).", entry.category, entry.id),
                Err(e) => format!("save_memory failed: {e}"),
            }
        }
```

- [ ] **Step 2: Update `send_with` to pass store and embedder to `execute_tool`**

Inside `send_with`, change the signature to add:

```rust
    pub async fn send_with(
        api_key: &str,
        history: &[ClaudeMessage],
        message: &str,
        ctx: &AgentContext,
        store: &Mutex<MemoryStore>,
        embedder: &Embedder,
        event_bus: &EventBus,
    ) -> Result<(String, Vec<ClaudeMessage>), String> {
```

Find the existing tool-use loop:

```rust
                        let result = execute_tool(name, &input);
```

Change to:

```rust
                        let result = execute_tool(name, &input, store, embedder);
```

- [ ] **Step 3: Compile check (expecting caller errors)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20`
Expected: errors in `commands/chat.rs` and `voice/stt.rs` because they call `send_with` with the old signature. This is expected — Task 10 fixes the callers.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/agent/engine.rs
git commit -m "feat(agent): wire save_memory tool to MemoryStore with embeddings"
```

---

## Task 8: Add `recall_memory` tool definition and implementation

**Files:**
- Modify: `src-tauri/src/agent/engine.rs`

- [ ] **Step 1: Add tool definition**

In `src-tauri/src/agent/engine.rs`, find `get_tool_definitions()` and add this entry inside the returned `vec![...]` before the recruiting tools:

```rust
        ToolDefinition {
            name: "recall_memory".to_string(),
            description: "Search the user's long-term memory for entries matching a natural-language query. Use this when the user asks what you know about them, or when you need prior context before answering.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "What to look for (e.g. 'drinks', 'work schedule', 'birthday')" },
                    "limit": { "type": "integer", "description": "Max results (default 6)", "minimum": 1, "maximum": 20 }
                },
                "required": ["query"]
            }),
        },
```

- [ ] **Step 2: Add the execution arm**

In `execute_tool`, add this arm alongside `save_memory`:

```rust
        "recall_memory" => {
            let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = input
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
                .unwrap_or(6);
            if query.is_empty() {
                return "recall_memory called with empty query.".to_string();
            }

            // Try semantic search first; fall back to LIKE if embedding fails.
            let embedding = embedder.embed(query).ok();
            let guard = match store.lock() {
                Ok(g) => g,
                Err(e) => return format!("recall_memory: lock poisoned: {e}"),
            };

            let results: Vec<(String, f32)> = if let Some(q_emb) = embedding {
                match guard.semantic_search(&q_emb, limit, 0.3) {
                    Ok(hits) => hits
                        .into_iter()
                        .map(|(e, s)| (format!("[{}] {}", e.category, e.content), s))
                        .collect(),
                    Err(e) => return format!("recall_memory search failed: {e}"),
                }
            } else {
                match guard.search(query, limit) {
                    Ok(hits) => hits
                        .into_iter()
                        .map(|e| (format!("[{}] {}", e.category, e.content), 0.0))
                        .collect(),
                    Err(e) => return format!("recall_memory fallback search failed: {e}"),
                }
            };

            if results.is_empty() {
                "No matching memories.".to_string()
            } else {
                results
                    .into_iter()
                    .map(|(line, sim)| {
                        if sim > 0.0 {
                            format!("{line} (sim {:.2})", sim)
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
```

- [ ] **Step 3: Compile check**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10`
Expected: still complains about `send_with` callers — that's Task 10. No *new* errors in engine.rs.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/agent/engine.rs
git commit -m "feat(agent): add recall_memory tool with semantic + LIKE fallback"
```

---

## Task 9: Add `generate_greeting` and `JarvisGreeting` event

**Files:**
- Modify: `src-tauri/src/agent/engine.rs`
- Modify: `src-tauri/src/observability/event_bus.rs`

- [ ] **Step 1: Add the `JarvisGreeting` event variant**

Edit `src-tauri/src/observability/event_bus.rs`. Inside the `JarvisEvent` enum, add a new variant after `VoiceAgentError`:

```rust
    /// Standalone greeting emitted when JARVIS proactively addresses the
    /// user (first of session or after long idle). Rendered as an assistant
    /// message without a paired user message.
    JarvisGreeting { text: String },
```

In `event_name()`, add:

```rust
            Self::JarvisGreeting { .. } => "jarvis-greeting",
```

In `payload()`, add:

```rust
            Self::JarvisGreeting { text } => serde_json::json!({"text": text}),
```

- [ ] **Step 2: Add `generate_greeting` to `AgentEngine`**

Edit `src-tauri/src/agent/engine.rs`. Add a free async function near the bottom of the file (above `build_system_prompt`):

```rust
/// Produces a single short greeting line based on the agent context. Uses a
/// dedicated system prompt (no tools — greetings should never take action).
pub async fn generate_greeting(
    api_key: &str,
    ctx: &AgentContext,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("No API key configured.".to_string());
    }

    let system = format!(
        r#"You are JARVIS. Produce a single short greeting (1–2 sentences max) for a returning user. Factor in: local time of day, how long they've been away, their name if known, and any pinned memories provided. Do not answer a question, start a task, or list options — just greet. Be warm, witty, concise. Address them by name if known; otherwise "Sir".

Context:
- Local time: {now}
- User: {name}
- Last interaction: {last}

<user_memories>
{memories}
</user_memories>"#,
        now = ctx.now.format("%Y-%m-%d %H:%M"),
        name = ctx
            .user_name
            .as_deref()
            .unwrap_or("unknown (address as \"Sir\")"),
        last = ctx
            .last_interaction
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "first contact".to_string()),
        memories = if ctx.memories.is_empty() {
            "(none yet)".to_string()
        } else {
            ctx.memories
                .iter()
                .map(|m| format!("[{}] {}", m.category, m.content))
                .collect::<Vec<_>>()
                .join("\n")
        },
    );

    let request = ClaudeRequest {
        model: MODEL.to_string(),
        max_tokens: 200,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: serde_json::Value::String("Greet me.".to_string()),
        }],
        system,
        tools: vec![],
    };

    let client = Client::new();
    let response = client
        .post(CLAUDE_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("greeting network error: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("greeting API error {status}: {body}"));
    }

    let claude_response: ClaudeResponse = response
        .json()
        .await
        .map_err(|e| format!("greeting parse error: {e}"))?;

    let text = claude_response
        .content
        .iter()
        .filter_map(|c| match c {
            ResponseContent::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    if text.trim().is_empty() {
        Err("greeting returned empty text".to_string())
    } else {
        Ok(text)
    }
}
```

- [ ] **Step 3: Compile check**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -15`
Expected: still the pre-existing caller errors for `send_with`; no new errors here.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/agent/engine.rs src-tauri/src/observability/event_bus.rs
git commit -m "feat(agent): add generate_greeting and JarvisGreeting event"
```

---

## Task 10: Thread context + greeting through chat and STT dispatch

**Files:**
- Modify: `src-tauri/src/commands/chat.rs`
- Modify: `src-tauri/src/voice/stt.rs`

- [ ] **Step 1: Build context helper**

This helper is used by both callers. Add a new function at the top of `src-tauri/src/agent/engine.rs`, just below the `AgentContext` impl:

```rust
/// Build an AgentContext by resolving the user's name from memory and
/// pulling top-K semantic matches (or pinned memories if no query is given).
pub fn build_context(
    store: &Mutex<MemoryStore>,
    embedder: &Embedder,
    tracker: &crate::session::SessionTracker,
    query: Option<&str>,
) -> AgentContext {
    let now = Local::now();
    let last_interaction = tracker.last_interaction();

    let (user_name, memories) = match store.lock() {
        Ok(guard) => {
            // Name lookup: first Profile memory whose content contains "name is".
            let user_name = guard
                .list(Some(MemoryCategory::Profile), 20)
                .unwrap_or_default()
                .into_iter()
                .find_map(|m| extract_name_from_profile(&m.content));

            let memories = match query {
                Some(q) => match embedder.embed(q) {
                    Ok(q_emb) => guard
                        .semantic_search(&q_emb, 6, 0.3)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(e, _)| e)
                        .collect(),
                    Err(_) => guard.search(q, 6).unwrap_or_default(),
                },
                None => guard.list_pinned(5).unwrap_or_default(),
            };

            (user_name, memories)
        }
        Err(_) => (None, vec![]),
    };

    AgentContext {
        now,
        user_name,
        last_interaction,
        memories,
    }
}

/// Extracts a name from a Profile memory like "User's name is Nakya" or
/// "name: Nakya". Returns None when nothing plausible is found.
fn extract_name_from_profile(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    for marker in ["name is ", "name: ", "i am ", "i'm "] {
        if let Some(idx) = lower.find(marker) {
            let start = idx + marker.len();
            let tail = &content[start..];
            let name = tail
                .split(|c: char| !(c.is_alphabetic() || c == '-' || c == '\''))
                .next()
                .unwrap_or("")
                .trim();
            if !name.is_empty() && name.len() <= 40 {
                return Some(name.to_string());
            }
        }
    }
    None
}
```

Also append these tests to the existing `tests` module at the bottom of engine.rs:

```rust
    #[test]
    fn extract_name_handles_common_phrasings() {
        assert_eq!(
            extract_name_from_profile("User's name is Nakya."),
            Some("Nakya".to_string())
        );
        assert_eq!(
            extract_name_from_profile("name: Alex"),
            Some("Alex".to_string())
        );
        assert_eq!(
            extract_name_from_profile("I'm Jordan, a software engineer"),
            Some("Jordan".to_string())
        );
        assert_eq!(extract_name_from_profile("Prefers espresso"), None);
    }
```

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib agent::engine::tests::extract_name`
Expected: test passes.

- [ ] **Step 2: Update `commands/chat.rs` to build context, handle greeting, call new signature**

Replace the body of `send_message` in `src-tauri/src/commands/chat.rs` with:

```rust
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

    // Greeting pre-step (if due and the message isn't itself a pure greeting).
    if tracker.should_greet() && !is_pure_greeting(&message) {
        let greeting_ctx = build_context(&*mem_store, &*embedder, &*tracker, None);
        let api_key = {
            let e = engine.lock().map_err(|e| e.to_string())?;
            e.api_key().to_string()
        };
        if !api_key.is_empty() {
            match generate_greeting(&api_key, &greeting_ctx).await {
                Ok(text) => {
                    tracker.mark_greeted();
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

    tracker.mark_interaction();

    let (api_key, history) = {
        let e = engine.lock().map_err(|e| e.to_string())?;
        (e.api_key().to_string(), e.history().to_vec())
    };

    if api_key.is_empty() {
        return Err("ANTHROPIC_API_KEY is not configured. Add it to .env or the environment and restart.".to_string());
    }

    let ctx = build_context(&*mem_store, &*embedder, &*tracker, Some(&message));
    let bus = event_bus.lock().map_err(|e| e.to_string())?.clone();

    let result = AgentEngine::send_with(
        &api_key,
        &history,
        &message,
        &ctx,
        &*mem_store,
        &*embedder,
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
```

Leave `check_health` unchanged.

- [ ] **Step 3: Update `voice/stt.rs` to use the new signature and greeting flow**

Edit `src-tauri/src/voice/stt.rs`. In the `dispatch_to_agent` function, replace the block from `tauri::async_runtime::spawn(async move {` onwards with:

```rust
    tauri::async_runtime::spawn(async move {
        use crate::agent::{build_context, generate_greeting};
        use crate::memory::{Embedder, MemoryStore};
        use crate::session::SessionTracker;

        let tracker_state = app.state::<SessionTracker>();
        let mem_state = app.state::<Mutex<MemoryStore>>();
        let embedder_state = app.state::<Embedder>();

        // Greeting pre-step
        if tracker_state.should_greet() && !is_pure_greeting(&trimmed) {
            let greeting_ctx = build_context(&*mem_state, &*embedder_state, &*tracker_state, None);
            let (api_key,) = {
                let engine = match app.state::<Mutex<AgentEngine>>().lock() {
                    Ok(e) => e,
                    Err(_) => return,
                };
                (engine.api_key().to_string(),)
            };
            if !api_key.is_empty() {
                match generate_greeting(&api_key, &greeting_ctx).await {
                    Ok(text) => {
                        tracker_state.mark_greeted();
                        if let Ok(bus) = event_bus.lock() {
                            bus.emit(JarvisEvent::JarvisGreeting { text: text.clone() });
                        }
                        if let Ok(speech) = app.state::<Mutex<SpeechManager>>().lock() {
                            let _ = speech.speak_async(&text, &app);
                        }
                        // Small buffer so the greeting finishes spinning up before
                        // the main response starts speaking.
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    }
                    Err(e) => eprintln!("[STT→Greet] greeting skipped: {e}"),
                }
            }
        }

        tracker_state.mark_interaction();

        let (api_key, history) = {
            let engine_state = app.state::<Mutex<AgentEngine>>();
            let engine = match engine_state.lock() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[STT→Agent] AgentEngine lock failed: {}", e);
                    return;
                }
            };
            (engine.api_key().to_string(), engine.history().to_vec())
        };

        if api_key.is_empty() {
            eprintln!("[STT→Agent] ERROR: ANTHROPIC_API_KEY is empty");
            if let Ok(bus) = event_bus.lock() {
                bus.emit(JarvisEvent::VoiceAgentError {
                    user_text: trimmed.clone(),
                    message: "ANTHROPIC_API_KEY is not configured. Add it to .env and restart.".to_string(),
                });
            }
            return;
        }

        let bus_snapshot = match event_bus.lock() {
            Ok(b) => b.clone(),
            Err(e) => {
                eprintln!("[STT→Agent] EventBus lock failed: {}", e);
                return;
            }
        };

        let ctx = build_context(&*mem_state, &*embedder_state, &*tracker_state, Some(&trimmed));

        let result = AgentEngine::send_with(
            &api_key,
            &history,
            &trimmed,
            &ctx,
            &*mem_state,
            &*embedder_state,
            &bus_snapshot,
        )
        .await;

        match result {
            Ok((response, new_history)) => {
                eprintln!("[STT→Agent] agent replied ({} chars)", response.len());

                if let Ok(mut engine) = app.state::<Mutex<AgentEngine>>().lock() {
                    engine.set_history(new_history);
                }

                if let Ok(bus) = event_bus.lock() {
                    bus.emit(JarvisEvent::VoiceAgentResponse {
                        user_text: trimmed.clone(),
                        assistant_text: response.clone(),
                    });
                }

                if let Ok(speech) = app.state::<Mutex<SpeechManager>>().lock() {
                    if let Err(e) = speech.speak_async(&response, &app) {
                        eprintln!("[STT→Agent] TTS failed: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[STT→Agent] agent error: {}", e);
                if let Ok(bus) = event_bus.lock() {
                    bus.emit(JarvisEvent::VoiceAgentError {
                        user_text: trimmed.clone(),
                        message: e,
                    });
                }
            }
        }
    });
}

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
```

Note the closing `}` of the outer `dispatch_to_agent` function — make sure the helper lives at module scope, not nested inside the spawn closure.

- [ ] **Step 4: Re-export helpers from `agent` mod**

Edit `src-tauri/src/agent/mod.rs`:

```rust
pub mod engine;

pub use engine::{AgentEngine, AgentContext, build_context, generate_greeting};
```

- [ ] **Step 5: Compile + run all tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src-tauri/Cargo.toml --lib -- --skip ignored`
Expected: clean build, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/agent/engine.rs src-tauri/src/agent/mod.rs src-tauri/src/commands/chat.rs src-tauri/src/voice/stt.rs
git commit -m "feat: thread AgentContext and greeting flow through chat and STT"
```

---

## Task 11: Frontend — handle `jarvis-greeting` event

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Add handler and register event**

Edit `src/App.tsx`. Add a handler near the other `useCallback` handlers (e.g. just after `handleVoiceAgentError`):

```tsx
  const handleJarvisGreeting = useCallback(
    (payload: { text: string }) => {
      uiLog('Voice', `jarvis-greeting: len=${payload.text.length}`);
      const now = Date.now();
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: payload.text,
        timestamp: now,
      });
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'JarvisGreeted',
        description: payload.text.slice(0, 160),
        timestamp: now,
      });
    },
    [addMessage, addActionEvent]
  );
```

Then register it alongside the other `useTauriEvent` calls:

```tsx
  useTauriEvent('jarvis-greeting', handleJarvisGreeting);
```

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: exit 0 (no output).

- [ ] **Step 3: Commit**

```bash
git add src/App.tsx
git commit -m "feat(ui): render JARVIS standalone greetings in chat and timeline"
```

---

## Task 12: Smoke test + final verify

**Files:**
- None modified. Manual test only.

- [ ] **Step 1: Build and launch**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && npm run tauri:dev`
Expected: app launches. First run downloads the embedding model (~90MB, one-time).

- [ ] **Step 2: Verify greeting on first activation**

- Wait ~2 seconds after the window appears; clap once to activate, then say "Hello".
- Expected: JARVIS delivers a contextual greeting (e.g. "Good evening, Sir. I don't believe I have your name yet — shall I remember it for you?"). The greeting appears in chat + timeline.
- Backend log should show `[STT→Greet]` lines and `jarvis-greeting` event.

- [ ] **Step 3: Verify save_memory wires to DB**

- Say: "Remember my name is Nakya."
- Then: "Remember that I prefer espresso over coffee."
- After each, check the Memory panel in the dashboard — it should show a `MemorySaved` entry.

- [ ] **Step 4: Verify recall via semantic search**

- Say: "What do I drink?"
- Expected: JARVIS references the espresso preference. (Optional: inspect backend log for `recall_memory` tool invocation.)

- [ ] **Step 5: Verify greeting re-fires after long idle**

- Hard path: change `IDLE_THRESHOLD` in `session/tracker.rs` to `Duration::from_secs(60)` temporarily, rebuild, wait 61 seconds silent, say anything.
- Expected: JARVIS greets again before answering. Revert the constant once verified.

- [ ] **Step 6: Final compile clean**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check --manifest-path src-tauri/Cargo.toml && npx tsc --noEmit`
Expected: both pass silently.

- [ ] **Step 7: Commit any fix-ups, push**

```bash
git status
# If clean, you're done. Otherwise commit the fix-ups first:
# git add <files> && git commit -m "fix: smoke-test follow-ups"
git push origin main
```

---

## Self-Review Results

**Spec coverage:**
- Persona prompt → Task 6
- Explicit `save_memory` wiring → Task 7
- `recall_memory` tool → Task 8
- Embedding column + `semantic_search` + `list_pinned` → Tasks 2, 3
- Semantic retrieval threshold (sim > 0.3) → applied in Tasks 8, 10
- Local ONNX via `fastembed` → Task 1
- Graceful failure fallback to LIKE → Task 8 (recall), Task 10 (build_context), Task 1 (Embedder returns Err)
- `SessionTracker` + >30min threshold → Task 4
- Greeting flow (first-of-session + long idle) → Tasks 9, 10
- Pure-greeting detection regex → implemented as a keyword allowlist in `is_pure_greeting` in Tasks 10 (simpler than regex; equivalent semantics)
- `JarvisGreeting` event → Task 9
- Frontend renders greeting → Task 11
- Testing plan from spec → inline tests in Tasks 1, 2, 3, 4, 6, 10; manual smoke in Task 12

**No gaps found.**

**Placeholder scan:** none.

**Type consistency:**
- `AgentContext` fields match across Tasks 6, 9, 10.
- `build_context`, `generate_greeting`, `send_with` signatures consistent in every caller.
- `MemoryCategory`, `MemoryEntry` used identically.
- `SessionTracker` methods (`mark_interaction`, `mark_greeted`, `should_greet`, `last_interaction`) used consistently.

Plan is ready.
