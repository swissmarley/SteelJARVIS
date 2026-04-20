import { useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useConversationStore } from './stores/conversation';
import { useDashboardStore } from './stores/dashboard';
import { useVoiceStore } from './stores/voice';
import { useTauriEvent } from './hooks/useTauriEvent';
import AppShell from './components/AppShell/AppShell';

// Pipes UI-side events to the Rust stderr so the user can trace behaviour in
// the same terminal as the `[Chat]`/`[STT]` logs without opening devtools.
const uiLog = (tag: string, message: string) => {
  console.log(`[${tag}] ${message}`);
  invoke('log_debug', { tag, message }).catch(() => { /* ignore */ });
};

export default function App() {
  const addMessage = useConversationStore((s) => s.addMessage);
  const appendCurrentResponse = useConversationStore((s) => s.appendCurrentResponse);
  const setStreaming = useConversationStore((s) => s.setStreaming);
  const setError = useConversationStore((s) => s.setError);

  const setExecutionMode = useDashboardStore((s) => s.setExecutionMode);
  const setVoiceState = useDashboardStore((s) => s.setVoiceState);
  const addActionEvent = useDashboardStore((s) => s.addActionEvent);
  const addError = useDashboardStore((s) => s.addError);
  const addMemoryEvent = useDashboardStore((s) => s.addMemoryEvent);
  const updateSystemHealth = useDashboardStore((s) => s.updateSystemHealth);
  const addToolInUse = useDashboardStore((s) => s.addToolInUse);

  const handleStateChanged = useCallback(
    (payload: { from: string; to: string }) => {
      setExecutionMode(payload.to as any);
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'StateChanged',
        description: `${payload.from} → ${payload.to}`,
        timestamp: Date.now(),
      });
    },
    [setExecutionMode, addActionEvent]
  );

  const handleVoiceStateChanged = useCallback(
    (payload: { state: string }) => {
      console.log('[STT] voice-state-changed:', payload.state);
      setVoiceState(payload.state as any);
      const voiceStore = useVoiceStore.getState();
      if (payload.state === 'idle') {
        voiceStore.setListeningForSpeech(false);
        voiceStore.setPartialTranscript('');
        voiceStore.setSpeaking(false);
      } else if (payload.state === 'listening') {
        voiceStore.setListeningForSpeech(true);
      } else if (payload.state === 'speaking') {
        voiceStore.setSpeaking(true);
      }
    },
    [setVoiceState]
  );

  const handleError = useCallback(
    (payload: { source: string; message: string }) => {
      addError(payload.source, payload.message);
    },
    [addError]
  );

  const handleSystemHealth = useCallback(
    (payload: Record<string, unknown>) => {
      updateSystemHealth(payload as any);
    },
    [updateSystemHealth]
  );

  const handleMemorySaved = useCallback(
    (payload: { id: string; category: string; preview: string }) => {
      addMemoryEvent({ type: 'save', memoryId: payload.id, timestamp: Date.now(), preview: payload.preview });
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'MemorySaved',
        description: `[${payload.category}] ${payload.preview}`,
        timestamp: Date.now(),
      });
    },
    [addMemoryEvent, addActionEvent]
  );

  const handleToolInvoked = useCallback(
    (payload: { tool: string; params: Record<string, unknown> }) => {
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'ToolInvoked',
        description: `${payload.tool}(${JSON.stringify(payload.params).slice(0, 80)})`,
        timestamp: Date.now(),
      });
    },
    [addActionEvent]
  );

  const handleToolCompleted = useCallback(
    (payload: { tool: string; result: unknown }) => {
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'ToolCompleted',
        description: `${payload.tool}: ${String(payload.result).slice(0, 100)}`,
        timestamp: Date.now(),
      });
    },
    [addActionEvent]
  );

  const handleActivationTriggered = useCallback(
    (payload: { source: string }) => {
      useDashboardStore.getState().setActivationSource(payload.source as any);
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'ActivationTriggered',
        description: `Activated via ${payload.source}`,
        timestamp: Date.now(),
      });
    },
    [addActionEvent]
  );

  // Clap detected: auto-start speech recognition
  const handleClapDetected = useCallback(
    (payload: { confidence: number }) => {
      uiLog('Clap', `clap-detected, starting listening (conf=${payload.confidence.toFixed(2)})`);
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'ClapDetected',
        description: `Clap detected (confidence: ${payload.confidence.toFixed(2)})`,
        timestamp: Date.now(),
      });
      // Auto-start listening when JARVIS appears so the user can speak immediately.
      useVoiceStore.getState().startListening();
    },
    [addActionEvent]
  );

  // The backend now owns the speech→agent lifecycle. These refs are only here
  // so existing handlers can clear any timers left over from earlier paths.
  const speechDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const speechMaxTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearSpeechTimers = () => {
    if (speechDebounceRef.current) {
      clearTimeout(speechDebounceRef.current);
      speechDebounceRef.current = null;
    }
    if (speechMaxTimerRef.current) {
      clearTimeout(speechMaxTimerRef.current);
      speechMaxTimerRef.current = null;
    }
  };

  // Speech recognized (final): clear any pending UI timers. The backend now
  // runs the full STT→agent→TTS loop and will emit voice-agent-response; we
  // no longer dispatch send_message from here.
  const handleSpeechRecognized = useCallback(
    (payload: { text: string; is_final: boolean }) => {
      uiLog('STT', `speech-recognized: is_final=${payload.is_final}, text=${JSON.stringify(payload.text)}`);
      if (payload.is_final) {
        clearSpeechTimers();
        setStreaming(true);
        setExecutionMode('acting');
      }
    },
    [setStreaming, setExecutionMode]
  );

  // Partial speech result: just update the live transcript preview. The
  // backend drives finalization via the Swift silence timer → is_final event,
  // so no frontend debounce is required.
  const handleSpeechPartial = useCallback(
    (payload: { text: string }) => {
      uiLog('STT', `speech-partial: ${JSON.stringify(payload.text)}`);
      const text = (payload.text ?? '').trim();
      if (!text || text.startsWith('[')) return;
      useVoiceStore.getState().setPartialTranscript(text);
    },
    []
  );

  // STT error — surface it, then stop listening so CPAL/clap detection resumes.
  const handleSttError = useCallback(
    (payload: { message: string }) => {
      uiLog('STT', `stt-error: ${payload.message}`);
      addError('stt', payload.message);
      useVoiceStore.getState().setSttError(payload.message);
      clearSpeechTimers();
      useVoiceStore.getState().stopListening();
    },
    [addError]
  );

  // Backend-driven voice round-trip: Rust ran STT→agent→TTS and is handing us
  // the transcript to render. No invoke/send_message call needed here.
  const handleVoiceAgentResponse = useCallback(
    (payload: { userText: string; assistantText: string }) => {
      uiLog('Voice', `voice-agent-response: user=${JSON.stringify(payload.userText)}, assistant len=${payload.assistantText.length}`);
      clearSpeechTimers();
      // Keep STT running so the user can follow up (or barge-in) without
      // having to clap / toggle listen again. The backend recognizer loops
      // automatically after each final.
      useVoiceStore.getState().setPartialTranscript('');
      const now = Date.now();
      addMessage({
        id: crypto.randomUUID(),
        role: 'user',
        content: payload.userText,
        timestamp: now,
      });
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: payload.assistantText,
        timestamp: now,
      });
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'UserSaid',
        description: payload.userText,
        timestamp: now,
      });
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'JarvisSaid',
        description: payload.assistantText.slice(0, 160),
        timestamp: now,
      });
      setStreaming(false);
      setExecutionMode('idle');
    },
    [addMessage, addActionEvent, setStreaming, setExecutionMode]
  );

  const handleVoiceAgentError = useCallback(
    (payload: { userText: string; message: string }) => {
      uiLog('Voice', `voice-agent-error: ${payload.message}`);
      clearSpeechTimers();
      useVoiceStore.getState().setPartialTranscript('');
      const now = Date.now();
      addMessage({
        id: crypto.randomUUID(),
        role: 'user',
        content: payload.userText,
        timestamp: now,
      });
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `I encountered an error: ${payload.message}`,
        timestamp: now,
      });
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'UserSaid',
        description: payload.userText,
        timestamp: now,
      });
      addError('agent', payload.message);
      setStreaming(false);
      setExecutionMode('idle');
    },
    [addMessage, addActionEvent, addError, setStreaming, setExecutionMode]
  );

  const handleJarvisGreeting = useCallback(
    (payload: { text: string }) => {
      uiLog('Voice', `jarvis-greeting: len=${payload.text.length}`);
      const now = Date.now();
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: payload.text,
        timestamp: now,
      });
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'JarvisGreeted',
        description: payload.text.slice(0, 160),
        timestamp: now,
      });
    },
    [addMessage, addActionEvent]
  );

  useTauriEvent('state-changed', handleStateChanged);
  useTauriEvent('voice-state-changed', handleVoiceStateChanged);
  useTauriEvent('error', handleError);
  useTauriEvent('system-health', handleSystemHealth);
  useTauriEvent('memory-saved', handleMemorySaved);
  useTauriEvent('tool-invoked', handleToolInvoked);
  useTauriEvent('tool-completed', handleToolCompleted);
  useTauriEvent('activation-triggered', handleActivationTriggered);
  useTauriEvent('clap-detected', handleClapDetected);
  useTauriEvent('speech-recognized', handleSpeechRecognized);
  useTauriEvent('speech-partial', handleSpeechPartial);
  useTauriEvent('stt-error', handleSttError);
  useTauriEvent('voice-agent-response', handleVoiceAgentResponse);
  useTauriEvent('voice-agent-error', handleVoiceAgentError);
  useTauriEvent('jarvis-greeting', handleJarvisGreeting);

  useTauriEvent('toggle-clap-from-tray', useCallback(() => {
    const { isListening, startClapDetection, stopClapDetection } = useVoiceStore.getState();
    if (isListening) {
      stopClapDetection();
    } else {
      startClapDetection();
    }
  }, []));

  // Stop speech recognition when window is hidden
  useTauriEvent('window-hidden', useCallback(() => {
    useVoiceStore.getState().stopListening();
  }, []));

  useEffect(() => {
    setExecutionMode('idle');

    // Delay auto-starting mic services so the window is fully mounted and any
    // first-run macOS microphone / speech-recognition permission prompts have
    // a chance to resolve before CPAL and AVAudioEngine open the input device.
    const clapBootTimer = setTimeout(() => {
      const voice = useVoiceStore.getState();
      voice.startClapDetection().catch((err) => {
        console.warn('[Voice] auto-start clap detection failed:', err);
      });
      // Start STT immediately so the user can speak without manually toggling
      // the mic. STT is automatically paused while JARVIS speaks and resumed
      // after TTS finishes.
      voice.startListening().catch((err) => {
        console.warn('[Voice] auto-start listening failed:', err);
      });
    }, 1500);

    invoke('check_health')
      .then((health: any) => {
        updateSystemHealth(health);
      })
      .catch(() => {
        updateSystemHealth({ providerConnected: false, dbConnected: false, audioAvailable: false });
      });

    return () => clearTimeout(clapBootTimer);
  }, []);

  return <AppShell />;
}