import { useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useConversationStore } from './stores/conversation';
import { useDashboardStore } from './stores/dashboard';
import { useVoiceStore } from './stores/voice';
import { useTauriEvent } from './hooks/useTauriEvent';
import AppShell from './components/AppShell/AppShell';

const SPEECH_DEBOUNCE_MS = 2000;

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
      console.log('[STT] clap-detected, starting listening');
      addActionEvent({
        id: crypto.randomUUID(),
        type: 'ClapDetected',
        description: `Clap detected (confidence: ${payload.confidence.toFixed(2)})`,
        timestamp: Date.now(),
      });
      // Auto-start listening when clap is detected
      useVoiceStore.getState().startListening();
    },
    [addActionEvent]
  );

  // Debounce timer for auto-sending when partial results stop
  const speechDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Send a voice message to the agent
  const sendVoiceMessage = useCallback(
    (text: string) => {
      if (!text.trim()) return;
      // Clear debounce timer
      if (speechDebounceRef.current) {
        clearTimeout(speechDebounceRef.current);
        speechDebounceRef.current = null;
      }
      // Stop listening while agent processes
      useVoiceStore.getState().stopListening();

      const userMessage = {
        id: crypto.randomUUID(),
        role: 'user' as const,
        content: text.trim(),
        timestamp: Date.now(),
      };
      addMessage(userMessage);
      setStreaming(true);
      setExecutionMode('acting');

      invoke<string>('send_message', { message: text.trim() })
        .then((response) => {
          addMessage({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: response,
            timestamp: Date.now(),
          });
          if (useVoiceStore.getState().voiceEnabled) {
            useVoiceStore.getState().speak(response);
            const duration = Math.max(1000, response.length * 60);
            setTimeout(() => {
              useVoiceStore.getState().startListening();
            }, duration + 500);
          }
        })
        .catch((err) => {
          setError(String(err));
          addMessage({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: `I encountered an error: ${err}. Please check your API key configuration.`,
            timestamp: Date.now(),
          });
        })
        .finally(() => {
          setStreaming(false);
          setExecutionMode('idle');
        });
    },
    [addMessage, setStreaming, setError, setExecutionMode]
  );

  // Speech recognized (final result): cancel debounce and send immediately
  const handleSpeechRecognized = useCallback(
    (payload: { text: string; is_final: boolean }) => {
      console.log('[STT] speech-recognized event:', payload);
      if (payload.is_final && payload.text.trim()) {
        sendVoiceMessage(payload.text);
      }
    },
    [sendVoiceMessage]
  );

  // Partial speech result: update transcript and reset debounce timer.
  // If no new partial arrives within SPEECH_DEBOUNCE_MS, auto-send.
  const handleSpeechPartial = useCallback(
    (payload: { text: string }) => {
      console.log('[STT] speech-partial event:', payload);
      useVoiceStore.getState().setPartialTranscript(payload.text);

      if (speechDebounceRef.current) {
        clearTimeout(speechDebounceRef.current);
      }
      speechDebounceRef.current = setTimeout(() => {
        const text = useVoiceStore.getState().partialTranscript;
        if (text.trim()) {
          sendVoiceMessage(text);
        }
      }, SPEECH_DEBOUNCE_MS);
    },
    [sendVoiceMessage]
  );

  // STT error
  const handleSttError = useCallback(
    (payload: { message: string }) => {
      console.log('[STT] stt-error event:', payload);
      addError('stt', payload.message);
      useVoiceStore.getState().setSttError(payload.message);
    },
    [addError]
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

    invoke('check_health')
      .then((health: any) => {
        updateSystemHealth(health);
      })
      .catch(() => {
        updateSystemHealth({ providerConnected: false, dbConnected: false, audioAvailable: false });
      });
  }, []);

  return <AppShell />;
}