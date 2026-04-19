use std::process::Command;
use std::sync::{Arc, Mutex};

/// Speech manager using macOS native speech APIs.
/// STT: Uses `speech` CLI or AppleScript with SFSpeechRecognizer
/// TTS: Uses `say` command (AVSpeechSynthesizer CLI equivalent)
pub struct SpeechManager {
    voice_name: String,
    rate: u32,
    available_voices: Vec<String>,
    // Text currently being spoken — used by the STT thread to distinguish
    // JARVIS's own voice bleeding into the mic from a real user interrupt.
    current_utterance: Arc<Mutex<Option<String>>>,
}

impl SpeechManager {
    pub fn new() -> Self {
        let voices = Self::list_voices_sync();
        Self {
            voice_name: "Samantha".to_string(),
            rate: 200,
            available_voices: voices,
            current_utterance: Arc::new(Mutex::new(None)),
        }
    }

    #[allow(dead_code)]
    pub fn speak(&self, text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }

        let output = Command::new("say")
            .arg("-v")
            .arg(&self.voice_name)
            .arg("-r")
            .arg(self.rate.to_string())
            .arg(text)
            .output()
            .map_err(|e| format!("TTS error: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!("TTS failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    pub fn speak_async(&self, text: &str) -> Result<(), String> {
        let spoken = sanitize_for_tts(text);
        if spoken.trim().is_empty() {
            return Ok(());
        }

        // Kill any previous `say` so back-to-back responses never overlap.
        let _ = Command::new("killall").arg("say").output();

        if let Ok(mut guard) = self.current_utterance.lock() {
            *guard = Some(spoken.clone());
        }

        let mut child = Command::new("say")
            .arg("-v")
            .arg(&self.voice_name)
            .arg("-r")
            .arg(self.rate.to_string())
            .arg(&spoken)
            .spawn()
            .map_err(|e| format!("TTS spawn error: {}", e))?;

        // Clear the current utterance once this `say` exits so later partials
        // aren't compared against stale text. Only clear if we're still the
        // active utterance — a newer speak_async may have taken over.
        let utt = self.current_utterance.clone();
        let my_text = spoken.clone();
        std::thread::spawn(move || {
            let _ = child.wait();
            if let Ok(mut guard) = utt.lock() {
                if guard.as_deref() == Some(my_text.as_str()) {
                    *guard = None;
                }
            }
        });

        Ok(())
    }

    pub fn stop_speaking(&self) -> Result<(), String> {
        let _ = Command::new("killall").arg("say").spawn();
        if let Ok(mut guard) = self.current_utterance.lock() {
            *guard = None;
        }
        Ok(())
    }

    /// True while a `say` subprocess is still running.
    pub fn is_speaking(&self) -> bool {
        self.current_utterance
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    /// Heuristic echo filter: returns true when `partial` looks like JARVIS's
    /// own TTS output being picked up by the mic, rather than fresh user
    /// speech. Both sides are stripped of punctuation and lowercased before
    /// comparing; we also accept a 50%+ word overlap since SFSpeechRecognizer
    /// occasionally mis-transcribes the synthesized voice (e.g. "I" → "eye").
    pub fn is_echo_of_current(&self, partial: &str) -> bool {
        let guard = match self.current_utterance.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        let utterance = match guard.as_ref() {
            Some(u) => normalize_for_match(u),
            None => return false,
        };
        let p = normalize_for_match(partial);
        if p.is_empty() {
            return true;
        }

        if utterance.contains(&p) {
            return true;
        }

        let partial_words: Vec<&str> = p.split_whitespace().collect();
        if partial_words.is_empty() {
            return true;
        }
        let utterance_words: std::collections::HashSet<&str> =
            utterance.split_whitespace().collect();
        let overlap = partial_words
            .iter()
            .filter(|w| utterance_words.contains(*w))
            .count();
        let ratio = overlap as f32 / partial_words.len() as f32;
        ratio >= 0.5
    }

    pub fn set_voice(&mut self, name: &str) -> Result<(), String> {
        if self.available_voices.contains(&name.to_string()) || name.is_empty() {
            self.voice_name = name.to_string();
            Ok(())
        } else {
            Err(format!("Voice '{}' not available", name))
        }
    }

    pub fn set_rate(&mut self, rate: u32) {
        self.rate = rate.clamp(50, 500);
    }

    pub fn get_voice(&self) -> &str {
        &self.voice_name
    }

    pub fn get_rate(&self) -> u32 {
        self.rate
    }

    pub fn list_voices_sync() -> Vec<String> {
        let output = Command::new("say")
            .arg("-v")
            .arg("?")
            .output();

        match output {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
                        parts.first().map(|s| s.to_string())
                    })
                    .filter(|s| !s.is_empty())
                    .collect()
            }
            _ => vec!["Samantha".to_string(), "Alex".to_string(), "Victoria".to_string()],
        }
    }

    pub fn available_voices(&self) -> &[String] {
        &self.available_voices
    }
}

/// Strip emoji, markdown markers, and fenced code blocks so macOS `say`
/// doesn't read "rocket emoji" or asterisks aloud.
fn sanitize_for_tts(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }
        for ch in line.chars() {
            if is_emoji_or_symbol(ch) {
                continue;
            }
            // Drop markdown emphasis/heading/code markers; keep regular punctuation.
            if matches!(ch, '*' | '_' | '#' | '`' | '~') {
                continue;
            }
            out.push(ch);
        }
        out.push(' ');
    }
    // Collapse runs of whitespace introduced by stripping.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_emoji_or_symbol(ch: char) -> bool {
    let code = ch as u32;
    // Pictographs, symbols, dingbats, supplemental symbols, and the ZWJ /
    // variation selectors that glue multi-codepoint emoji together.
    (0x1F300..=0x1FAFF).contains(&code)
        || (0x2600..=0x27BF).contains(&code)
        || (0x1F000..=0x1F2FF).contains(&code)
        || (0x2300..=0x23FF).contains(&code)
        || code == 0xFE0F
        || code == 0x200D
}

/// Lowercase, drop punctuation, collapse whitespace — used for comparing
/// partial transcripts against current TTS text.
fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
