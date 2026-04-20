//! Shared greeting pre-step for the chat and voice paths.
//!
//! Before this module existed, `commands/chat.rs` and `voice/stt.rs` each
//! carried a nearly identical block that decided whether to fire a
//! contextual greeting, called `generate_greeting`, and emitted
//! `JarvisEvent::JarvisGreeting`. Two copies drifted apart over the course
//! of review (one fixed the retry-storm bug, the other didn't; one marked
//! greeted on pure-hellos, the other didn't). Collapsing them into one
//! async helper means behaviour stays in lock-step and the two call sites
//! only differ in how they speak the returned text.

use std::sync::Mutex;

use crate::agent::{build_context, generate_greeting};
use crate::memory::{Embedder, MemoryStore};
use crate::observability::{EventBus, JarvisEvent};
use crate::session::SessionTracker;

/// Heuristic: is this just a hello-style utterance with no task content?
///
/// When the user opens with a pure hello, the agent's own reply is the
/// greeting — we don't want to fire a second greeting on top of it. But
/// we still need to `mark_greeted` so the next real message doesn't
/// trigger a delayed one.
pub fn is_pure_greeting(s: &str) -> bool {
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

/// Try to produce a contextual JARVIS greeting for this turn.
///
/// Semantics (identical for chat and voice paths):
///   * `should_greet == false` → no-op, returns `None`.
///   * `should_greet && is_pure_greeting(message)` → mark greeted,
///     return `None`. The agent's normal reply covers the hello.
///   * `should_greet && !is_pure_greeting(message)` → mark greeted
///     *before* the API call (so a failure can't retry-storm on every
///     subsequent turn), generate a greeting, emit `JarvisGreeting`,
///     return `Some(text)`. The caller is responsible for speaking the
///     text the way its transport prefers (frontend TTS event for the
///     chat path, `SpeechManager::speak_async` for voice).
///
/// On API error, `mark_greeted` stays set and we return `None` after
/// logging. This is the deliberate trade-off from the end-of-branch
/// review: skip one greeting rather than hammer Anthropic every turn
/// on a flaky network.
pub async fn try_greet(
    api_key: &str,
    message: &str,
    mem_store: &Mutex<MemoryStore>,
    embedder: &Embedder,
    tracker: &SessionTracker,
    event_bus: &Mutex<EventBus>,
) -> Option<String> {
    if !tracker.should_greet() {
        return None;
    }
    if is_pure_greeting(message) {
        tracker.mark_greeted();
        return None;
    }
    tracker.mark_greeted();
    if api_key.is_empty() {
        return None;
    }

    let greeting_ctx = build_context(mem_store, embedder, tracker, None);
    match generate_greeting(api_key, &greeting_ctx).await {
        Ok(text) => {
            if let Ok(bus) = event_bus.lock() {
                bus.emit(JarvisEvent::JarvisGreeting { text: text.clone() });
            }
            Some(text)
        }
        Err(e) => {
            eprintln!("[Greet] greeting skipped: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_pure_greeting;

    #[test]
    fn short_hellos_are_pure() {
        assert!(is_pure_greeting("hi"));
        assert!(is_pure_greeting("  Hello  "));
        assert!(is_pure_greeting("hey jarvis"));
        assert!(is_pure_greeting("Good evening"));
    }

    #[test]
    fn task_sentences_are_not_pure() {
        assert!(!is_pure_greeting("hi, open spotify"));
        assert!(!is_pure_greeting("hello what is the weather"));
    }

    #[test]
    fn overly_long_input_is_not_pure() {
        assert!(!is_pure_greeting("hello, I was just wondering about the weather"));
    }
}
