use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tauri::{AppHandle, Manager};

use crate::agent::AgentEngine;
use crate::observability::{EventBus, JarvisEvent};
use crate::voice::SpeechManager;

// FFI declarations for Swift SpeechRecognizer
extern "C" {
    fn speech_recognizer_create() -> *mut std::ffi::c_void;
    fn speech_recognizer_start(
        ptr: *mut std::ffi::c_void,
        on_result: extern "C" fn(*const c_char, Bool),
        on_error: extern "C" fn(*const c_char),
    );
    fn speech_recognizer_stop(ptr: *mut std::ffi::c_void);
    fn speech_recognizer_destroy(ptr: *mut std::ffi::c_void);
}

// C bool type for FFI
type Bool = bool;

// Speech event data sent from FFI callbacks to the event-emitting thread
enum SpeechEvent {
    Partial { text: String },
    Final { text: String },
    Error { message: String },
}

// Global sender for speech events — set once during start, cleared on stop
static SPEECH_TX: std::sync::OnceLock<Mutex<Option<mpsc::SyncSender<SpeechEvent>>>> = std::sync::OnceLock::new();

fn send_speech_event(event: SpeechEvent) {
    if let Some(lock) = SPEECH_TX.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.try_send(event);
            }
        }
    }
}

// Callback invoked by Swift when speech is recognized
extern "C" fn on_speech_result(text_ptr: *const c_char, is_final: Bool) {
    if text_ptr.is_null() {
        eprintln!("[STT] on_speech_result called with null text_ptr");
        return;
    }
    let text = unsafe { CStr::from_ptr(text_ptr) }
        .to_string_lossy()
        .to_string();

    eprintln!("[STT] callback: is_final={}, text_len={}, text={:?}", is_final, text.len(), &text[..text.len().min(80)]);

    if is_final {
        eprintln!("[STT] sending Final event through channel");
        send_speech_event(SpeechEvent::Final { text });
    } else {
        eprintln!("[STT] sending Partial event through channel");
        send_speech_event(SpeechEvent::Partial { text });
    }
}

// Callback invoked by Swift on error
extern "C" fn on_speech_error(msg_ptr: *const c_char) {
    if msg_ptr.is_null() {
        eprintln!("[STT] on_speech_error called with null msg_ptr");
        return;
    }
    let message = unsafe { CStr::from_ptr(msg_ptr) }
        .to_string_lossy()
        .to_string();

    eprintln!("[STT] ERROR from Swift: {}", message);
    send_speech_event(SpeechEvent::Error { message });
}

/// Manages the native macOS speech recognizer via FFI.
pub struct SpeechRecognizer {
    ptr: *mut std::ffi::c_void,
    event_bus: Arc<Mutex<EventBus>>,
    app_handle: AppHandle,
}

// Safety: The pointer is managed by this struct and only used on the main thread.
// The Swift object is thread-safe (dispatched to main queue).
unsafe impl Send for SpeechRecognizer {}
unsafe impl Sync for SpeechRecognizer {}

impl SpeechRecognizer {
    pub fn new(event_bus: Arc<Mutex<EventBus>>, app_handle: AppHandle) -> Result<Self, String> {
        let ptr = unsafe { speech_recognizer_create() };
        if ptr.is_null() {
            return Err("Failed to create speech recognizer. Speech framework may not be available.".to_string());
        }

        Ok(Self {
            ptr,
            event_bus,
            app_handle,
        })
    }

