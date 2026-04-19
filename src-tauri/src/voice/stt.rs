use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tauri::AppHandle;

use crate::observability::{EventBus, JarvisEvent};

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

        // Spawn event-emitting thread (same pattern as clap detection)
        let event_bus = self.event_bus.clone();
        let _event_app = self.app_handle.clone();
        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                if let Ok(bus) = event_bus.lock() {
                    match event {
                        SpeechEvent::Partial { text } => {
                            eprintln!("[STT] event thread: emitting SpeechPartial");
                            bus.emit(JarvisEvent::SpeechPartial { text });
                        }
                        SpeechEvent::Final { text } => {
                            eprintln!("[STT] event thread: emitting SpeechRecognized");
                            bus.emit(JarvisEvent::SpeechRecognized { text, is_final: true });
                        }
                        SpeechEvent::Error { message } => {
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