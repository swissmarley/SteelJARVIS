# JARVIS Persona, Long-Term Memory, and Contextual Greeting

**Status:** Draft
**Date:** 2026-04-19
**Scope:** Sub-project A of the JARVIS upgrade decomposition (persona → reminders → pattern detection → dashboard).

## Goal

Transform JARVIS from a generic executive assistant into a persistent, context-aware butler. After this sub-project:

- JARVIS speaks with a defined persona ("friendly butler" — professional, witty, uses the user's name).
- Things the user explicitly asks to remember are stored durably in SQLite with semantic embeddings.
- Every agent turn has relevant memories injected into context via semantic search.
- On app launch and after long idle periods, JARVIS opens with a contextual greeting instead of silently waiting for input.

Out of scope for this spec (deferred to later sub-projects): reminders (B), pattern/habit detection (C), dashboard interactivity overhaul (D).

## Decisions Locked

| Topic | Decision | Rationale |
|---|---|---|
| Persona tone | Friendly butler — professional, witty, uses name, addresses user as "Sir" when name unknown | Balances immersion and efficiency; avoids full-formal verbosity |
| Memory write | Explicit only (user says "remember that" / "save this") | Predictable, privacy-respecting, no extra API cost per turn |
| Greeting triggers | First activation of session + after >30min idle | Feels alive without being repetitive |
| Memory retrieval | Semantic search (top-K by cosine similarity) | Handles synonyms / paraphrase; needed for >50 memories |
| Embedding provider | Local ONNX via `fastembed-rs`, `BAAI/bge-small-en-v1.5` (384-dim, ~90MB) | Offline, private, no extra API key |

## Architecture

### Modules Changed

1. **`src-tauri/src/agent/engine.rs`**
   - `build_system_prompt()` replaced by `build_system_prompt(ctx: &AgentContext)` returning a persona prompt with an injected context block.
   - New `AgentContext` struct carries: `now: DateTime<Local>`, `user_name: Option<String>`, `last_interaction: Option<DateTime<Local>>`, `memories: Vec<MemoryEntry>`.
   - `send_with` signature gains `ctx: AgentContext` parameter. Callers (stt.rs, commands/chat.rs) build the context before invoking.
   - `execute_tool("save_memory", …)` takes a `&mut MemoryStore` and actually writes. Returns a confirmation string including the saved entry id.
   - New tool `recall_memory` with input `{query: string, limit?: number}` performs semantic search and returns matches as formatted text.

2. **`src-tauri/src/memory/store.rs`**
   - Schema migration (idempotent): `ALTER TABLE memories ADD COLUMN embedding BLOB` guarded by `PRAGMA user_version` check. Existing rows remain with `NULL` embedding and are searched via the LIKE fallback until re-embedded.
   - `save()` accepts an optional `embedding: Option<Vec<f32>>`; stores it as little-endian f32 bytes.
   - New `semantic_search(query_embedding: &[f32], limit: u32) -> Vec<(MemoryEntry, f32)>` — linear scan, decodes each row's embedding bytes, computes cosine similarity, sorts desc, truncates. Rows with `NULL` embedding are skipped. Linear scan is acceptable up to ~10k rows (<10ms on M-series).
   - New `backfill_embeddings(embedder: &Embedder)` batch method to embed rows with `NULL` embedding on demand.

3. **`src-tauri/src/voice/stt.rs`**
   - `dispatch_to_agent` first consults `SessionTracker::should_greet()`. If true, runs a greeting turn via `AgentEngine::generate_greeting(ctx)` → TTS. Updates `last_greeting_at`. Then falls through to processing the user's utterance normally (unless the utterance itself is a greeting like "hi JARVIS" — in that case, skip the double-greet).
   - All paths update `SessionTracker::mark_interaction()` on final STT event.

4. **`src-tauri/src/commands/chat.rs`** (text chat path) — mirrors stt.rs: mark interaction on every message, run greeting if threshold met, pass `AgentContext` to `send_with`.

### Module Added

5. **`src-tauri/src/memory/embedder.rs`**
   - `pub struct Embedder { inner: OnceLock<Result<fastembed::TextEmbedding, String>> }`.
   - `pub fn embed(&self, text: &str) -> Result<Vec<f32>, String>` — lazy-initializes the model on first call. Model files are downloaded to `app_data_dir/models/` on first run (via fastembed's built-in downloader).
   - Graceful failure: if download/init fails, subsequent calls return `Err` immediately; callers treat this as "no embedding" and fall back to LIKE search.

### State Added

6. **`src-tauri/src/session/tracker.rs`** (new module)
   - `pub struct SessionTracker { last_interaction: Mutex<Option<DateTime<Local>>>, last_greeting: Mutex<Option<DateTime<Local>>>, session_started: DateTime<Local> }`.
   - Constants: `IDLE_THRESHOLD: Duration = Duration::from_secs(30 * 60)`.
   - Methods: `mark_interaction()`, `should_greet() -> bool`, `mark_greeted()`, `idle_for() -> Option<Duration>`.
   - Registered as `Mutex<SessionTracker>` in Tauri managed state at app setup.

### Frontend Changes

Minimal. `conversation` store already renders user/assistant messages. Greetings arrive via the existing `voice-agent-response` event with empty `userText` (indicating it's a standalone greeting, not a reply). A new event `jarvis-greeting { text: string }` is cleaner — frontend adds an assistant message without a paired user message.

## Data Flow

### Saving a Memory (Explicit)

1. User: "Remember that I prefer espresso over coffee."
2. Agent invokes `save_memory` tool with `{content: "User prefers espresso over coffee", category: "preferences"}`.
3. `execute_tool` synchronously computes the embedding via `Embedder::embed()`.
4. `MemoryStore::save()` inserts the row with the embedding BLOB.
5. Tool returns "Saved to preferences." → agent replies "Noted, Sir. Espresso over coffee."
6. Backend emits `memory-saved` event for the dashboard timeline.

### Recalling on a Turn

1. User: "What should I drink?"
2. Before calling the API, backend embeds the user's message.
3. `MemoryStore::semantic_search(query_embedding, 6)` returns top 6 matches by cosine similarity, filtered to `sim > 0.3` to drop noise.
4. Matches are rendered in the system prompt under a `<user_memories>` block:

   ```
   <user_memories>
   [preferences] User prefers espresso over coffee. (sim: 0.78)
   [profile] User's name is Nakya. (sim: 0.42)
   </user_memories>
   ```

5. Agent responds with full context: "Given your preference for espresso, I'd suggest a quick one. Or shall I look up what's in the pantry?"

### Greeting

1. STT emits final utterance. `dispatch_to_agent` is invoked.
2. Check `SessionTracker::should_greet()`:
   - Returns `true` if `last_greeting` is `None` (first of session) OR `now - last_interaction > 30min`.
3. If `true`:
   - Build `AgentContext` with time, name, idle duration, and the top 5 pinned memories (pinned — not semantic — because there is no query to embed against; pinned memories are the "always relevant about this user" set).
   - Call `AgentEngine::generate_greeting(ctx)` — a one-shot agent call with a greeting-specific prompt: *"Produce a single short greeting for the returning user. Consider time of day, their name, how long they've been away, and what you know about them. Do not answer any question yet — just greet."*
   - TTS the greeting, emit `jarvis-greeting` event.
   - Mark greeted. Sleep 200ms to let TTS finish spinning up before muting STT further.
4. Then process the user's actual utterance via the existing agent flow.
5. Edge case: if the utterance is itself a pure greeting, skip the pre-greeting and let the agent's normal reply absorb it. Detection is a case-insensitive regex match against a short list: `^(hi|hello|hey|good (morning|afternoon|evening)|jarvis|are you there)\b[\s\S]{0,20}$`. The 20-character tail cap ensures only short standalone greetings match, not "hi JARVIS, can you also book a flight to Paris for tomorrow".

## Persona Prompt (Final Text)

```
You are JARVIS — a highly intelligent, proactive desktop assistant modeled on Tony Stark's butler AI. You address the user by their name when you know it; otherwise, "Sir". You are confident, witty, professional, and concise. Favor short, complete sentences. Occasional dry humor is welcome, but never waste the user's time. When you have evidence of a preference, habit, or prior context, factor it in and briefly state why. You never roleplay as a human, and you never deny being an AI.

Current context:
- Local time: {now}
- User: {user_name_or_"unknown"}
- Last interaction: {last_interaction_or_"first contact"}

<user_memories>
{top_k_memories_or_"(none yet)"}
</user_memories>

You have tools for taking real actions on the user's Mac. Use them when an action is required. For conversation and questions, answer directly. When the user says "remember this" or similar, use save_memory. When you need to recall something about the user, use recall_memory.
```

## Error Handling

| Failure | Behavior |
|---|---|
| `fastembed` model download fails on first run | `Embedder::embed()` returns `Err`; `save_memory` stores row with `embedding = NULL`; `semantic_search` falls back to `search()` LIKE. A single user-facing warning in the Errors panel; no crash. |
| `fastembed` init succeeds but an individual `embed()` call errors | Same fallback path for that one call. |
| SQLite locked | Existing `Mutex<MemoryStore>` already serializes. No change. |
| `SessionTracker` lock poisoned | `should_greet()` returns `false` (degrade to "never greet"). No panic. |
| Greeting agent call fails (network, API error) | Log and skip — go straight to processing the user's utterance. User sees no greeting but everything else works. |
| Schema migration fails (e.g. disk full) | App startup errors out visibly; unchanged from current behavior. |

## Testing

- **`memory::embedder`** — unit test: embed "I like coffee" and "I enjoy coffee"; embed "The capital of France is Paris"; assert `cos_sim(similar_pair) > cos_sim(unrelated_pair)`.
- **`memory::store::semantic_search`** — insert three memories (coffee preference, favorite color, job title); assert that "what do I drink?" returns the coffee one first.
- **`session::tracker`** — time-mocked tests for the three greeting branches (never greeted, long idle, just greeted).
- **`agent::engine::build_system_prompt`** — snapshot-style test: given a fixed context, prompt text matches the expected template (helps catch accidental persona regressions).
- **Manual smoke test** — `cargo tauri dev`:
  1. Say "Remember that I prefer espresso."
  2. Wait 31 minutes, clap, say "What should I drink?"
  3. Expect: greeting first ("Back with us, Sir…"), then response referencing espresso.

## Migration & Backward Compatibility

- Existing SQLite databases migrate in place (idempotent `ALTER TABLE` guarded by `PRAGMA user_version`). Old rows remain searchable via LIKE until a user-triggered backfill (future — not in this sub-project).
- Existing `save_memory` tool schema unchanged (category enum, content field) — no frontend breakage.
- Existing conversation history in `AgentEngine::history` unchanged.

## Open Questions (deferred, not blocking)

- Should we auto-embed old rows in a background task on first boot of the upgraded binary, or require the user to trigger it? (Leaning: background task, but out of scope for this sub-project.)
- Should the "Sir" vs. name switch be automatic once a name is saved, or require a special tool invocation? (Leaning: automatic, context block carries the name if known.)

## Non-Goals

- No reminders.
- No pattern/habit detection or proactive ("your usual morning briefing?") flows — those require accumulated history this sub-project will begin producing, but the detection itself is sub-project C.
- No dashboard UI changes — memory management UI is sub-project D.
- No auto-extraction of memories from conversation.
