# JARVIS — Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────┐
│                    PRESENTATION LAYER                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  │
│  │  Chat UI  │ │Dashboard │ │ Settings │ │ Mem Review │  │
│  └─────┬────┘ └─────┬────┘ └─────┬────┘ └──────┬─────┘  │
│        └─────────────┼───────────┼──────────────┘        │
│                      │           │                        │
│              ┌───────┴───────────┴───────┐               │
│              │    Frontend State Store   │               │
│              │  (Zustand + Immer)        │               │
│              └───────────┬───────────────┘               │
└──────────────────────────┼──────────────────────────────┘
                           │ Tauri IPC (invoke / events)
┌──────────────────────────┼──────────────────────────────┐
│                    RUST BACKEND                          │
│              ┌───────────┴───────────┐                 │
│              │     Command Router     │                 │
│              └───┬────┬────┬────┬─────┘                 │
│     ┌────────────┼────┼────┼────────────────┐          │
│  ┌──┴───┐  ┌────┴──┐ │ ┌──┴────┐  ┌────────┴──┐      │
│  │Agent │  │Memory │ │ │Desktop│  │  Voice/   │      │
│  │Engine│  │ Store │ │ │Control│  │  Audio    │      │
│  └──┬───┘  └───┬───┘ │ └──┬────┘  └─────┬─────┘      │
│     │          │     │    │             │             │
│  ┌──┴──────────┴─────┴────┴─────────────┴──────┐      │
│  │              Event Bus (tokio::broadcast)     │      │
│  └──────────────────┬──────────────────────────┘      │
│                     │                                  │
│  ┌──────────────────┴──────────────────────────┐      │
│  │           Permission Manager                 │      │
│  └─────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────┘
         │                    │
    ┌────┴─────┐        ┌────┴─────┐
    │ Claude   │        │ SQLite   │
    │ API      │        │ +vec     │
    └──────────┘        └──────────┘
```

## Layer Details

### 1. Presentation Layer (React + TypeScript)

**Framework**: React 18+ with TypeScript
**Build**: Vite
**State**: Zustand with Immer for immutable updates
**Styling**: CSS Modules + CSS custom properties (no heavy framework)

**Components**:
- `AppShell` — main window container, layout manager
- `ChatPanel` — conversation input/output, streaming text
- `DashboardPanel` — collapsible dashboard sections
- `VoiceIndicator` — microphone state, listening visualization
- `MemoryPanel` — memory search, review, edit, pin/delete
- `SettingsPanel` — API keys, voice config, permissions, privacy
- `PermissionPrompt` — modal for action approvals
- `TaskTimeline` — execution timeline with status indicators
- `StatusIndicator` — execution mode, provider status, errors

**State Structure** (Zustand stores):
```typescript
interface ConversationStore {
  messages: Message[];
  isStreaming: boolean;
  currentResponse: string;
}

interface DashboardStore {
  executionMode: ExecutionMode;
  activeGoal: string | null;
  taskQueue: Task[];
  actionTimeline: ActionEvent[];
  toolsInUse: ToolInfo[];
  memoryEvents: MemoryEvent[];
  permissionRequests: PermissionRequest[];
  errors: ErrorEvent[];
  voiceState: VoiceState;
  systemHealth: SystemHealth;
}
```

### 2. Interaction Layer (Rust + Frontend)

**Text Pipeline**:
1. User types in chat input → frontend captures
2. Frontend calls `tauri::invoke('send_message', { text })` → Rust backend
3. Backend routes to agent engine → Claude API call
4. Response streams back via Tauri events

**Voice Pipeline**:
1. Microphone capture (CPAL + Apple Speech framework)
2. STT: Apple Speech framework (`SFSpeechRecognizer`)
3. Text sent to agent engine (same as text pipeline)
4. TTS: `AVSpeechSynthesizer` for response

**Activation Manager**:
- Unified `ActivationManager` that manages multiple activation sources
- Priority: clap > hotkey > push-to-talk > wake word > UI click
- Cooldown logic prevents rapid re-activation
- Each activation source reports events to the event bus

**Clap Detection Module**:
- Audio captured via CPAL (cross-platform audio)
- Transient detection: short-time energy + spectral analysis
- Threshold tuning: configurable sensitivity (1-10 scale)
- Calibration: user claps 3 times, system calibrates thresholds
- Cooldown: minimum 2 seconds between activations
- False positive mitigation: bandpass filter focused on clap frequency range (1-4kHz), temporal pattern matching

### 3. Agent Layer (Rust)

**Components**:

```rust
struct AgentEngine {
    planner: Planner,
    executor: Executor,
    context: ContextManager,
    approval: ApprovalManager,
    tool_registry: ToolRegistry,
}

struct Planner {
    // Takes user intent → produces step-by-step plan
    // Uses Claude API with planning prompt
}

