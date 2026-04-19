export type ExecutionMode =
  | 'idle'
  | 'listening'
  | 'planning'
  | 'acting'
  | 'waiting'
  | 'blocked'
  | 'error'
  | 'completed';

export type VoiceState = 'idle' | 'listening' | 'processing' | 'speaking';

export type ActivationSource = 'clap' | 'hotkey' | 'push-to-talk' | 'wake-word' | 'ui-click';

export interface JarvisEvent {
  type: string;
  timestamp: number;
  [key: string]: unknown;
}

export interface StateChangedEvent extends JarvisEvent {
  type: 'StateChanged';
  from: ExecutionMode;
  to: ExecutionMode;
}

export interface ToolInvokedEvent extends JarvisEvent {
  type: 'ToolInvoked';
  tool: string;
  params: Record<string, unknown>;
}

export interface ToolCompletedEvent extends JarvisEvent {
  type: 'ToolCompleted';
  tool: string;
  result: unknown;
}

export interface MemorySavedEvent extends JarvisEvent {
  type: 'MemorySaved';
  id: string;
  category: string;
  preview: string;
}

export interface MemoryRetrievedEvent extends JarvisEvent {
  type: 'MemoryRetrieved';
  id: string;
  query: string;
}

export interface PermissionRequestedEvent extends JarvisEvent {
  type: 'PermissionRequested';
  action: string;
  details: string;
}

export interface ClapDetectedEvent extends JarvisEvent {
  type: 'ClapDetected';
  confidence: number;
}

export interface VoiceStateChangedEvent extends JarvisEvent {
  type: 'VoiceStateChanged';
  state: VoiceState;
}

export interface ErrorEvent extends JarvisEvent {
  type: 'Error';
  source: string;
  message: string;
}

export interface StepEvent extends JarvisEvent {
  type: 'StepStarted' | 'StepCompleted' | 'StepFailed';
  index: number;
  description?: string;
  result?: string;
  error?: string;
}

export interface ActivationTriggeredEvent extends JarvisEvent {
  type: 'ActivationTriggered';
  source: ActivationSource;
}

export interface PlanCreatedEvent extends JarvisEvent {
  type: 'PlanCreated';
  steps: string[];
}

export interface GoalSetEvent extends JarvisEvent {
  type: 'GoalSet';
  goal: string;
}