# JARVIS — Milestones

## M1: Project Scaffold + Core Shell
**Status**: Not Started
**Goal**: Launchable Tauri app with dark HUD shell

**Deliverables**:
- [ ] `cargo tauri dev` launches a window
- [ ] React + TypeScript + Vite pipeline works
- [ ] Folder structure matches ARCHITECTURE.md
- [ ] Zustand stores scaffolded
- [ ] Basic dark theme applied
- [ ] App shell layout renders (sidebar + main area)

**Exit Criteria**: Window launches, shows placeholder layout, hot reload works

---

## M2: HUD Dashboard UI
**Status**: Not Started
**Goal**: Full dashboard UI with placeholder data

**Deliverables**:
- [ ] Dashboard panels: Status, Tasks, Timeline, Tools, Memory, Permissions, Logs
- [ ] Collapsible panel behavior
- [ ] CSS variables for JARVIS HUD theme
- [ ] Execution mode indicator
- [ ] Voice state indicator (static)
- [ ] Memory panel layout (static)
- [ ] Permission prompt modal layout
- [ ] Smooth transitions on state changes

**Exit Criteria**: Dashboard renders with mock data, all panels collapse/expand, theme looks premium

---

## M3: Chat Interface + Agent Connection
**Status**: Not Started
**Goal**: User can chat with Claude via the JARVIS UI

**Deliverables**:
- [ ] Chat input component (text)
- [ ] Message list with assistant/user distinction
- [ ] Streaming text rendering
- [ ] Rust backend: `send_message` command
- [ ] Claude API client with SSE streaming
- [ ] Conversation store
- [ ] API key configuration (env var or settings)
- [ ] Error handling and display
- [ ] Greeting message on startup

**Exit Criteria**: User types message, sees Claude response stream in, can have multi-turn conversation

---

## M4: Agent Engine + Tools
**Status**: Not Started
**Goal**: JARVIS can plan and execute multi-step tasks using tools

**Deliverables**:
- [ ] Tool trait and registry
- [ ] Planner: intent → plan via Claude
- [ ] Executor: step-by-step with progress events
- [ ] Context manager: history + memory injection
- [ ] Tool: `open_app` (macOS)
- [ ] Tool: `open_url` / `open_file`
- [ ] Tool: `clipboard_read` / `clipboard_write`
- [ ] Permission manager (category-based)
- [ ] Permission prompt UI connected
- [ ] Dashboard: tool activity, plan steps

**Exit Criteria**: "Open Safari" → plan shown → permission prompt → Safari opens. "Copy this to clipboard" → permission → clipboard written.

---

## M5: Memory System
**Status**: Not Started
**Goal**: Long-term memory with semantic search

**Deliverables**:
- [ ] SQLite + migrations initialization
- [ ] Memory schema (memories, embeddings, relations)
- [ ] sqlite-vec integration
- [ ] Embedding generation (local or API)
- [ ] Memory save (explicit + auto-candidate)
- [ ] Memory search (semantic + metadata)
- [ ] Memory ranking (recency + importance)
- [ ] Memory CRUD API (Rust commands)
- [ ] Frontend: memory panel (search, edit, pin, delete)
- [ ] Dashboard: memory events

**Exit Criteria**: "Remember I prefer dark mode" → saved → "What are my preferences?" → recalled correctly

---

## M6: Voice Pipeline
**Status**: Not Started
**Goal**: Voice input and output working

**Deliverables**:
- [ ] Microphone capture
- [ ] Apple Speech framework STT
- [ ] AVSpeechSynthesizer TTS
- [ ] Voice state machine
- [ ] Push-to-talk activation
- [ ] Global hotkey activation
- [ ] Voice indicator UI wired
- [ ] Settings: voice, device, speed

**Exit Criteria**: Push hotkey → speak → transcription → agent response → TTS speaks it back

---

## M7: Clap Detection + Activation
**Status**: Not Started
**Goal**: Clap activation as primary/experimental method

**Deliverables**:
- [ ] Audio stream analysis module
- [ ] Transient detection (energy + spectral)
- [ ] Calibration UI (3-clap calibration)
- [ ] Sensitivity controls
- [ ] Cooldown logic
- [ ] False-positive mitigation
- [ ] Activation manager (unified priority)
- [ ] Dashboard: activation events

**Exit Criteria**: User enables clap → calibrates → claps → JARVIS activates → listens → responds

---

## M8: Desktop Control (Full)
**Status**: Not Started
**Goal**: Complete macOS desktop control

**Deliverables**:
- [ ] App launching (NSWorkspace + AppleScript)
- [ ] App focus/switching
- [ ] App quitting (with confirmation)
- [ ] Window listing (AXUIElement)
- [ ] Window move/resize
- [ ] Window arrangement commands
- [ ] Menu item triggering
- [ ] Keystroke sending
- [ ] Permission prompts per category
- [ ] Dashboard: desktop control actions

**Exit Criteria**: "Open Safari and put it on the left half" → Safari opens + window arranged

---

## M9: Web Search + Recruiting Tools
**Status**: Not Started
**Goal**: Web search and lightweight recruiting capabilities

**Deliverables**:
- [ ] Web search provider (API)
- [ ] Search result formatting
- [ ] Answer synthesis
- [ ] Recruiting: draft JD
- [ ] Recruiting: draft outreach
- [ ] Recruiting: draft interview questions
- [ ] Recruiting: resume parsing
- [ ] Recruiting: candidate comparison
- [ ] Tool registration
- [ ] Dashboard: search + tool activity

**Exit Criteria**: "Search for Rust Tauri tutorials" → results with sources. "Draft a JD for a senior backend engineer" → structured JD output

---

## M10: Settings + System Tray
**Status**: Not Started
**Goal**: Complete settings and tray behavior

**Deliverables**:
- [ ] Settings panel UI
- [ ] API key management
- [ ] Voice configuration
- [ ] Permission review/edit
- [ ] Privacy settings
- [ ] Activation method toggles
- [ ] System tray icon + menu
- [ ] Minimize to tray

**Exit Criteria**: Settings persist across restarts, tray shows status, minimize works

---

## M11: Observability + Polish
**Status**: Not Started
**Goal**: Full real-time observability and polished UX

**Deliverables**:
- [ ] All events wired to dashboard
- [ ] Action log persistence
- [ ] Error tracking
- [ ] Performance metrics
- [ ] Smooth CSS transitions everywhere
- [ ] Loading states for async ops
- [ ] Graceful offline handling
- [ ] Debug mode

**Exit Criteria**: Every action produces visible dashboard events. No jarring state transitions.

---

## M12: Verification + Documentation
**Status**: Not Started
**Goal**: Shippable, documented, verified MVP

**Deliverables**:
- [ ] TypeScript type check passes
- [ ] ESLint clean
- [ ] Cargo clippy clean
- [ ] `cargo tauri build` succeeds
- [ ] README with setup/run/test
- [ ] .env.example
- [ ] Known limitations documented
- [ ] Mocked vs real features documented
- [ ] Experimental features documented

**Exit Criteria**: Fresh clone → install → build → run works end-to-end