struct Executor {
    // Executes plan steps one at a time
    // Reports progress to event bus
    // Handles retry/fallback on failure
}

struct ContextManager {
    // Manages conversation history
    // Injects relevant memories
    // Tracks token usage
}

struct ApprovalManager {
    // Checks action against permission policies
    // Returns approval/denial/ask-user
}
```

**Agent Loop**:
```
User Input → Intent Parse → Plan Generation → [Approval Check] → Step Execution → Progress Report → Next Step → ... → Completion Summary
```

**Tool Registry**:
```rust
trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    fn execute(&self, params: serde_json::Value) -> Result<ToolOutput, ToolError>;
    fn requires_approval(&self) -> bool;
}
```

### 4. Memory Layer (Rust + SQLite)

**Schema**:
```sql
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    category TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    source TEXT NOT NULL,  -- 'explicit' | 'auto_extracted' | 'tool_result'
    privacy_label TEXT DEFAULT 'normal',  -- 'normal' | 'sensitive' | 'private'
    pinned BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_accessed TIMESTAMP,
    access_count INTEGER DEFAULT 0,
    conversation_id TEXT,
    metadata JSON
);

CREATE TABLE memory_embeddings (
    memory_id TEXT PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
    embedding BLOB NOT NULL  -- vector stored as blob for sqlite-vec
);

CREATE TABLE memory_relations (
    from_id TEXT REFERENCES memories(id),
    to_id TEXT REFERENCES memories(id),
    relation_type TEXT,
    PRIMARY KEY (from_id, to_id, relation_type)
);

