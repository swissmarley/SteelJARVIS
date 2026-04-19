use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

use crate::voice::SpeechRecognizer;

/// Speech manager using macOS native speech APIs.
/// STT: Uses `speech` CLI or AppleScript with SFSpeechRecognizer
/// TTS: Uses `say` command (AVSpeechSynthesizer CLI equivalent)
pub struct SpeechManager {
    voice_name: String,
    rate: u32,
    available_voices: Vec<String>,
    // Tracks the text currently being spoken. Used to decide when a spawned
    // `say` has finished so we can resume STT exactly once per utterance.
    current_utterance: Arc<Mutex<Option<String>>>,
}

impl SpeechManager {
    pub fn new() -> Self {
        let voices = Self::list_voices_sync();
        // Daniel (en_GB) is the macOS voice closest to the Paul Bettany JARVIS
        // timbre — British male, measured cadence. Prefer the Premium /
        // Enhanced variants when the user has downloaded them since they
        // sound noticeably less synthetic. Rate 180 gives a slight butler
        // deliberation vs. the default 200.
        let preferred = [
            "Daniel (Premium)",
            "Daniel (Enhanced)",
            "Oliver (Premium)",
            "Oliver (Enhanced)",
            "Arthur (Premium)",
            "Arthur (Enhanced)",
            "Daniel",
            "Oliver",
            "Arthur",
            "Samantha",
        ];
        let default_voice = preferred
            .iter()
            .find(|name| voices.iter().any(|v| v == *name))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Samantha".to_string());
        Self {
            voice_name: default_voice,
            rate: 180,
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

    /// Speak asynchronously. Pauses STT for the duration of the utterance so
    /// the recognizer never picks up JARVIS's own voice (which previously led
    /// to phantom "user said" events and "you're repeating" replies). STT is
    /// resumed ~400ms after `say` exits to let the mic tail settle.
    pub fn speak_async(&self, text: &str, app: &AppHandle) -> Result<(), String> {
        let spoken = sanitize_for_tts(text);
        if spoken.trim().is_empty() {
            return Ok(());
        }

        // Mute STT before any audio leaves the speaker.
        if let Some(stt_state) = app.try_state::<Mutex<SpeechRecognizer>>() {
            if let Ok(mut stt) = stt_state.lock() {
                let _ = stt.stop();
            }
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

        let utt = self.current_utterance.clone();
        let my_text = spoken.clone();
        let app_for_resume = app.clone();
        std::thread::spawn(move || {
            let _ = child.wait();
            let should_resume = {
                if let Ok(mut guard) = utt.lock() {
                    // Only the last-started utterance resumes STT; if a newer
                    // speak_async overwrote us, its own wait thread will handle it.
                    if guard.as_deref() == Some(my_text.as_str()) {
                        *guard = None;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if !should_resume {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
            if let Some(stt_state) = app_for_resume.try_state::<Mutex<SpeechRecognizer>>() {
                if let Ok(mut stt) = stt_state.lock() {
                    if let Err(e) = stt.start() {
                        eprintln!("[TTS→STT] failed to resume listening: {}", e);
                    }
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
                // `say -v ?` prints lines like:
                //   Daniel (Enhanced)   en_GB    # Hello! My name is Daniel.
                // The voice name can contain spaces and parens, so we can't
                // just split on whitespace. Strip the `# greeting`, then
                // treat everything before the final whitespace-separated
                // token (the locale, e.g. `en_GB`) as the voice name. Only
                // keep English voices so the UI picker stays manageable —
                // non-English voices are still usable via direct `set_voice`.
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter_map(|line| {
                        let before_hash = line.split('#').next().unwrap_or("").trim_end();
                        let split_pos = before_hash.rfind(char::is_whitespace)?;
                        let locale = before_hash[split_pos..].trim();
                        if !locale.starts_with("en_") {
                            return None;
                        }
                        let name = before_hash[..split_pos].trim();
                        if name.is_empty() {
                            None
                        } else {
                            Some(name.to_string())
                        }
                    })
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
