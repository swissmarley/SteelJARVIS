# SteelJARVIS — Project Conventions

## Stack
- **Desktop Shell**: Tauri 2.x (Rust backend + React frontend)
- **Frontend**: React 18+ with TypeScript, Vite bundler
- **State Management**: Zustand with Immer
- **Styling**: CSS Modules + CSS custom properties (no Tailwind)
- **Backend**: Rust (Tauri commands, tokio async runtime)
- **Database**: SQLite + sqlite-vec (via rusqlite)
- **AI Provider**: Claude API (Anthropic SDK via HTTP/SSE)
- **Speech**: Apple native (AVSpeechSynthesizer, SFSpeechRecognizer)
- **Audio**: CPAL for raw microphone capture
- **Desktop Control**: AppleScript + macOS Accessibility API (AXUIElement)
- **Embeddings**: ONNX Runtime (local) or API-based fallback

## Architecture
- See ARCHITECTURE.md for full system design
- See SPEC.md for product specification
- See ROADMAP.md for implementation order
- See MILESTONES.md for milestone definitions and checklists

## Code Style

### Rust
- Use `#[tauri::command]` for all IPC commands
- All commands return `Result<T, String>` for error propagation to frontend
- Use `serde::Serialize` for all types crossing the IPC boundary
- Prefer `tokio::spawn` for async work; never block the main thread
- All errors use `thiserror` for structured error types
- Event bus uses `tokio::sync::broadcast`
- Database access through `rusqlite::Connection` wrapped in `Mutex`

### TypeScript/React
- Functional components only, no class components
- Zustand stores in `src/stores/`
- Components in `src/components/`
- Types in `src/types/`
- Hooks in `src/hooks/`
- CSS Modules for component styles, `global.css` for variables/reset
- No inline styles except dynamic values
- No emoji in UI unless explicitly requested

### Naming
- Rust: snake_case for functions/variables, PascalCase for types/traits
- TypeScript: camelCase for functions/variables, PascalCase for components/types
- CSS: kebab-case for custom properties, camelCase or PascalCase for class names
- Files: kebab-case for utilities, PascalCase for React components
- Tauri commands: snake_case (e.g., `send_message`, `save_memory`)

## Commands

### Development
```bash
cargo tauri dev          # Start dev server with hot reload
cargo tauri build        # Production build
```

### Checks
```bash
npx tsc --noEmit         # TypeScript type check
npx eslint src/          # Lint frontend
cargo clippy             # Lint Rust
cargo test               # Run Rust tests
```

### Dependencies
```bash
npm install              # Install frontend deps
cargo build              # Build Rust deps
```

## Git
- Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`
- No force pushes to main
- Branch naming: `feat/mN-description`, `fix/description`

## Environment
- API keys: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` (optional)
- Store in `.env` (gitignored) or macOS Keychain
- `.env.example` documents all required/optional keys

## Security
- Never log API keys or conversation content to persistent logs
- All desktop control actions require permission approval
- Destructive actions (quit app, delete) always require confirmation
- Secrets stored in macOS Keychain or encrypted local config
- No telemetry by default

## Testing
- Rust: unit tests in each module, `#[cfg(test)]` blocks
- Frontend: component tests with React Testing Library
- Integration: test critical paths (message send, memory save, app launch)
- Manual verification required for: voice, clap detection, desktop control

## Known Limitations (Current)
- macOS only (V1)
- Internet required for AI reasoning
- Clap detection is experimental
- No custom wake word training
- No ATS integration (recruiting is lightweight)
- Accessibility API may break with macOS updates