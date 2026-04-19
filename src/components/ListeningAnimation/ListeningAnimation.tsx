import { useVoiceStore } from '../../stores/voice';
import styles from './ListeningAnimation.module.css';

export default function ListeningAnimation() {
  const isListeningForSpeech = useVoiceStore((s) => s.isListeningForSpeech);
  const partialTranscript = useVoiceStore((s) => s.partialTranscript);
  const sttError = useVoiceStore((s) => s.sttError);

  if (!isListeningForSpeech) return null;

  return (
    <div className={styles.container}>
      <div className={styles.rings}>
        <div className={`${styles.ring} ${styles.ring1}`} />
        <div className={`${styles.ring} ${styles.ring2}`} />
        <div className={`${styles.ring} ${styles.ring3}`} />
        <div className={styles.core}>
          <span className={styles.icon}>J</span>
        </div>
      </div>
      {sttError ? (
        <div className={styles.error}>{sttError}</div>
      ) : partialTranscript ? (
        <div className={styles.transcript}>{partialTranscript}</div>
      ) : (
        <div className={styles.listening}>Listening...</div>
      )}
    </div>
  );
}