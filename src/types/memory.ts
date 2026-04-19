export type MemoryCategory =
  | 'profile'
  | 'preferences'
  | 'facts'
  | 'task_history'
  | 'workflows'
  | 'app_preferences'
  | 'recruiting'
  | 'relationships'
  | 'notes';

export type MemorySource = 'explicit' | 'auto_extracted' | 'tool_result';
export type PrivacyLabel = 'normal' | 'sensitive' | 'private';

export interface Memory {
  id: string;
  content: string;
  category: MemoryCategory;
  confidence: number;
  source: MemorySource;
  privacyLabel: PrivacyLabel;
  pinned: boolean;
  createdAt: number;
  updatedAt: number;
  lastAccessed: number | null;
  accessCount: number;
  metadata: Record<string, unknown>;
}

export interface MemorySearchResult {
  memory: Memory;
  score: number;
}

export interface MemoryEvent {
  type: 'save' | 'retrieve' | 'update' | 'delete' | 'pin' | 'unpin';
  memoryId: string;
  timestamp: number;
  preview: string;
}