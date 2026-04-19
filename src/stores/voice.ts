import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { invoke } from '@tauri-apps/api/core';

interface VoiceState {
  isListening: boolean;
  isListeningForSpeech: boolean;
  isSpeaking: boolean;
  sttError: string | null;
  currentVoice: string;
  rate: number;
  availableVoices: string[];
  clapEnabled: boolean;
  clapSensitivity: number;
  activationMethod: string;
  voiceEnabled: boolean;
  partialTranscript: string;
}

interface VoiceActions {
  speak: (text: string) => Promise<void>;
  stopSpeaking: () => Promise<void>;
  setVoice: (name: string) => Promise<void>;
  setRate: (rate: number) => Promise<void>;
  loadVoices: () => Promise<void>;
  loadConfig: () => Promise<void>;
  setClapEnabled: (enabled: boolean) => Promise<void>;
  setClapSensitivity: (sensitivity: number) => Promise<void>;
  startClapDetection: () => Promise<void>;
  stopClapDetection: () => Promise<void>;
  startListening: () => Promise<void>;
  stopListening: () => Promise<void>;
  setListening: (listening: boolean) => void;
  setListeningForSpeech: (listening: boolean) => void;
  setSpeaking: (speaking: boolean) => void;
  setVoiceEnabled: (enabled: boolean) => void;
  setPartialTranscript: (text: string) => void;
  setSttError: (error: string | null) => void;
}

export const useVoiceStore = create<VoiceState & VoiceActions>()(
  immer((set, get) => ({
    isListening: false,
    isListeningForSpeech: false,
    isSpeaking: false,
    sttError: null,
    currentVoice: 'Samantha',
    rate: 200,
    availableVoices: [],
    clapEnabled: false,
    clapSensitivity: 5,
    activationMethod: 'hotkey',
    voiceEnabled: true,
    partialTranscript: '',

    speak: async (text) => {
      if (!text.trim()) return;
      set((s) => { s.isSpeaking = true; });
      try {
        await invoke('speak', { text });
        const duration = Math.max(1000, text.length * 60);
        setTimeout(() => set((s) => { s.isSpeaking = false; }), duration);
      } catch (e) {
        console.error('TTS failed:', e);
        set((s) => { s.isSpeaking = false; });
      }
    },

    stopSpeaking: async () => {
      try {
        await invoke('stop_speaking');
      } finally {
        set((s) => { s.isSpeaking = false; });
      }
    },

    setVoice: async (name) => {
      await invoke('set_voice', { voiceName: name });
      set((s) => { s.currentVoice = name; });
    },

    setRate: async (rate) => {
      await invoke('set_speech_rate', { rate });
      set((s) => { s.rate = rate; });
    },

    loadVoices: async () => {
      try {
        const voices = await invoke<string[]>('list_voices');
        set((s) => { s.availableVoices = voices; });
      } catch { /* ignore */ }
    },

    loadConfig: async () => {
      try {
        const config = await invoke<Record<string, unknown>>('get_voice_config');
        set((s) => {
          if (config.voice) s.currentVoice = config.voice as string;
          if (config.rate) s.rate = config.rate as number;
        });
      } catch { /* ignore */ }
    },

    setClapEnabled: async (enabled) => {
      await invoke('set_clap_enabled', { enabled });
      set((s) => { s.clapEnabled = enabled; });
    },

    setClapSensitivity: async (sensitivity) => {
      await invoke('set_clap_sensitivity', { sensitivity });
      set((s) => { s.clapSensitivity = sensitivity; });
    },

    startClapDetection: async () => {
      try {
        await invoke('start_clap_detection');
        set((s) => { s.isListening = true; });
      } catch (e) {
        console.error('Failed to start clap detection:', e);
      }
    },

    stopClapDetection: async () => {
      try {
        await invoke('stop_clap_detection');
        set((s) => { s.isListening = false; });
      } catch (e) {
        console.error('Failed to stop clap detection:', e);
      }
    },

    startListening: async () => {
      console.log('[Voice] startListening called');
      try {
        await invoke('start_listening');
        console.log('[Voice] start_listening invoke succeeded');
        set((s) => {
          s.isListeningForSpeech = true;
          s.sttError = null;
        });
      } catch (e) {
        console.error('[Voice] Failed to start speech recognition:', e);
        set((s) => { s.sttError = String(e); });
      }
    },

    stopListening: async () => {
      try {
        await invoke('stop_listening');
        set((s) => {
          s.isListeningForSpeech = false;
          s.partialTranscript = '';
          s.sttError = null;
        });
      } catch (e) {
        console.error('Failed to stop speech recognition:', e);
      }
    },

    setListening: (listening) => {
      set((s) => { s.isListening = listening; });
    },

    setListeningForSpeech: (listening) => {
      set((s) => { s.isListeningForSpeech = listening; });
    },

    setSpeaking: (speaking) => {
      set((s) => { s.isSpeaking = speaking; });
    },

    setVoiceEnabled: (enabled) => {
      set((s) => { s.voiceEnabled = enabled; });
    },

    setPartialTranscript: (text) => {
      set((s) => { s.partialTranscript = text; });
    },

    setSttError: (error) => {
      set((s) => { s.sttError = error; });
    },
  }))
);