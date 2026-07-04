export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export type ObservationMode = 'none' | 'text' | 'html';

export interface ExtensionSettings {
  apiHost: string;
  apiPort: number;
  buddyHost: string;
  buddyPort: number;
  workspaceUrl: string;
  defaultModel: string;
  desktopConnectionToken: string;
  observationMode: ObservationMode;
  autoAllowActionsByHost: string[];
}

export interface AuditEntry {
  id: string;
  action: string;
  url: string;
  selector?: string;
  payload?: Record<string, unknown>;
  result: 'allowed' | 'denied' | 'success' | 'error';
  createdAt: string;
  message?: string;
}

export interface PageSnapshot {
  title: string;
  url: string;
  visibleText: string;
  html?: string;
}

export interface SelectedElementInfo {
  selector: string;
  text: string;
  tagName: string;
}

export type BackgroundRequest =
  | { type: 'PING' }
  | { type: 'GET_STATUS' }
  | { type: 'CONNECT' }
  | { type: 'LIST_MODELS' }
  | { type: 'GET_SETTINGS' }
  | { type: 'SAVE_SETTINGS'; settings: Partial<ExtensionSettings> }
  | { type: 'OPEN_WORKSPACE' }
  | { type: 'GET_PAGE_SNAPSHOT'; tabId?: number; includeHtml?: boolean }
  | { type: 'SUMMARIZE_CURRENT_PAGE' }
  | { type: 'CHAT'; messages: ChatMessage[] }
  | { type: 'CHAT_STREAM'; messages: ChatMessage[]; streamId: string }
  | { type: 'REQUEST_AUTOMATION'; tabId: number; action: 'click' | 'type' | 'navigate' | 'scroll'; selector?: string; text?: string; url?: string }
  | { type: 'GET_AUDIT_LOG' }
  | { type: 'CLEAR_AUDIT_LOG' };

export type BackgroundResponse =
  | { ok: true; data?: unknown }
  | { ok: false; error: string };

export type ExtensionEvent =
  | { type: 'CHAT_TOKEN'; streamId: string; token: string }
  | { type: 'CHAT_DONE'; streamId: string }
  | { type: 'CONNECTION_STATUS'; connected: boolean; detail?: string }
  | { type: 'ACTIVITY'; message: string }
  | { type: 'AUDIT_UPDATED' };
