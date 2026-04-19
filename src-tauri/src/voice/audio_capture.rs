use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::{AppHandle, Manager};

use crate::voice::clap_detector::ClapDetector;
use crate::observability::{EventBus, JarvisEvent};

/// Audio capture bridge that connects CPAL input streams to the ClapDetector.
/// When active, captures microphone audio and feeds it to process_samples().
/// When a clap is detected, emits Tauri events and shows the window.
pub struct AudioCapture {
    state: Arc<Mutex<CaptureState>>,
    clap_detector: Arc<Mutex<ClapDetector>>,
    event_bus: Arc<Mutex<EventBus>>,
    app_handle: AppHandle,
}

struct CaptureState {
    stream: Option<cpal::Stream>,
    active: bool,
}

impl AudioCapture {
    pub fn new(
        clap_detector: Arc<Mutex<ClapDetector>>,
        event_bus: Arc<Mutex<EventBus>>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(CaptureState {
                stream: None,
                active: false,
            })),
            clap_detector,
            event_bus,
            app_handle,
        }
    }

    /// Start capturing audio from the default input device.
    pub fn start(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        if state.active {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| "No audio input device available. Check microphone permissions in System Settings > Privacy > Microphone.".to_string())?;

        let config = device.default_input_config()
            .map_err(|e| format!("Failed to get audio config: {}", e))?;

        let stream_config = cpal::StreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let clap_detector = self.clap_detector.clone();
        let event_bus = self.event_bus.clone();
        let app_handle = self.app_handle.clone();

        // Channel for clap detections from audio thread to event thread
        let (tx, rx) = mpsc::sync_channel::<f64>(16);

        // Spawn event-emitting thread
        let event_app = app_handle.clone();
        thread::spawn(move || {
            while let Ok(confidence) = rx.recv() {
                if let Ok(bus) = event_bus.lock() {
                    bus.emit(JarvisEvent::ClapDetected { confidence });
                    bus.emit(JarvisEvent::ActivationTriggered { source: "clap".to_string() });
                    bus.emit(JarvisEvent::VoiceStateChanged { state: "listening".to_string() });
                }
                // Show and focus window on clap
                if let Some(window) = event_app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });

        let err_fn = |err: cpal::StreamError| {
            eprintln!("Audio stream error: {}", err);
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(detector) = clap_detector.lock() {
                            if detector.is_active() {
                                for chunk in data.chunks(1024) {
                                    if let Some(confidence) = detector.process_samples(chunk) {
                                        let _ = tx.try_send(confidence);
                                    }
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let clap_det = clap_detector.clone();
                let tx_i16 = tx.clone();
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(detector) = clap_det.lock() {
                            if detector.is_active() {
                                let float_data: Vec<f32> = data.iter()
                                    .map(|&s| s as f32 / i16::MAX as f32)
                                    .collect();
                                for chunk in float_data.chunks(1024) {
                                    if let Some(confidence) = detector.process_samples(chunk) {
                                        let _ = tx_i16.try_send(confidence);
                                    }
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => {
                return Err("Unsupported audio format. Use a device with f32 or i16 samples.".to_string());
            }
        };

        let stream = stream.map_err(|e| {
            format!("Failed to create audio stream: {}. Check microphone permissions.", e)
        })?;

        stream.play().map_err(|e| format!("Failed to start audio stream: {}", e))?;

        // Set clap detector active
        if let Ok(detector) = self.clap_detector.lock() {
            let _ = detector.start();
        }

        state.stream = Some(stream);
        state.active = true;

        Ok(())
    }

    /// Stop capturing audio.
    pub fn stop(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.stream = None; // Dropping stops the stream
        state.active = false;

        if let Ok(detector) = self.clap_detector.lock() {
            let _ = detector.stop();
        }

        if let Ok(bus) = self.event_bus.lock() {
            bus.emit(JarvisEvent::VoiceStateChanged { state: "idle".to_string() });
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.state.lock().map(|s| s.active).unwrap_or(false)
    }
}