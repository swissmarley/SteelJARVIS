export type MessageRole = 'user' | 'assistant' | 'system';

export interface Message {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
  isStreaming?: boolean;
  metadata?: {
    toolCalls?: ToolCall[];
    planSteps?: string[];
    duration?: number;
  };
}

export interface ToolCall {
  id: string;
  name: string;
  params: Record<string, unknown>;
  result?: unknown;
  status: 'pending' | 'running' | 'completed' | 'failed';
}

export interface ConversationState {
  messages: Message[];
  isStreaming: boolean;
  currentResponse: string;
  error: string | null;
}