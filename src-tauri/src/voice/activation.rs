use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Activation manager that coordinates multiple activation sources.
/// Sources are prioritized: clap > hotkey > push-to-talk > wake-word > ui-click
pub struct ActivationManager {
    clap_enabled: bool,
    hotkey_enabled: bool,
    ptt_enabled: bool,
    #[allow(dead_code)]
    last_activation: Arc<Mutex<Option<Instant>>>,
    cooldown_ms: u64,
}

impl ActivationManager {
    pub fn new() -> Self {
        Self {
            clap_enabled: false,
            hotkey_enabled: true,
            ptt_enabled: false,
            last_activation: Arc::new(Mutex::new(None)),
            cooldown_ms: 1000,
        }
    }

    pub fn set_clap_enabled(&mut self, enabled: bool) {
        self.clap_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn set_hotkey_enabled(&mut self, enabled: bool) {
        self.hotkey_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn set_ptt_enabled(&mut self, enabled: bool) {
        self.ptt_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn is_clap_enabled(&self) -> bool {
        self.clap_enabled
    }

    #[allow(dead_code)]
    pub fn is_hotkey_enabled(&self) -> bool {
        self.hotkey_enabled
    }

    #[allow(dead_code)]
    pub fn is_ptt_enabled(&self) -> bool {
        self.ptt_enabled
    }

    /// Check if activation is allowed (respects cooldown)
    #[allow(dead_code)]
    pub fn can_activate(&self) -> bool {
        let last = self.last_activation.lock().ok().and_then(|l| *l);
        if let Some(last) = last {
            last.elapsed().as_millis() as u64 >= self.cooldown_ms
        } else {
            true
        }
    }

    /// Record an activation event
    #[allow(dead_code)]
    pub fn activate(&self, source: &str) -> bool {
        if !self.can_activate() {
            return false;
        }

        if let Ok(mut last) = self.last_activation.lock() {
            *last = Some(Instant::now());
        }

        // Check if this source is enabled
        match source {
            "clap" => self.clap_enabled,
            "hotkey" => self.hotkey_enabled,
            "push-to-talk" => self.ptt_enabled,
            "wake-word" => true, // wake word is always available if registered
            "ui-click" => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActivationConfig {
    pub clap_enabled: bool,
    pub hotkey_enabled: bool,
    pub ptt_enabled: bool,
    pub cooldown_ms: u64,
}

impl From<&ActivationManager> for ActivationConfig {
    fn from(m: &ActivationManager) -> Self {
        Self {
            clap_enabled: m.clap_enabled,
            hotkey_enabled: m.hotkey_enabled,
            ptt_enabled: m.ptt_enabled,
            cooldown_ms: m.cooldown_ms,
        }
    }
}