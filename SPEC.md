# JARVIS — Product Specification

## Overview

JARVIS is a production-grade autonomous desktop assistant inspired by Tony Stark's JARVIS. It is a real multi-layer application combining conversation, voice interaction, memory, planning, tools, desktop control, task execution, a live control center dashboard, and persistent context.

JARVIS is not a chatbot wrapper. It is an autonomous desktop operating companion.

## Product Principles

1. **Real architecture, not a demo** — modular, extensible, production-minded
2. **Honest about limitations** — no faked functionality
3. **Observable** — every major capability visible through the dashboard
4. **Auditable** — every autonomous action has a visible status and audit trail
5. **Conversational and agentic** — proactive, not passive
6. **Premium feel** — futuristic, minimal, highly readable
7. **Graceful degradation** — capabilities degrade cleanly when unavailable
8. **Safe** — explicit permissions for destructive/sensitive actions

## Platform

- **Primary**: macOS (Apple Silicon + Intel)
- **Secondary**: Windows (future milestone)
- **Shell**: Tauri (Rust backend + React frontend)
- **OS Abstraction**: Platform-specific modules behind a common interface

## User Experience

### Startup
- On launch, JARVIS opens in a visually impressive desktop window
- Proactive greeting: "Good evening. What would you like me to do?"
- Text input, microphone input, and system status visible from first screen
- Tray/minimized behavior supported

### Core Interaction
- Natural language input (text or voice)
- Intent interpretation and action planning
- Clarifying questions only when needed
- Multi-step autonomous task execution
- Progress narration while working

### Dashboard
Persistent interactive dashboard with sections:
- Current status / execution mode
- Active goal
- Task queue (open, in-progress, completed, blocked)
- Action timeline
- Tools in use
- Memory writes and retrievals
- Permission/approval requests
- Open apps / running automations
- Search/research results
- Errors/warnings
- Voice state (listening, speaking, idle)
- Wake/activation state
- Agent confidence
- Provider status
- System health

Execution modes: idle | listening | planning | acting | waiting | blocked | error | completed

### Dashboard Panels (Collapsible)
- Timeline view
- Task inspector
- Memory inspector
- Execution inspector
- Settings for wake modes
- Settings for privacy and permissions

## Voice and Activation

### Speech Pipeline
- **STT**: Apple Speech framework (on-device when possible, server fallback)
- **TTS**: AVSpeechSynthesizer (on-device, premium Apple voices)
- **Voice activity state**: idle | listening | processing | speaking
- **Interruption handling**: user can interrupt TTS with new input

### Activation Methods (Priority Order)
1. **Clap detection** (primary) — audio transient detection via macOS audio APIs
2. **Hotkey** — global keyboard shortcut
3. **Push-to-talk** — hold key to speak
4. **Wake word** — optional, configurable phrase
5. **Click/tap** — UI button

### Clap Detection
- Dedicated module with sensitivity controls and threshold tuning
- Cooldown logic to prevent double-triggering
- False-positive prevention (noise filtering, transient pattern matching)
- Calibration UI for user-tuning
- Dashboard shows detection events
- User opt-in required
- Documented limitations for noisy environments

### Device Selection
- Microphone device picker in settings
- Default to system default input

## Long-Term Memory

### Memory Categories
- User profile
- Preferences
- Explicit facts ("remember this")
- Task history
- Recurring workflows
- App and system preferences
- Recruiting context
- Relationships between entities
- Semantic notes and summaries

### Memory Rules
- Explicit save: user says "remember this"
- Automatic candidate extraction: clearly useful long-term facts
- User confirmation for auto-extracted memories
- Every memory event visible and reviewable in dashboard
- User can inspect, edit, pin, delete, and search memories

### Memory Metadata
- Confidence score
- Source (explicit | auto-extracted | tool-result)
- Timestamp
- Type/category
- Privacy label
- Provenance (which conversation/context produced it)

### Memory Operations
- Semantic similarity search (via vector embeddings)
- Recency + importance scoring
- Memory compaction (summarize old memories)
- Deduplication
- Conflict handling (contradictory memories flagged)
- Retrieval ranking

### Storage
- **Structured data**: SQLite
- **Vector search**: SQLite-vec extension
- **Local only**: all memory stays on-device

## Tools and Capabilities

