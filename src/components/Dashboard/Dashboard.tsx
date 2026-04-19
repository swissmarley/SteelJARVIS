import { useState } from 'react';
import { useDashboardStore } from '../../stores/dashboard';
import styles from './Dashboard.module.css';

type PanelKey = 'status' | 'tasks' | 'timeline' | 'tools' | 'memory' | 'permissions' | 'errors' | 'health';

export default function Dashboard({ className }: { className?: string }) {
  const [expanded, setExpanded] = useState<Set<PanelKey>>(
    new Set(['status', 'tasks', 'timeline'])
  );

  const toggle = (key: PanelKey) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const {
    executionMode,
    activeGoal,
    taskQueue,
    actionTimeline,
    toolsInUse,
    memoryEvents,
    permissionRequests,
    voiceState,
    activationSource,
    systemHealth,
    errors,
  } = useDashboardStore();

  const modeColor: Record<string, string> = {
    idle: 'var(--text-muted)',
    listening: 'var(--color-info)',
    planning: 'var(--color-warning)',
    acting: 'var(--accent-primary)',
    waiting: 'var(--color-warning)',
    blocked: 'var(--color-error)',
    error: 'var(--color-error)',
    completed: 'var(--color-success)',
  };

  return (
    <div className={`${styles.dashboard} ${className || ''}`}>
      <div className={styles.title}>Dashboard</div>

      <Panel label="Status" panelKey="status" expanded={expanded} toggle={toggle}>
        <div className={styles.statusGrid}>
          <div className={styles.statusRow}>
            <span className={styles.statusLabel}>Mode</span>
            <span className={styles.statusValue} style={{ color: modeColor[executionMode] }}>
              {executionMode.toUpperCase()}
            </span>
          </div>
          <div className={styles.statusRow}>
            <span className={styles.statusLabel}>Voice</span>
            <span className={styles.statusValue}>{voiceState}</span>
          </div>
          <div className={styles.statusRow}>
            <span className={styles.statusLabel}>Activation</span>
            <span className={styles.statusValue}>{activationSource || 'none'}</span>
          </div>
          <div className={styles.statusRow}>
            <span className={styles.statusLabel}>Goal</span>
            <span className={styles.statusValue}>{activeGoal || '—'}</span>
          </div>
        </div>
      </Panel>

      <Panel label="Tasks" panelKey="tasks" expanded={expanded} toggle={toggle}>
        {taskQueue.length === 0 ? (
          <div className={styles.empty}>No active tasks</div>
        ) : (
          taskQueue.map((task) => (
            <div key={task.id} className={styles.taskItem}>
              <span className={`${styles.taskDot} ${styles[task.status]}`} />
              <span className={styles.taskDesc}>{task.description}</span>
            </div>
          ))
        )}
      </Panel>

      <Panel label="Timeline" panelKey="timeline" expanded={expanded} toggle={toggle}>
        {actionTimeline.length === 0 ? (
          <div className={styles.empty}>No activity yet</div>
        ) : (
          actionTimeline.slice(0, 20).map((event) => (
            <div key={event.id} className={styles.timelineItem}>
              <span className={styles.timelineTime}>
                {new Date(event.timestamp).toLocaleTimeString()}
              </span>
              <span className={styles.timelineType}>{event.type}</span>
              <span className={styles.timelineDesc}>{event.description}</span>
            </div>
          ))
        )}
      </Panel>

      <Panel label="Tools" panelKey="tools" expanded={expanded} toggle={toggle}>
        {toolsInUse.length === 0 ? (
          <div className={styles.empty}>No tools active</div>
        ) : (
          toolsInUse.map((tool) => (
            <div key={tool.name} className={styles.toolItem}>
              <span className={`${styles.toolStatus} ${styles[tool.status]}`} />
              <span className={styles.toolName}>{tool.name}</span>
            </div>
          ))
        )}
      </Panel>

      <Panel label="Memory" panelKey="memory" expanded={expanded} toggle={toggle}>
        {memoryEvents.length === 0 ? (
          <div className={styles.empty}>No memory events</div>
        ) : (
          memoryEvents.slice(0, 10).map((event, i) => (
            <div key={i} className={styles.memoryEvent}>
              <span className={styles.memoryType}>{event.type}</span>
              <span className={styles.memoryPreview}>{event.preview}</span>
            </div>
          ))
        )}
      </Panel>

      <Panel label="Permissions" panelKey="permissions" expanded={expanded} toggle={toggle}>
        {permissionRequests.filter((r) => !r.resolved).length === 0 ? (
          <div className={styles.empty}>No pending approvals</div>
        ) : (
          permissionRequests
            .filter((r) => !r.resolved)
            .map((req) => (
              <div key={req.id} className={styles.permItem}>
                <span className={styles.permAction}>{req.action}</span>
                <span className={styles.permDetails}>{req.details}</span>
              </div>
            ))
        )}
      </Panel>

      <Panel label="Errors" panelKey="errors" expanded={expanded} toggle={toggle}>
        {errors.length === 0 ? (
          <div className={styles.empty}>No errors</div>
        ) : (
          errors.slice(0, 10).map((err, i) => (
            <div key={i} className={styles.errorItem}>
              <span className={styles.errorSource}>{err.source}</span>
              <span className={styles.errorMessage}>{err.message}</span>
            </div>
          ))
        )}
      </Panel>

      <Panel label="System Health" panelKey="health" expanded={expanded} toggle={toggle}>
        <div className={styles.healthGrid}>
          <HealthDot label="AI Provider" ok={systemHealth.providerConnected} />
          <HealthDot label="Database" ok={systemHealth.dbConnected} />
          <HealthDot label="Audio" ok={systemHealth.audioAvailable} />
        </div>
      </Panel>
    </div>
  );
}

function Panel({
  label,
  panelKey,
  expanded,
  toggle,
  children,
}: {
  label: string;
  panelKey: PanelKey;
  expanded: Set<PanelKey>;
  toggle: (key: PanelKey) => void;
  children: React.ReactNode;
}) {
  const isOpen = expanded.has(panelKey);
  return (
    <div className={styles.panel}>
      <button className={styles.panelHeader} onClick={() => toggle(panelKey)}>
        <span className={styles.panelArrow}>{isOpen ? '▾' : '▸'}</span>
        <span className={styles.panelLabel}>{label}</span>
      </button>
      {isOpen && <div className={styles.panelBody}>{children}</div>}
    </div>
  );
}

function HealthDot({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div className={styles.statusRow}>
      <span className={styles.healthDot} style={{ background: ok ? 'var(--color-success)' : 'var(--color-error)' }} />
      <span className={styles.statusLabel}>{label}</span>
    </div>
  );
}