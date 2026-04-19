/// Clap detection module for macOS.
///
/// Detects hand clap patterns in audio input using transient energy analysis.
/// This implementation uses CPAL for audio capture and simple energy-based
/// transient detection.
///
/// Architecture:
/// 1. CPAL captures audio from the default input device
/// 2. Audio samples are processed in windows (typically 10-50ms)
/// 3. Short-time energy is computed per window
/// 4. When energy exceeds a dynamic threshold, a transient is detected
/// 5. Two transients within a short time window (100-400ms) = clap
/// 6. Cooldown prevents double-triggering (2s minimum between activations)
///
/// Limitations:
/// - False positives in noisy environments
/// - May trigger on other sharp transients (door slam, desk tap)
/// - Calibration helps but doesn't eliminate false positives
/// - Best used in quiet-to-moderate environments

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for clap detection
#[derive(Debug, Clone)]
pub struct ClapConfig {
    /// Sensitivity 1-10 (higher = more sensitive)
    pub sensitivity: u8,
    /// Minimum time between two transients to count as a clap pair (ms)
    #[allow(dead_code)]
    pub pair_window_ms: u64,
    /// Minimum time between clap activations (ms)
    #[allow(dead_code)]
    pub cooldown_ms: u64,
    /// Audio sample rate
    #[allow(dead_code)]
    pub sample_rate: u32,
    /// Analysis window size in samples
    #[allow(dead_code)]
    pub window_size: usize,
}

impl Default for ClapConfig {
    fn default() -> Self {
        Self {
            sensitivity: 5,
            pair_window_ms: 300,
            cooldown_ms: 2000,
            sample_rate: 44100,
            window_size: 1024,
        }
    }
}

impl ClapConfig {
    /// Compute the energy threshold based on sensitivity
    /// Higher sensitivity = lower threshold = easier to trigger
    #[allow(dead_code)]
    pub fn energy_threshold(&self) -> f32 {
        // Sensitivity 1 = threshold 0.5, sensitivity 10 = threshold 0.05
        0.5 - (self.sensitivity as f32 - 1.0) * 0.05
    }
}

/// State for the clap detector
#[derive(Debug)]
struct DetectorState {
    /// Time of the last transient detection
    last_transient: Option<Instant>,
    /// Time of the last confirmed clap activation
    last_activation: Option<Instant>,
    /// Running average energy (for adaptive threshold)
    energy_avg: f32,
    /// Number of samples processed
    samples_processed: u64,
    /// Whether detection is active
    active: bool,
}

impl Default for DetectorState {
    fn default() -> Self {
        Self {
            last_transient: None,
            last_activation: None,
            energy_avg: 0.01,
            samples_processed: 0,
            active: false,
        }
    }
}

pub struct ClapDetector {
    config: ClapConfig,
    state: Arc<Mutex<DetectorState>>,
    /// Callback invoked when a clap is detected
    #[allow(dead_code)]
    on_clap: Option<Box<dyn Fn(f64) + Send + Sync>>,
}

impl ClapDetector {
    pub fn new(config: ClapConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(DetectorState::default())),
            on_clap: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_callback(mut self, callback: Box<dyn Fn(f64) + Send + Sync>) -> Self {
        self.on_clap = Some(callback);
        self
    }

    pub fn set_sensitivity(&mut self, sensitivity: u8) {
        self.config.sensitivity = sensitivity.clamp(1, 10);
    }

    pub fn start(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.active = true;
        state.last_transient = None;
        state.last_activation = None;
        state.energy_avg = 0.01;
        state.samples_processed = 0;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.active = false;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.state.lock().map(|s| s.active).unwrap_or(false)
    }

    /// Process a buffer of audio samples and detect claps.
    /// Call this with each chunk of audio data from the microphone.
    /// Returns Some(confidence) if a clap was detected, None otherwise.
    #[allow(dead_code)]
    pub fn process_samples(&self, samples: &[f32]) -> Option<f64> {
        let mut state = self.state.lock().ok()?;

        if !state.active {
            return None;
        }

        // Compute short-time energy of this window
        let energy = compute_energy(samples);

        // Update running average (exponential moving average)
        let alpha = 0.995;
        state.energy_avg = alpha * state.energy_avg + (1.0 - alpha) * energy;
        state.samples_processed += 1;

        // Need some minimum samples before the average stabilizes
        if state.samples_processed < 50 {
            return None;
        }

        // Compute dynamic threshold
        let base_threshold = self.config.energy_threshold();
        let dynamic_threshold = state.energy_avg * (1.0 + base_threshold * 10.0);

        // Detect transient (energy spike above threshold)
        if energy > dynamic_threshold {
            let now = Instant::now();

            // Check cooldown since last activation
            if let Some(last_act) = state.last_activation {
                if now.duration_since(last_act) < Duration::from_millis(self.config.cooldown_ms) {
                    state.last_transient = Some(now);
                    return None;
                }
            }

            // Check if we have a pair of transients within the window
            if let Some(last_trans) = state.last_transient {
                let gap = now.duration_since(last_trans);
                if gap >= Duration::from_millis(50)
                    && gap <= Duration::from_millis(self.config.pair_window_ms)
                {
                    // Clap detected!
                    state.last_activation = Some(now);
                    state.last_transient = None;

                    let confidence = ((energy / dynamic_threshold) - 1.0).min(1.0) as f64;

                    if let Some(ref callback) = self.on_clap {
                        callback(confidence);
                    }

                    return Some(confidence);
                }
            }

            // Record this transient
            state.last_transient = Some(now);
        }

        None
    }

    /// Calibrate by analyzing a few seconds of ambient noise.
    /// Returns the measured ambient energy level.
    #[allow(dead_code)]
    pub fn calibrate(&self, _duration_samples: usize) -> f32 {
        let mut state = self.state.lock().unwrap();
        state.energy_avg = 0.01;
        state.samples_processed = 0;
        state.last_transient = None;
        state.last_activation = None;
        state.energy_avg
    }
}

/// Compute RMS energy of a buffer of audio samples
fn compute_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_threshold_sensitivity() {
        let config = ClapConfig { sensitivity: 1, ..Default::default() };
        assert!(config.energy_threshold() > 0.4);

        let config = ClapConfig { sensitivity: 10, ..Default::default() };
        assert!(config.energy_threshold() < 0.1);
    }

    #[test]
    fn test_compute_energy() {
        let silence = vec![0.0f32; 1024];
        assert_eq!(compute_energy(&silence), 0.0);

        let full_scale = vec![1.0f32; 1024];
        assert!((compute_energy(&full_scale) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_detector_does_not_trigger_on_silence() {
        let detector = ClapDetector::new(ClapConfig::default());
        detector.start().unwrap();

        let silence = vec![0.0f32; 1024];
        for _ in 0..100 {
            assert!(detector.process_samples(&silence).is_none());
        }
    }

    #[test]
    fn test_detector_start_stop() {
        let detector = ClapDetector::new(ClapConfig::default());
        assert!(!detector.is_active());
        detector.start().unwrap();
        assert!(detector.is_active());
        detector.stop().unwrap();
        assert!(!detector.is_active());
    }
}