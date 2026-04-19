# JARVIS — Autonomous Desktop Assistant

A production-grade JARVIS-style desktop assistant built with Tauri, React, and Claude.

## Features

- Conversational AI assistant powered by Claude with tool use
- Live dashboard with 8 collapsible panels (status, tasks, timeline, tools, memory, permissions, errors, health)
- Long-term memory with SQLite storage and search
- macOS desktop control (app launching, URL/file opening, quit, list apps)
- Voice output via macOS `say` command (AVSpeechSynthesizer)
- Clap detection module with transient analysis (experimental)
- Permission system with 8 categories and 4 levels
- Web search via Google Custom Search API
- Recruiting tools: draft JDs, outreach messages, interview questions
- HUD-style dark interface with glass/panel design
- Event bus for full observability

## Prerequisites

- macOS 13+ (Apple Silicon or Intel)
- [Rust](https://rustup.rs/) (1.75+)
- [Node.js](https://nodejs.org/) (20+)
- Xcode Command Line Tools: `xcode-select --install`
- Anthropic API key ([get one here](https://console.anthropic.com/))

## Setup

```bash
# Clone and enter the project
cd SteelJARVIS

# Install frontend dependencies
npm install --cache /tmp/npm-cache

# Copy environment template and add your API key
cp .env.example .env
# Edit .env and add your ANTHROPIC_API_KEY

# Run in development mode
npm run tauri:dev
```

**Note for external SSD users:** If your project is on an external drive, set `CARGO_TARGET_DIR` to avoid macOS AppleDouble file issues:

```bash
export CARGO_TARGET_DIR=/tmp/jarvis-target
npm run tauri:dev
```

## Development

```bash
# Start dev server with hot reload
npm run tauri:dev

# Type check frontend
npm run typecheck

# Lint frontend
npm run lint

# Check Rust backend
cargo check --manifest-path src-tauri/Cargo.toml

# Production build
npm run tauri:build
```

## Architecture

```
SteelJARVIS/
├── src/                     # React frontend
│   ├── components/
│   │   ├── AppShell/        # Main layout + header
│   │   ├── ChatPanel/       # Conversation UI + speak button
│   │   ├── Dashboard/       # 8 collapsible panels
│   │   └── VoiceIndicator/  # Voice state display
│   ├── stores/              # Zustand (conversation, dashboard, voice)
│   ├── hooks/               # useTauriEvent
│   ├── types/               # events, messages, memory, tools
│   └── styles/              # CSS variables + global + modules
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── commands/        # 30+ Tauri IPC commands
│   │   ├── agent/           # Claude API + tool-use engine
│   │   ├── memory/          # SQLite memory store
│   │   ├── desktop/         # macOS desktop control
│   │   ├── voice/           # TTS, clap detection, activation
│   │   ├── search/          # Web search provider
│   │   ├── permissions/     # Permission manager
│   │   └── observability/   # Event bus
│   └── icons/               # App icons
├── SPEC.md                  # Product specification
├── ARCHITECTURE.md          # System architecture
├── ROADMAP.md               # Implementation roadmap
├── RISKS.md                 # Risks and constraints
└── MILESTONES.md            # Milestone breakdown
```

## Current Status

- [x] Project scaffold (Tauri 2 + React 19 + TypeScript)
- [x] HUD dashboard UI with 8 collapsible panels
- [x] Chat interface with streaming display
- [x] Claude API integration with tool use (5+ tools)
- [x] Memory system (SQLite with search, CRUD, pin)
- [x] macOS desktop control (launch, open, quit, list apps)
- [x] Permission system (8 categories, 4 levels)
- [x] Event bus / observability
- [x] Voice output (macOS `say` command)
- [x] Clap detection module (experimental, configurable)
- [x] Activation manager (clap, hotkey, push-to-talk, UI)
- [x] Web search (Google Custom Search API)
- [x] Recruiting tools (JD, outreach, interview questions)
- [x] Speak button on assistant messages
- [ ] System tray icon
- [ ] Settings persistence
- [ ] Wake word detection
- [ ] STT via Apple Speech framework (requires Swift bridge)
- [ ] Window management (Accessibility API)
- [ ] Menu/keyboard control (Accessibility API)

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Shell | Tauri 2.10 (Rust) |
| Frontend | React 19 + TypeScript |
| Build | Vite 8 |
| State | Zustand + Immer |
| AI | Claude API (Anthropic) with tool use |
| Database | SQLite |
| Speech TTS | macOS `say` (AVSpeechSynthesizer) |
| Desktop Control | AppleScript + NSWorkspace |
| Clap Detection | CPAL + transient analysis |
| Bundle Size | ~15MB (vs Electron ~150MB) |

## License

Private project.