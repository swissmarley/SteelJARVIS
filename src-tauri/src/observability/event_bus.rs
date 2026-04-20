use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
#[allow(dead_code)]
pub enum JarvisEvent {
    StateChanged { from: String, to: String },
    GoalSet { goal: String },
    PlanCreated { steps: Vec<String> },
    StepStarted { index: usize, description: String },
    StepCompleted { index: usize, result: String },
    StepFailed { index: usize, error: String },
    ToolInvoked { tool: String, params: serde_json::Value },
    ToolCompleted { tool: String, result: serde_json::Value },
    MemorySaved { id: String, category: String, preview: String },
    MemoryRetrieved { id: String, query: String },
    PermissionRequested { action: String, details: String },
    PermissionGranted { action: String },
    PermissionDenied { action: String },
    VoiceStateChanged { state: String },
    ActivationTriggered { source: String },
    ClapDetected { confidence: f64 },
    SpeechRecognized { text: String, is_final: bool },
    SpeechPartial { text: String },
    SttError { message: String },
    Error { source: String, message: String },
    ProviderStatusChanged { provider: String, status: String },
    // Full voice round-trip: user said X, JARVIS answered Y. Emitted after the
    // backend handles the recognised speech end-to-end (agent + TTS) so the
    // frontend only has to render the transcript.
    VoiceAgentResponse { user_text: String, assistant_text: String },
    VoiceAgentError { user_text: String, message: String },
    /// Standalone greeting emitted when JARVIS proactively addresses the
    /// user (first of session or after long idle). Rendered as an assistant
    /// message without a paired user message.
    JarvisGreeting { text: String },
}

impl JarvisEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::StateChanged { .. } => "state-changed",
            Self::GoalSet { .. } => "goal-set",
            Self::PlanCreated { .. } => "plan-created",
            Self::StepStarted { .. } => "step-started",
            Self::StepCompleted { .. } => "step-completed",
            Self::StepFailed { .. } => "step-failed",
            Self::ToolInvoked { .. } => "tool-invoked",
            Self::ToolCompleted { .. } => "tool-completed",
            Self::MemorySaved { .. } => "memory-saved",
            Self::MemoryRetrieved { .. } => "memory-retrieved",
            Self::PermissionRequested { .. } => "permission-requested",
            Self::PermissionGranted { .. } => "permission-granted",
            Self::PermissionDenied { .. } => "permission-denied",
            Self::VoiceStateChanged { .. } => "voice-state-changed",
            Self::ActivationTriggered { .. } => "activation-triggered",
            Self::ClapDetected { .. } => "clap-detected",
            Self::SpeechRecognized { .. } => "speech-recognized",
            Self::SpeechPartial { .. } => "speech-partial",
            Self::SttError { .. } => "stt-error",
            Self::Error { .. } => "error",
            Self::ProviderStatusChanged { .. } => "system-health",
            Self::VoiceAgentResponse { .. } => "voice-agent-response",
            Self::VoiceAgentError { .. } => "voice-agent-error",
            Self::JarvisGreeting { .. } => "jarvis-greeting",
        }
    }

    pub fn payload(&self) -> serde_json::Value {
        match self {
            Self::StateChanged { from, to } => serde_json::json!({"from": from, "to": to}),
            Self::GoalSet { goal } => serde_json::json!({"goal": goal}),
            Self::PlanCreated { steps } => serde_json::json!({"steps": steps}),
            Self::StepStarted { index, description } => serde_json::json!({"index": index, "description": description}),
            Self::StepCompleted { index, result } => serde_json::json!({"index": index, "result": result}),
            Self::StepFailed { index, error } => serde_json::json!({"index": index, "error": error}),
            Self::ToolInvoked { tool, params } => serde_json::json!({"tool": tool, "params": params}),
            Self::ToolCompleted { tool, result } => serde_json::json!({"tool": tool, "result": result}),
            Self::MemorySaved { id, category, preview } => serde_json::json!({"id": id, "category": category, "preview": preview}),
            Self::MemoryRetrieved { id, query } => serde_json::json!({"id": id, "query": query}),
            Self::PermissionRequested { action, details } => serde_json::json!({"action": action, "details": details}),
            Self::PermissionGranted { action } => serde_json::json!({"action": action}),
            Self::PermissionDenied { action } => serde_json::json!({"action": action}),
            Self::VoiceStateChanged { state } => serde_json::json!({"state": state}),
            Self::ActivationTriggered { source } => serde_json::json!({"source": source}),
            Self::ClapDetected { confidence } => serde_json::json!({"confidence": confidence}),
            Self::SpeechRecognized { text, is_final } => serde_json::json!({"text": text, "is_final": is_final}),
            Self::SpeechPartial { text } => serde_json::json!({"text": text}),
            Self::SttError { message } => serde_json::json!({"message": message}),
            Self::Error { source, message } => serde_json::json!({"source": source, "message": message}),
            Self::ProviderStatusChanged { provider, status } => serde_json::json!({"provider": provider, "status": status}),
            Self::VoiceAgentResponse { user_text, assistant_text } => serde_json::json!({"userText": user_text, "assistantText": assistant_text}),
            Self::VoiceAgentError { user_text, message } => serde_json::json!({"userText": user_text, "message": message}),
            Self::JarvisGreeting { text } => serde_json::json!({"text": text}),
        }
    }
}

pub struct EventBus {
    sender: broadcast::Sender<JarvisEvent>,
    app: Option<AppHandle>,
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            app: self.app.clone(),
        }
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender, app: None }
    }

    pub fn set_app_handle(&mut self, app: AppHandle) {
        self.app = Some(app);
    }

    pub fn emit(&self, event: JarvisEvent) {
        let _ = self.sender.send(event.clone());
        if let Some(app) = &self.app {
            let _ = app.emit(event.event_name(), event.payload());
        }
    }

    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<JarvisEvent> {
        self.sender.subscribe()
    }
}