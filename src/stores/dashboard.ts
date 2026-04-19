import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import type { ExecutionMode, VoiceState, ActivationSource } from '../types/events';
import type { ToolInfo, PermissionRequest } from '../types/tools';
import type { MemoryEvent } from '../types/memory';

export interface TaskItem {
  id: string;
  description: string;
  status: 'pending' | 'in_progress' | 'completed' | 'blocked' | 'failed';
  result?: string;
  error?: string;
}

export interface ActionEvent {
  id: string;
  type: string;
  description: string;
  timestamp: number;
  details?: Record<string, unknown>;
}

export interface SystemHealth {
  providerConnected: boolean;
  dbConnected: boolean;
  audioAvailable: boolean;
  lastHeartbeat: number;
}

interface DashboardState {
  executionMode: ExecutionMode;
  activeGoal: string | null;
  taskQueue: TaskItem[];
  actionTimeline: ActionEvent[];
  toolsInUse: ToolInfo[];
  memoryEvents: MemoryEvent[];
  permissionRequests: PermissionRequest[];
  voiceState: VoiceState;
  activationSource: ActivationSource | null;
  systemHealth: SystemHealth;
  errors: Array<{ source: string; message: string; timestamp: number }>;
}

interface DashboardActions {
  setExecutionMode: (mode: ExecutionMode) => void;
  setActiveGoal: (goal: string | null) => void;
  addTask: (task: TaskItem) => void;
  updateTaskStatus: (id: string, status: TaskItem['status'], result?: string, error?: string) => void;
  addActionEvent: (event: ActionEvent) => void;
  addMemoryEvent: (event: MemoryEvent) => void;
  addPermissionRequest: (request: PermissionRequest) => void;
  resolvePermission: (id: string, approved: boolean) => void;
  setVoiceState: (state: VoiceState) => void;
  setActivationSource: (source: ActivationSource | null) => void;
  updateSystemHealth: (health: Partial<SystemHealth>) => void;
  addToolInUse: (tool: ToolInfo) => void;
  addError: (source: string, message: string) => void;
  clearErrors: () => void;
}

export const useDashboardStore = create<DashboardState & DashboardActions>()(
  immer((set) => ({
    executionMode: 'idle',
    activeGoal: null,
    taskQueue: [],
    actionTimeline: [],
    toolsInUse: [],
    memoryEvents: [],
    permissionRequests: [],
    voiceState: 'idle',
    activationSource: null,
    systemHealth: {
      providerConnected: false,
      dbConnected: false,
      audioAvailable: false,
      lastHeartbeat: Date.now(),
    },
    errors: [],

    setExecutionMode: (mode) =>
      set((state) => { state.executionMode = mode; }),

    setActiveGoal: (goal) =>
      set((state) => { state.activeGoal = goal; }),

    addTask: (task) =>
      set((state) => { state.taskQueue.push(task); }),

    updateTaskStatus: (id, status, result, error) =>
      set((state) => {
        const task = state.taskQueue.find((t) => t.id === id);
        if (task) {
          task.status = status;
          if (result) task.result = result;
          if (error) task.error = error;
        }
      }),

    addActionEvent: (event) =>
      set((state) => {
        state.actionTimeline.unshift(event);
        if (state.actionTimeline.length > 500) state.actionTimeline.pop();
      }),

    addMemoryEvent: (event) =>
      set((state) => { state.memoryEvents.unshift(event); }),

    addPermissionRequest: (request) =>
      set((state) => { state.permissionRequests.push(request); }),

    resolvePermission: (id, approved) =>
      set((state) => {
        const req = state.permissionRequests.find((r) => r.id === id);
        if (req) {
          req.resolved = true;
          req.approved = approved;
        }
      }),

    setVoiceState: (state) =>
      set((s) => { s.voiceState = state; }),

    setActivationSource: (source) =>
      set((s) => { s.activationSource = source; }),

    updateSystemHealth: (health) =>
      set((state) => {
        Object.assign(state.systemHealth, health);
        state.systemHealth.lastHeartbeat = Date.now();
      }),

    addError: (source, message) =>
      set((state) => {
        state.errors.unshift({ source, message, timestamp: Date.now() });
        if (state.errors.length > 100) state.errors.pop();
      }),

    addToolInUse: (tool) =>
      set((state) => {
        const existing = state.toolsInUse.findIndex((t) => t.name === tool.name);
        if (existing >= 0) {
          state.toolsInUse[existing] = tool;
        } else {
          state.toolsInUse.push(tool);
        }
      }),

    clearErrors: () =>
      set((state) => { state.errors = []; }),
  }))
);