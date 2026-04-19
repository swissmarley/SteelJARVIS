use std::process::Command;

/// Speech manager using macOS native speech APIs.
/// STT: Uses `speech` CLI or AppleScript with SFSpeechRecognizer
/// TTS: Uses `say` command (AVSpeechSynthesizer CLI equivalent)
pub struct SpeechManager {
    voice_name: String,
    rate: u32,
    available_voices: Vec<String>,
}

impl SpeechManager {
    pub fn new() -> Self {
        let voices = Self::list_voices_sync();
        Self {
            voice_name: "Samantha".to_string(),
            rate: 200,
            available_voices: voices,
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
        if text.is_empty() {
            return Ok(());
        }

        // Run `say` in background so it doesn't block
        Command::new("say")
            .arg("-v")
            .arg(&self.voice_name)
            .arg("-r")
            .arg(self.rate.to_string())
            .arg(text)
            .spawn()
            .map_err(|e| format!("TTS spawn error: {}", e))?;

        Ok(())
    }

    pub fn stop_speaking(&self) -> Result<(), String> {
        // Kill any running `say` process
        let _ = Command::new("killall")
            .arg("say")
            .spawn();
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

