import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { setWorkspace } from '$lib/stores/workspace';

export interface TokenStats {
  prompt_tokens:          number;
  completion_tokens:      number;
  tokens_per_second:      number;
  time_to_first_token_ms: number;
  total_time_ms:          number;
}

export interface ChatMessage {
  id:           string;
  role:         'user' | 'assistant';
  content:      string;
  timestamp:    Date;
  attachments?: string[];
  /** Token stats — only set on assistant messages. */
  stats?:       TokenStats;
  /** Tool names auto-executed before this response. */
  tools_used?:  string[];
}

export interface PermissionCardData {
  id:              string;
  type?:           string;
  description?:    string;
  rationale:       string;
  paths_affected:  string[];
  command?:        string;
  action?:         any;
  tool?:           string;
  args?:           any;
  /** Original model response containing the tool call (for HITL continuation). */
  raw_response?:   string;
  /** Full message context at the point the tool call was made (for HITL continuation). */
  ctx_snapshot?:   any[];
  /** Optional path for inline diff preview workflow. */
  file_path?:      string;
  /** Optional unified diff attached to the permission request. */
  unified_diff?:   string;
}

export type AskBonsaiAction = 'explain' | 'fix' | 'refactor';

export interface AskBonsaiRequest {
  action: AskBonsaiAction;
  prompt: string;
}

export const messages = writable<ChatMessage[]>([]);
export const permissionCards = writable<PermissionCardData[]>([]);
export const isThinking = writable(false);
export const tokenSpeed = writable<number>(0);
export const currentSessionId = writable<string | null>(null);
export const currentSessionTitle = writable<string>('');
export const askBonsaiRequest = writable<AskBonsaiRequest | null>(null);

const SESSION_STATE_KEY = 'bonsai-current-session';

export function setCurrentSession(id: string | null, title = '') {
  currentSessionId.set(id);
  currentSessionTitle.set(title);
  persistSessionState(id, title);
}

export function clearCurrentSession() {
  setCurrentSession(null, '');
}

function persistSessionState(id: string | null, title: string) {
  if (typeof window !== 'undefined' && window.localStorage) {
    window.localStorage.setItem(
      SESSION_STATE_KEY,
      JSON.stringify({ currentSessionId: id, currentSessionTitle: title })
    );
  }

  void invoke('set_current_session_state', {
    sessionId: id,
    title: title || null,
  }).catch(() => {
    // best-effort persistence
  });
}

export async function loadPersistentSession() {
  let state: { currentSessionId: string | null; currentSessionTitle: string | null } | null = null;

  if (typeof window !== 'undefined' && window.localStorage) {
    try {
      const stored = window.localStorage.getItem(SESSION_STATE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        state = {
          currentSessionId: parsed.currentSessionId ?? null,
          currentSessionTitle: parsed.currentSessionTitle ?? null,
        };
      }
    } catch {
      state = null;
    }
  }

  if (!state) {
    try {
      const backendState = await invoke<{ current_session_id: string | null; current_session_title: string | null }>('get_current_session_state');
      state = {
        currentSessionId: backendState.current_session_id ?? null,
        currentSessionTitle: backendState.current_session_title ?? null,
      };
    } catch {
      state = null;
    }
  }

  if (state?.currentSessionId) {
    currentSessionId.set(state.currentSessionId);
    currentSessionTitle.set(state.currentSessionTitle ?? '');
  }

  return state;
}

export async function restorePersistentSession() {
  const state = await loadPersistentSession();
  if (!state?.currentSessionId) return state;

  try {
    const result = await invoke<any>('load_chat_session', { sessionId: state.currentSessionId });
    const loadedMessages = result.messages.map((msg: any) => ({
      id: crypto.randomUUID(),
      role: msg.role,
      content: msg.content,
      timestamp: new Date(),
    }));

    if (typeof result.workspace_path === 'string' && result.workspace_path.trim()) {
      let branch = 'main';
      try {
        branch = await invoke<string>('get_git_branch', { workspacePath: result.workspace_path });
      } catch {
        // Keep fallback branch when workspace is not a git repo or unavailable.
      }
      setWorkspace(result.workspace_path, branch);
    }

    messages.set(loadedMessages);
    setCurrentSession(result.id, result.title ?? state.currentSessionTitle ?? '');
  } catch (error) {
    console.warn('Persisted session restore failed:', error);
  }

  return state;
}

export function addUserMessage(content: string, attachments: string[] = []) {
  messages.update((m) => [
    ...m,
    { id: crypto.randomUUID(), role: 'user', content, timestamp: new Date(), attachments },
  ]);
}

export function addAssistantMessage(
  content:     string,
  stats?:      TokenStats,
  tools_used?: string[],
) {
  messages.update((m) => [
    ...m,
    {
      id: crypto.randomUUID(),
      role: 'assistant' as const,
      content,
      timestamp: new Date(),
      stats,
      tools_used,
    },
  ]);
}

export function addPermissionCard(data: Omit<PermissionCardData, 'id'>) {
  permissionCards.update((cards) => [...cards, { ...data, id: crypto.randomUUID() }]);
}

export function removePermissionCard(id: string) {
  permissionCards.update((cards) => cards.filter((c) => c.id !== id));
}

export function requestAskBonsai(request: AskBonsaiRequest) {
  askBonsaiRequest.set(request);
}

export function clearChat() {
  messages.set([]);
}