### Web and Research
- Web search (via API)
- Answer synthesis
- Source citation

### File Operations
- Read files
- Summarize documents
- Open files, URLs, folders

### Desktop Control (macOS)
- App launching (by name)
- App focus/switching
- App quitting
- File/URL/folder opening (NSWorkspace)
- Menu item triggering (Accessibility API)
- Button clicking (Accessibility API)
- Keyboard input to other apps
- Window management (move, resize, arrange)

### Recruiting (Lightweight)
- Draft job descriptions
- Draft outreach messages
- Draft interview questions
- Resume/CV parsing and summarization
- Candidate comparison
- Pipeline status tracked in memory (not a dedicated data model)

### Communication
- Email/message drafting
- Clipboard operations

### System
- Calendar/task integration (if useful)
- Reminder creation
- Notes capture
- Multi-step workflow orchestration

## Desktop Control Details

### macOS Implementation
- **App launching**: AppleScript `tell application`, NSWorkspace
- **Window management**: Accessibility API (AXUIElement)
- **Menu/keyboard**: Accessibility API + CGEvent
- **File/URL opening**: NSWorkspace `openURL` / `openFile`

### Permission Model
- All desktop control actions require first-use approval
- Categories: app-launch, app-control, file-access, window-control
- User can pre-approve categories or require per-action confirmation
- Sensitive actions (quit app, keyboard input) always require confirmation
- Permissions are persisted and reviewable

### Platform Abstraction
- `DesktopProvider` trait in Rust
- `MacOSDesktopProvider` implementation
- Future: `WindowsDesktopProvider`

## Autonomy Model

### Operating Modes
1. **Manual** — JARVIS suggests, user approves every action
2. **Assistive** — JARVIS executes safe actions, asks for risky ones
3. **Autonomous** — JARVIS executes with minimal interruption
4. **Confirmation-required** — every action needs approval
5. **Safe/Read-only** — no mutations, only reads and displays

### Agent Behavior
- Propose plans before executing
- Execute approved actions step by step
- Recover from failures with retry/fallback logic
- Ask for confirmation on risky actions
- Summarize outcomes clearly
- Maintain execution trace in dashboard

## AI Provider

- **Primary**: Claude API (Anthropic) — claude-sonnet-4-6 for reasoning/planning/tool-use
- **Configuration**: API key stored securely in macOS Keychain or local config
- **Streaming**: responses stream to UI in real-time
- **Context**: conversation history + relevant memories + tool results
- **Fallback**: graceful error messages when API is unreachable

## Security

- Sensitive data (API keys) stored in macOS Keychain or encrypted local config
- No memory or conversation data leaves the machine except AI API calls
- Action confirmations for destructive operations (delete files, quit apps, send messages)
- Sandboxed boundaries where possible
- Redaction of sensitive content from logs
- Permission policies per tool category
- User-controllable privacy settings

## Design Direction

### JARVIS HUD Style
- Dark background (#0a0a0f range)
- Glowing accent panels (cyan/blue/white accents)
- Translucent card sections (glassmorphism in moderation)
- Monospace data displays for dashboard metrics
- Clean sans-serif for body text
- Smooth state transitions (no jarring jumps)
- Execution mode indicator with color coding
- Elegant microphone/listening visuals
- Sophisticated activity timeline

### Typography
- Data/code: monospace (JetBrains Mono or system monospace)
- UI text: clean sans-serif (Inter or system font)
- Headings: medium weight, generous spacing

### Color Palette
- Background: near-black (#0a0a0f)
- Surface: dark panels with subtle transparency (#1a1a2e at 80% opacity)
- Accent primary: cyan (#00d4ff)
- Accent secondary: warm gold (#ffd700) for warnings/active
- Error: red (#ff4444)
- Success: green (#00ff88)
- Text primary: white/near-white (#e0e0e0)
- Text secondary: muted (#888)

### Avoid
- Generic chatbot look
- Cheap neon sci-fi clichés
- Cluttered analytics UI
- Style over usability
- Fake "AI thinking" theatrics

## Non-Goals (V1)

- Mobile app
- Multi-user support
- Cloud sync of memory
- Plugin marketplace
- Full ATS integration for recruiting
- Custom wake word training
- Cross-platform in V1 (Windows is V2)