    /// Start listening for speech input.
    pub fn start(&mut self) -> Result<(), String> {
        if self.ptr.is_null() {
            eprintln!("[STT] start called but pointer is null");
            return Err("Speech recognizer not initialized".to_string());
        }

        // Create channel for speech events (same pattern as clap detection)
        let (tx, rx) = mpsc::sync_channel::<SpeechEvent>(64);

        // Store sender globally for FFI callbacks
        {
            let lock = SPEECH_TX.get_or_init(|| Mutex::new(None));
            if let Ok(mut guard) = lock.lock() {
                *guard = Some(tx);
            }
        }

        // Spawn event-emitting thread. On Final we drive the agent directly from
        // the backend — the frontend no longer has to dispatch; it only renders
        // the resulting transcript via the voice-agent-response event.
        let event_bus = self.event_bus.clone();
        let event_app = self.app_handle.clone();
        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                match event {
                    SpeechEvent::Partial { text } => {
                        // STT is muted while TTS plays (see SpeechManager::speak_async),
                        // so any partial we see here is genuinely the user — no
                        // echo filtering needed.
                        if let Ok(bus) = event_bus.lock() {
                            bus.emit(JarvisEvent::SpeechPartial { text });
                        }
                    }
                    SpeechEvent::Final { text } => {
                        if let Ok(bus) = event_bus.lock() {
                            eprintln!("[STT] event thread: emitting SpeechRecognized");
                            bus.emit(JarvisEvent::SpeechRecognized { text: text.clone(), is_final: true });
                        }
                        dispatch_to_agent(event_app.clone(), event_bus.clone(), text);
                    }
                    SpeechEvent::Error { message } => {
                        if let Ok(bus) = event_bus.lock() {
                            eprintln!("[STT] event thread: emitting SttError");
                            bus.emit(JarvisEvent::SttError { message });
                        }
                    }
                }
            }
            eprintln!("[STT] event thread: channel closed, exiting");
        });

        if let Ok(bus) = self.event_bus.lock() {
            bus.emit(JarvisEvent::VoiceStateChanged { state: "listening".to_string() });
        }

        unsafe {
            speech_recognizer_start(self.ptr, on_speech_result, on_speech_error);
        }

        eprintln!("[STT] FFI speech_recognizer_start called");
        Ok(())
    }

    /// Stop listening for speech input.
    pub fn stop(&mut self) -> Result<(), String> {
        if self.ptr.is_null() {
            return Err("Speech recognizer not initialized".to_string());
        }

        eprintln!("[STT] stop: calling FFI");
        unsafe {
            speech_recognizer_stop(self.ptr);
        }

        // Clear the channel sender so the event thread exits
        if let Some(lock) = SPEECH_TX.get() {
            if let Ok(mut guard) = lock.lock() {
                *guard = None;
            }
        }

        if let Ok(bus) = self.event_bus.lock() {
            bus.emit(JarvisEvent::VoiceStateChanged { state: "idle".to_string() });
        }

        eprintln!("[STT] stopped");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_listening(&self) -> bool {
        false
    }
}

impl Drop for SpeechRecognizer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                speech_recognizer_destroy(self.ptr);
            }
        }
    }
}

/// Runs the recognised utterance through the agent and announces the response
/// via TTS. Emits `voice-agent-response` (or `voice-agent-error`) when done.
/// Invoked from the STT event thread on `is_final=true`.
fn dispatch_to_agent(app: AppHandle, event_bus: Arc<Mutex<EventBus>>, user_text: String) {
    let trimmed = user_text.trim().to_string();
    if trimmed.is_empty() {
        eprintln!("[STT→Agent] empty utterance, skipping");
        return;
    }
    eprintln!("[STT→Agent] dispatching {:?} to agent", trimmed);

    tauri::async_runtime::spawn(async move {
        let (api_key, history) = {
            let engine_state = app.state::<Mutex<AgentEngine>>();
            let engine = match engine_state.lock() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[STT→Agent] AgentEngine lock failed: {}", e);
                    return;
                }
            };
            (engine.api_key().to_string(), engine.history().to_vec())
        };

        if api_key.is_empty() {
            eprintln!("[STT→Agent] ERROR: ANTHROPIC_API_KEY is empty");
            if let Ok(bus) = event_bus.lock() {
                bus.emit(JarvisEvent::VoiceAgentError {
                    user_text: trimmed.clone(),
                    message: "ANTHROPIC_API_KEY is not configured. Add it to .env and restart.".to_string(),
                });
            }
            return;
        }

        let bus_snapshot = match event_bus.lock() {
            Ok(b) => b.clone(),
            Err(e) => {
                eprintln!("[STT→Agent] EventBus lock failed: {}", e);
                return;
            }
        };

        let result = AgentEngine::send_with(&api_key, &history, &trimmed, &bus_snapshot).await;

        match result {
            Ok((response, new_history)) => {
                eprintln!("[STT→Agent] agent replied ({} chars)", response.len());

                if let Ok(mut engine) = app.state::<Mutex<AgentEngine>>().lock() {
                    engine.set_history(new_history);
                }

                if let Ok(bus) = event_bus.lock() {
                    bus.emit(JarvisEvent::VoiceAgentResponse {
                        user_text: trimmed.clone(),
                        assistant_text: response.clone(),
                    });
                }

                // Speak the response so JARVIS answers even if the UI is hidden.
                // speak_async mutes STT for the duration to prevent self-hearing.
                if let Ok(speech) = app.state::<Mutex<SpeechManager>>().lock() {
                    if let Err(e) = speech.speak_async(&response, &app) {
                        eprintln!("[STT→Agent] TTS failed: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[STT→Agent] agent error: {}", e);
                if let Ok(bus) = event_bus.lock() {
                    bus.emit(JarvisEvent::VoiceAgentError {
                        user_text: trimmed.clone(),
                        message: e,
                    });
                }
            }
        }
    });
}