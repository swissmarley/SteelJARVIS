import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useConversationStore } from '../../stores/conversation';
import { useDashboardStore } from '../../stores/dashboard';
import { useVoiceStore } from '../../stores/voice';
import ListeningAnimation from '../ListeningAnimation/ListeningAnimation';
import styles from './ChatPanel.module.css';

export default function ChatPanel({ className }: { className?: string }) {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const messages = useConversationStore((s) => s.messages);
  const isStreaming = useConversationStore((s) => s.isStreaming);
  const currentResponse = useConversationStore((s) => s.currentResponse);
  const addMessage = useConversationStore((s) => s.addMessage);
  const setStreaming = useConversationStore((s) => s.setStreaming);
  const setError = useConversationStore((s) => s.setError);
  const setExecutionMode = useDashboardStore((s) => s.setExecutionMode);

  const speak = useVoiceStore((s) => s.speak);
  const isSpeaking = useVoiceStore((s) => s.isSpeaking);
  const voiceEnabled = useVoiceStore((s) => s.voiceEnabled);
  const isListeningForSpeech = useVoiceStore((s) => s.isListeningForSpeech);
  const partialTranscript = useVoiceStore((s) => s.partialTranscript);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, currentResponse]);

  const handleSend = useCallback(async () => {
    const text = input.trim();
    if (!text || isStreaming) return;

    const userMessage = {
      id: crypto.randomUUID(),
      role: 'user' as const,
      content: text,
      timestamp: Date.now(),
    };
    addMessage(userMessage);
    setInput('');
    setStreaming(true);
    setExecutionMode('acting');

    try {
      const response = await invoke<string>('send_message', { message: text });
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: response,
        timestamp: Date.now(),
      });
      if (voiceEnabled) {
        speak(response);
      }
    } catch (err) {
      setError(String(err));
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `I encountered an error: ${err}. Please check your API key configuration.`,
        timestamp: Date.now(),
      });
    } finally {
      setStreaming(false);
      setExecutionMode('idle');
    }
  }, [input, isStreaming, addMessage, setStreaming, setError, setExecutionMode, voiceEnabled, speak]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleSpeak = useCallback((text: string) => {
    speak(text);
  }, [speak]);

  return (
    <div className={`${styles.panel} ${className || ''}`}>
      <ListeningAnimation />
      <div className={styles.messages}>
        {messages.map((msg) => (
          <div key={msg.id} className={`${styles.message} ${styles[msg.role]}`}>
            <div className={styles.messageHeader}>
              <div className={styles.roleLabel}>
                {msg.role === 'user' ? 'You' : 'JARVIS'}
              </div>
              {msg.role === 'assistant' && (
                <button
                  className={styles.speakBtn}
                  onClick={() => handleSpeak(msg.content)}
                  title="Read aloud"
                  disabled={isSpeaking}
                >
                  {isSpeaking ? '...' : 'speak'}
                </button>
              )}
            </div>
            <div className={styles.content}>{msg.content}</div>
          </div>
        ))}
        {isStreaming && currentResponse && (
          <div className={`${styles.message} ${styles.assistant}`}>
            <div className={styles.roleLabel}>JARVIS</div>
            <div className={styles.content}>
              {currentResponse}
              <span className={styles.cursor} />
            </div>
          </div>
        )}
        {isStreaming && !currentResponse && (
          <div className={`${styles.message} ${styles.assistant}`}>
            <div className={styles.roleLabel}>JARVIS</div>
            <div className={styles.content}>
              <span className={styles.thinking}>Processing</span>
              <span className={styles.dots}>...</span>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className={styles.inputArea}>
        <textarea
          ref={inputRef}
          className={styles.input}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Ask JARVIS anything..."
          rows={1}
          disabled={isStreaming}
        />
        <button
          className={styles.sendBtn}
          onClick={handleSend}
          disabled={isStreaming || !input.trim()}
        >
          Send
        </button>
      </div>
    </div>
  );
}