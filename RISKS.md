# JARVIS — Risks and Constraints

## High Risk

### Clap Detection Reliability
- **Risk**: Hand clap detection is notoriously unreliable in varied environments. Background noise, music, typing, and ambient sounds produce false positives.
- **Impact**: Primary activation method may frustrate users if it triggers unexpectedly or misses claps.
- **Mitigation**: Ship clap detection as experimental with clear labeling. Default activation is hotkey/push-to-talk. Clap module has calibration, sensitivity controls, and cooldown. Document limitations prominently.

### macOS Accessibility API Fragility
- **Risk**: AppleScript and Accessibility API calls break with OS updates. App names change, menu hierarchies shift, AXUIElement paths become invalid.
- **Impact**: Desktop control tools fail silently or crash. Window management stops working after macOS updates.
- **Mitigation**: All Accessibility calls wrapped in Result types with descriptive errors. Fallback strategies documented. Regular testing against OS updates. Keep AppleScript simple and resilient (use app names, not bundle IDs where possible).

### Claude API Dependency
- **Risk**: JARVIS is fundamentally dependent on Claude API. API outages, rate limits, or key issues make the assistant non-functional.
- **Impact**: Complete loss of reasoning capability when API is unavailable.
- **Mitigation**: Graceful degradation with clear error messages. Queue messages during outage. Show provider status in dashboard. Consider local model fallback for future versions.

## Medium Risk

### SQLite-vec Maturity
- **Risk**: sqlite-vec is relatively new. API may change, performance may not scale, bugs may exist.
- **Impact**: Memory semantic search could be slow, inaccurate, or break.
- **Mitigation**: SQLite-vec is lightweight and can be swapped. Keep embedding logic abstracted behind a trait. Fall back to simple keyword search if vector search fails.

### Tauri + Rust Complexity
- **Risk**: Tauri's Rust backend adds significant complexity vs. Electron. Fewer libraries, steeper learning curve for contributors.
- **Impact**: Slower development velocity. More difficult debugging. Smaller community support.
- **Mitigation**: Use well-maintained crates. Keep Rust code well-organized. Document patterns clearly. The performance and security benefits justify the complexity.

### Local Embedding Model
- **Risk**: Running an embedding model locally (even small ones like MiniLM) requires ONNX Runtime and may have compatibility issues across macOS versions.
- **Impact**: Memory semantic search won't work if embedding model fails to load.
- **Mitigation**: Fallback to API-based embedding (OpenAI embeddings endpoint). Abstract embedding behind a trait. Show clear status in dashboard.

### CPAL Audio Capture
- **Risk**: CPAL's macOS backend may have issues with specific audio hardware, sample rates, or permissions.
- **Impact**: Microphone input doesn't work, clap detection fails, voice input fails.
- **Mitigation**: Use Apple's AVAudioEngine as alternative. Test on multiple devices. Fallback to Apple Speech framework's built-in audio capture for STT.

## Low Risk

### macOS Keychain Access
- **Risk**: `security` command or keychain crate may require user authorization prompts.
- **Impact**: API key storage falls back to encrypted file.
- **Mitigation**: Implement file-based encrypted storage as fallback. Document both options.

### Tauri Window Customization
- **Risk**: Custom title bars, transparent windows, and glass effects may have rendering issues.
- **Impact**: HUD design looks broken on some macOS versions or hardware.
- **Mitigation**: Use standard Tauri decorations as fallback. Test on Intel and Apple Silicon. Keep glass effects as progressive enhancement.

### zustand State Size
- **Risk**: Dashboard state (timeline, events, logs) could grow unbounded in long sessions.
- **Impact**: Frontend performance degrades over time.
- **Mitigation**: Cap event lists at 1000 entries. Implement windowed rendering for timeline. Compact old events periodically.

## Constraints

### macOS Only (V1)
- No Windows or Linux support in V1. Platform abstraction exists but only macOS is implemented.
- Some features (Accessibility API, AppleScript) have no Windows equivalent yet.

### Internet Required for AI
- Local-first means data stays local, but AI reasoning requires internet for Claude API calls.
- Offline mode is limited to: memory recall, settings, dashboard, local file operations.

### No Custom Wake Word Training
- Wake word detection uses pre-built models. Custom phrase training is a V2 feature.

### Recruiting is Lightweight
- No ATS integration. No dedicated pipeline database. Recruiting context lives in memory system.
- Resume parsing is AI-assisted, not structured extraction.

### Clap Detection is Experimental
- Labeled as experimental in all UI. Default activation is hotkey/push-to-talk.
- Works best in quiet environments. Documented limitations.

### App Store Distribution
- Tauri apps can be distributed outside the App Store. Notary signing needed for macOS.
- Accessibility permissions require manual user approval in System Settings.