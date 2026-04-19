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

    #[allow(dead_code)]
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
