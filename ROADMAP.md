# JARVIS — Implementation Roadmap

## Phase 1: Foundation (Week 1-2)

### M1: Project Scaffold + Core Shell
- Initialize Tauri + React + TypeScript project
- Set up Vite build pipeline
- Create folder structure per ARCHITECTURE.md
- Basic window with dark theme, app shell layout
- Minimal Tauri IPC: ping/pong command
- Zustand store scaffolding
- Verify: `cargo tauri dev` launches window

### M2: HUD Dashboard UI
- Dashboard layout with collapsible panels
- CSS variables for JARVIS HUD theme (colors, glass, spacing)
- Status indicator component (execution modes)
- Task timeline component (static, wired later)
- Voice state indicator (static, wired later)
- Memory panel (static, wired later)
- Permission prompt modal (static, wired later)
- Responsive layout for main window
- Verify: Dashboard renders with placeholder data, panels collapse/expand

### M3: Chat Interface + Agent Connection
- Chat input component (text only)
- Message list with streaming text rendering
- Rust backend: `send_message` command → Claude API
- Claude API client in Rust (reqwest + streaming SSE)
- Conversation store (messages, streaming state)
- Basic agent loop: user message → Claude response → display
- Error handling for API failures
- API key configuration (env var or settings file)
- Verify: User can type, get Claude response, see streaming text

## Phase 2: Core Systems (Week 3-4)

### M4: Agent Engine + Tools
- Tool trait and registry
- Planner: Claude API call with tool definitions
- Executor: step-by-step execution with progress events
- Context manager: conversation history + memory injection
- Tool: `web_search` (stub → real in M8)
- Tool: `open_app` (macOS via osascript)
- Tool: `open_url` / `open_file`
- Tool: `clipboard_read` / `clipboard_write`
- Permission manager: category-based approval
- Permission prompt UI connected to Rust backend
- Dashboard: tool activity, plan steps, execution status
- Verify: User asks "Open Safari" → plan generated → permission prompt → Safari opens

### M5: Memory System
- SQLite initialization + migrations
- Memory schema (memories, embeddings, relations tables)
- sqlite-vec setup for vector search
- Embedding: local ONNX model or API-based embedding
- Memory save (explicit + auto-candidate extraction)
- Memory search (semantic + metadata)
- Memory ranking (recency + importance + access count)
- Memory CRUD API (Rust commands)
- Frontend: memory panel with search, edit, pin, delete
- Dashboard: memory events
- Verify: "Remember I like dark mode" → saved → "What are my preferences?" → recalled

### M6: Voice Pipeline
- Microphone capture via CPAL
- Apple Speech framework STT integration
- AVSpeechSynthesizer TTS integration
- Voice activity state machine (idle → listening → processing → speaking)
- Push-to-talk activation
- Hotkey activation (global shortcut via Tauri)
- Voice indicator UI wired to state
- Settings: voice selection, device picker
- Verify: Push-to-talk → speak → transcription → agent response → TTS speaks

## Phase 3: Advanced Features (Week 5-6)

### M7: Clap Detection + Activation
- Audio stream analysis module
- Transient detection: short-time energy + spectral analysis
- Threshold calibration UI (3-clap calibration)
- Sensitivity controls (1-10)
- Cooldown logic (2-second minimum between triggers)
- False-positive mitigation (bandpass filter, temporal pattern)
- Activation manager: unified source priority
- Dashboard: activation events, clap detection log
- Verify: Clap → JARVIS activates → user speaks → response

### M8: Desktop Control (Full)
- App launching (NSWorkspace + AppleScript)
- App focus/switching
- App quitting (with confirmation)
- Window management (AXUIElement: list, move, resize)
- Menu item triggering (Accessibility API)
- Keystroke sending (CGEvent)
- Window arrangement commands ("split left/right", "tile", "maximize")
- Permission prompts per category
- Dashboard: desktop control actions
- Verify: "Open Safari and put it on the left half" → Safari opens + window arranged

### M9: Web Search + Recruiting Tools
- Web search provider (API-based)
- Search result formatting and citation
- Answer synthesis (Claude summarizes search results)
- Recruiting tools:
  - Draft job description
  - Draft outreach message
  - Draft interview questions
  - Parse/summarize resume
  - Compare candidates
- Tool definitions registered in agent
- Dashboard: search results, tool activity
- Verify: "Search for Rust Tauri best practices" → results shown with sources

## Phase 4: Polish + Ship (Week 7-8)

### M10: Settings + System Tray
- Settings panel UI
- API key management (Keychain storage)
- Voice configuration (device, voice, speed)
- Permission review and editing
- Privacy settings
- Activation method toggles
- System tray icon
- Tray menu (show/hide, quit, status)
- Minimize to tray behavior
- Verify: Settings persist across restarts, tray works

### M11: Observability + Polish
- Event bus fully wired (all events → dashboard)
- Action log persistence (SQLite)
- Error tracking and display
- Performance metrics (response latency, memory lookup time)
- Smooth CSS transitions for all state changes
- Loading states for all async operations
- Graceful error messages for API failures
- Offline mode handling
- Debug mode toggle
- Verify: Dashboard shows real-time events for every action

### M12: Verification + Documentation
- Full end-to-end test pass
- Type check: `tsc --noEmit`
- Lint: ESLint
- Rust check: `cargo clippy`
- Build: `cargo tauri build`
- README with setup, run, test instructions
- .env.example with required keys
- Known limitations documented
- What's mocked vs real documented
- What's experimental documented (clap detection)
- Verify: Fresh clone → install → build → run works

## Post-V1 Enhancements

- Windows support (Tauri already supports it, needs `WindowsDesktopProvider`)
- Wake word detection (Porcupine or similar)
- Memory cloud sync (optional, encrypted)
- Plugin system for community tools
- MCP integration points
- Custom wake word training
- ATS integration for recruiting
- Multi-conversation support
- Export/import memory