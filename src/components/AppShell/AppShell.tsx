import { useState } from 'react';
import ChatPanel from '../ChatPanel/ChatPanel';
import Dashboard from '../Dashboard/Dashboard';
import VoiceIndicator from '../VoiceIndicator/VoiceIndicator';
import styles from './AppShell.module.css';

export default function AppShell() {
  const [dashboardCollapsed, setDashboardCollapsed] = useState(false);

  return (
    <div className={styles.shell} data-tauri-drag-region>
      <header className={styles.header}>
        <div className={styles.headerLeft}>
          <span className={styles.logo}>JARVIS</span>
          <VoiceIndicator />
        </div>
        <div className={styles.headerRight}>
          <button
            className={styles.toggleBtn}
            onClick={() => setDashboardCollapsed(!dashboardCollapsed)}
            title={dashboardCollapsed ? 'Show dashboard' : 'Hide dashboard'}
          >
            {dashboardCollapsed ? '◀' : '▶'}
          </button>
        </div>
      </header>
      <div className={styles.main}>
        <ChatPanel className={styles.chat} />
        {!dashboardCollapsed && <Dashboard className={styles.dashboard} />}
      </div>
    </div>
  );
}