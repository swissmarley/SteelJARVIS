export interface ToolInfo {
  name: string;
  description: string;
  status: 'available' | 'running' | 'error';
  lastUsed: number | null;
}

export interface ToolResult {
  success: boolean;
  data: unknown;
  error?: string;
}

export interface PermissionRequest {
  id: string;
  action: string;
  category: PermissionCategory;
  details: string;
  timestamp: number;
  resolved: boolean;
  approved: boolean | null;
}

export type PermissionCategory =
  | 'app_launch'
  | 'app_control'
  | 'file_access'
  | 'window_control'
  | 'web_search'
  | 'memory_write'
  | 'clipboard'
  | 'network';

export type PermissionLevel = 'allowed' | 'ask_once' | 'ask_always' | 'denied';

export interface PermissionRule {
  category: PermissionCategory;
  level: PermissionLevel;
}