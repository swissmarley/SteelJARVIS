import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import type { Message, ConversationState } from '../types/messages';

interface ConversationActions {
  addMessage: (message: Message) => void;
  updateMessageContent: (id: string, content: string) => void;
  setStreaming: (streaming: boolean) => void;
  setCurrentResponse: (response: string) => void;
  appendCurrentResponse: (chunk: string) => void;
  finalizeStreaming: (id: string) => void;
  setError: (error: string | null) => void;
  clearConversation: () => void;
}

export const useConversationStore = create<ConversationState & ConversationActions>()(
  immer((set) => ({
    messages: [],
    isStreaming: false,
    currentResponse: '',
    error: null,

    addMessage: (message) =>
      set((state) => {
        state.messages.push(message);
      }),

    updateMessageContent: (id, content) =>
      set((state) => {
        const msg = state.messages.find((m) => m.id === id);
        if (msg) msg.content = content;
      }),

    setStreaming: (streaming) =>
      set((state) => {
        state.isStreaming = streaming;
        if (!streaming) state.currentResponse = '';
      }),

    setCurrentResponse: (response) =>
      set((state) => {
        state.currentResponse = response;
      }),

    appendCurrentResponse: (chunk) =>
      set((state) => {
        state.currentResponse += chunk;
      }),

    finalizeStreaming: (id) =>
      set((state) => {
        const msg = state.messages.find((m) => m.id === id);
        if (msg) {
          msg.content = state.currentResponse;
          msg.isStreaming = false;
        }
        state.isStreaming = false;
        state.currentResponse = '';
      }),

    setError: (error) =>
      set((state) => {
        state.error = error;
      }),

    clearConversation: () =>
      set((state) => {
        state.messages = [];
        state.currentResponse = '';
        state.isStreaming = false;
        state.error = null;
      }),
  }))
);