CREATE VIRTUAL TABLE memory_vec USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding float[768]
);
```

**Memory Operations** (Rust module):
- `save_memory(content, category, source)` → insert + embed
- `search_memories(query, limit)` → embed query → cosine similarity via sqlite-vec
- `rank_memories(query)` → combine semantic similarity + recency + importance
- `compact_memories()` → summarize old/low-access memories
- `resolve_conflicts()` → flag contradictory memories

**Embedding**:
- Use a small local embedding model (e.g., `all-MiniLM-L6-v2` via ONNX Runtime)
- Or call an embedding API if local model is too heavy for MVP
- Embedding stored as f32 vector blob in sqlite-vec

### 5. Integration Layer (Rust)

**Desktop Provider** (macOS):
```rust
#[async_trait]
trait DesktopProvider: Send + Sync {
    async fn launch_app(&self, name: &str) -> Result<()>;
    async fn focus_app(&self, name: &str) -> Result<()>;
    async fn quit_app(&self, name: &str) -> Result<()>;
    async fn open_url(&self, url: &str) -> Result<()>;
    async fn open_file(&self, path: &str) -> Result<()>;
    async fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    async fn move_window(&self, window_id: &str, x: i32, y: i32) -> Result<()>;
    async fn resize_window(&self, window_id: &str, w: u32, h: u32) -> Result<()>;
    async fn trigger_menu_item(&self, app: &str, menu: &str, item: &str) -> Result<()>;
    async fn send_keystroke(&self, keys: &str) -> Result<()>;
}
```

**macOS Implementation**:
- App launching: `NSWorkspace::shared().open_application()`
- AppleScript execution: `osascript` command via `std::process::Command`
- Accessibility API: via Swift bridge or `core-foundation` bindings
- Window management: AXUIElement API

**Web Search Provider**:
```rust
#[async_trait]
trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str, limit: u32) -> Result<Vec<SearchResult>>;
}
```

**Speech Provider**:
```rust
#[async_trait]
trait SpeechProvider: Send + Sync {
    async fn transcribe(&self, audio: &[u8]) -> Result<String>;
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>>;
    async fn set_voice(&self, voice_id: &str) -> Result<()>;
    async fn list_voices(&self) -> Result<Vec<VoiceInfo>>;
}
```

### 6. Observability Layer (Rust)

**Event Bus**:
```rust
#[derive(Clone, Debug, Serialize)]
enum JarvisEvent {
    StateChanged { from: ExecutionMode, to: ExecutionMode },
    GoalSet { goal: String },
    PlanCreated { steps: Vec<String> },
    StepStarted { index: usize, description: String },
    StepCompleted { index: usize, result: String },
    StepFailed { index: usize, error: String },
    ToolInvoked { tool: String, params: Value },
    ToolCompleted { tool: String, result: Value },
    MemorySaved { id: String, category: String, preview: String },
    MemoryRetrieved { id: String, query: String },
    PermissionRequested { action: String, details: String },
    PermissionGranted { action: String },
    PermissionDenied { action: String },
    VoiceStateChanged { state: VoiceState },
    ActivationTriggered { source: String },
    ClapDetected { confidence: f64 },
    Error { source: String, message: String },
    ProviderStatusChanged { provider: String, status: String },
}
```

All events are:
- Broadcast via `tokio::sync::broadcast`
- Persisted to SQLite action log
- Forwarded to frontend via Tauri events
- Available for the dashboard in real-time

### 7. Security / Permission Layer (Rust)

**Permission Categories**:
```rust
enum PermissionCategory {
    AppLaunch,       // launching apps
    AppControl,      // menu clicks, keystrokes, quit
    FileAccess,      // opening files/folders
    WindowControl,   // move, resize, arrange
    WebSearch,       // external API calls
    MemoryWrite,     // saving memories
    Clipboard,       // clipboard operations
    NetworkAccess,   // any outbound network
}
```

**Permission Levels**:
```rust
enum PermissionLevel {
    Allowed,                    // pre-approved by user
    AskOnce,                    // ask once, remember decision
    AskAlways,                  // always ask
    Denied,                     // always deny
}
```

**Storage**: Permissions persisted in SQLite config table.

**API Key Storage**: macOS Keychain via `security` command or `keychain-services` crate.

## Data Flow

### User Request Flow
```
1. User types "Open Spotify and arrange it on the left half of my screen"
2. Frontend → Tauri IPC → Rust Command Router
3. Router → AgentEngine
4. AgentEngine → Planner (Claude API call with tools available)
5. Planner returns: [Step1: launch_app("Spotify"), Step2: arrange_window("Spotify", "left")]
6. Events emitted: PlanCreated, StepStarted
7. Executor → ToolRegistry.get("launch_app") → DesktopProvider.launch_app("Spotify")
8. PermissionManager checks: AppLaunch → AskOnce (first time)
9. Frontend shows permission prompt → User approves
10. Step1 executes → StepCompleted event
11. Step2 executes → DesktopProvider.arrange_window(...)
12. All steps done → completion summary → Dashboard updated
```

### Memory Flow
```
1. User: "Remember that I prefer dark mode in all apps"
2. AgentEngine detects memory intent → MemoryStore.save_memory()
3. MemoryStore: embed content, insert to SQLite + sqlite-vec
4. Events: MemorySaved emitted → Dashboard shows new memory
5. Later: User: "What are my preferences?"
6. MemoryStore.search("user preferences") → semantic + metadata query
7. Events: MemoryRetrieved → results injected into agent context
8. Agent responds with recalled preferences
```

## File Structure

```
SteelJARVIS/
├── SPEC.md
├── ARCHITECTURE.md
├── ROADMAP.md
├── RISKS.md
├── MILESTONES.md
├── CLAUDE.md
├── README.md
├── .env.example
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── src/                          # React frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── styles/
│   │   ├── global.css
│   │   ├── variables.css
│   │   └── components/
│   ├── components/
│   │   ├── AppShell/
│   │   ├── ChatPanel/
│   │   ├── Dashboard/
│   │   ├── VoiceIndicator/
│   │   ├── MemoryPanel/
│   │   ├── SettingsPanel/
│   │   ├── PermissionPrompt/
│   │   └── common/
│   ├── stores/
│   │   ├── conversation.ts
│   │   ├── dashboard.ts
│   │   ├── memory.ts
│   │   ├── voice.ts
│   │   └── settings.ts
│   ├── hooks/
│   │   ├── useTauriEvent.ts
│   │   ├── useVoice.ts
│   │   └── useMemory.ts
│   └── types/
│       ├── events.ts
│       ├── messages.ts
│       ├── memory.ts
│       └── tools.ts
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── chat.rs
│   │   │   ├── memory.rs
│   │   │   ├── desktop.rs
│   │   │   ├── voice.rs
│   │   │   ├── search.rs
│   │   │   └── settings.rs
│   │   ├── agent/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs
│   │   │   ├── planner.rs
│   │   │   ├── executor.rs
│   │   │   └── context.rs
│   │   ├── memory/
│   │   │   ├── mod.rs
│   │   │   ├── store.rs
│   │   │   ├── embeddings.rs
│   │   │   └── schema.rs
│   │   ├── desktop/
│   │   │   ├── mod.rs
│   │   │   ├── provider.rs
│   │   │   └── macos.rs
│   │   ├── voice/
│   │   │   ├── mod.rs
│   │   │   ├── speech.rs
│   │   │   ├── clap_detector.rs
│   │   │   └── activation.rs
│   │   ├── search/
│   │   │   ├── mod.rs
│   │   │   └── provider.rs
│   │   ├── permissions/
│   │   │   ├── mod.rs
│   │   │   └── manager.rs
│   │   ├── observability/
│   │   │   ├── mod.rs
│   │   │   └── event_bus.rs
│   │   └── config/
│   │       ├── mod.rs
│   │       └── settings.rs
│   └── migrations/
│       └── 001_initial.sql
└── tests/
    ├── frontend/
    └── rust/
```