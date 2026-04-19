import { useEffect } from 'react';
import { useDashboardStore } from '../../stores/dashboard';
import { useVoiceStore } from '../../stores/voice';
import styles from './VoiceIndicator.module.css';

export default function VoiceIndicator() {
  const voiceState = useDashboardStore((s) => s.voiceState);
  const activationSource = useDashboardStore((s) => s.activationSource);
  const voiceEnabled = useVoiceStore((s) => s.voiceEnabled);
  const setVoiceEnabled = useVoiceStore((s) => s.setVoiceEnabled);
  const isListening = useVoiceStore((s) => s.isListening);
  const isListeningForSpeech = useVoiceStore((s) => s.isListeningForSpeech);
  const isSpeaking = useVoiceStore((s) => s.isSpeaking);
  const startClapDetection = useVoiceStore((s) => s.startClapDetection);
  const stopClapDetection = useVoiceStore((s) => s.stopClapDetection);
  const startListening = useVoiceStore((s) => s.startListening);
  const stopListening = useVoiceStore((s) => s.stopListening);
  const currentVoice = useVoiceStore((s) => s.currentVoice);
  const availableVoices = useVoiceStore((s) => s.availableVoices);
  const setVoice = useVoiceStore((s) => s.setVoice);
  const loadVoices = useVoiceStore((s) => s.loadVoices);
  const loadConfig = useVoiceStore((s) => s.loadConfig);

  useEffect(() => {
    loadVoices();
    loadConfig();
  }, [loadVoices, loadConfig]);

  const stateConfig: Record<string, { label: string; color: string; pulse: boolean }> = {
    idle: { label: isListeningForSpeech ? 'HEARING' : (isListening ? 'CLAP ON' : 'MIC OFF'), color: isListeningForSpeech ? 'var(--color-success)' : (isListening ? 'var(--accent-primary)' : 'var(--text-muted)'), pulse: isListening || isListeningForSpeech },
    listening: { label: 'LISTENING', color: 'var(--accent-primary)', pulse: true },
    processing: { label: 'PROCESSING', color: 'var(--color-warning)', pulse: true },
    speaking: { label: 'SPEAKING', color: 'var(--color-success)', pulse: true },
  };

  const config = stateConfig[voiceState] || stateConfig.idle;

  const handleClapToggle = () => {
    if (isListening) {
      stopClapDetection();
    } else {
      startClapDetection();
    }
  };

  const handleListenToggle = () => {
    if (isListeningForSpeech) {
      stopListening();
    } else {
      startListening();
    }
  };

  return (
    <div className={styles.indicator}>
      <div
        className={`${styles.dot} ${config.pulse ? styles.pulse : ''}`}
        style={{ background: config.color, boxShadow: config.pulse ? `0 0 8px ${config.color}` : 'none' }}
      />
      <span className={styles.label} style={{ color: config.color }}>
        {config.label}
      </span>
      {activationSource && (
        <span className={styles.source}>{activationSource}</span>
      )}
      <button
        className={`${styles.toggleBtn} ${isListeningForSpeech ? styles.listenActive : ''}`}
        onClick={handleListenToggle}
        disabled={isSpeaking}
        title={isListeningForSpeech ? 'Stop listening' : 'Start listening'}
      >
        {isListeningForSpeech ? 'Listening' : 'Listen'}
      </button>
      <button
        className={`${styles.toggleBtn} ${isListening ? styles.active : ''}`}
        onClick={handleClapToggle}
        title={isListening ? 'Stop clap detection' : 'Start clap detection'}
      >
        {isListening ? 'Clap ON' : 'Clap OFF'}
      </button>
      <button
        className={`${styles.voiceToggle} ${voiceEnabled ? styles.voiceOn : ''}`}
        onClick={() => setVoiceEnabled(!voiceEnabled)}
        title={voiceEnabled ? 'Auto-speak ON' : 'Auto-speak OFF'}
      >
        {voiceEnabled ? 'Voice ON' : 'Voice OFF'}
      </button>
      {availableVoices.length > 0 && (
        <select
          className={styles.voiceSelect}
          value={currentVoice}
          onChange={(e) => setVoice(e.target.value)}
          title="JARVIS voice"
        >
          {availableVoices.map((v) => (
            <option key={v} value={v}>{v}</option>
          ))}
        </select>
      )}
    </div>
  );
}