import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { currentSessionId, loadSessions, createSession } from './assistantSessions';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface AssistantProfile {
  id: string;
  name: string;
  persona_id: string | null;
  avatar_id: string | null;
  tts_voice: string;
  tts_speed: number;
  tts_pitch: number;
  tts_enabled: boolean;
  wake_word: string | null;
  tool_permissions: string;
  system_prompt: string;
  model_id: string | null;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface AvatarAsset {
  id: string;
  name: string;
  asset_type: string;
  asset_data: string | null;
  file_path: string | null;
  thumbnail_svg: string | null;
  validated: boolean;
  created_at: number;
  updated_at: number;
}

export interface ChatGameState {
  game_type: string;        // "chess" | "go"
  session_id: string;
  position: string;         // FEN for chess, JSON stones array for Go
  last_move: string | null;
  legal_moves: string[];
  turn: string;
  orientation: string;
  interactive: boolean;
  result: string;
  board_size: number | null;
  score_estimate: number | null;
}

export interface AssistantMessage {
  id: string;
  session_id: string;
  role: 'user' | 'assistant' | 'tool';
  content: string;
  tool_name: string | null;
  tool_result: string | null;
  tts_synthesized: boolean;
  created_at: number;
  game_state?: ChatGameState | null;
}

// ── Stores ────────────────────────────────────────────────────────────────────

export const allProfiles = writable<AssistantProfile[]>([]);
export const activeProfile = writable<AssistantProfile | null>(null);
export const avatarAssets = writable<AvatarAsset[]>([]);
export const currentAvatarSvg = writable<string | null>(null);

export const assistantMessages = writable<AssistantMessage[]>([]);
export const streamingToken = writable<string>('');
export const isAssistantThinking = writable(false);
export const assistantInitError = writable('');
export const assistantError = writable<string>('');

// ── Init ──────────────────────────────────────────────────────────────────────

let streamUnlisten: (() => void) | null = null;

export async function loadAssistantSessionMessages(sessionId: string): Promise<void> {
  const msgs: AssistantMessage[] = await invoke('load_assistant_session', { sessionId });
  assistantMessages.set(Array.isArray(msgs) ? msgs : []);
}

export async function startNewAssistantSession(title = 'New conversation'): Promise<void> {
  const profile = get(activeProfile);
  if (!profile) return;
  const session = await createSession(profile.id, title);
  currentSessionId.set(session.id);
  assistantMessages.set([]);
  streamingToken.set('');
  await loadAssistantSessionMessages(session.id);
}

export async function initAssistantStores(): Promise<void> {
  assistantInitError.set('');
  try {
    // Load profiles
    const profiles: AssistantProfile[] = await invoke('list_assistant_profiles');
    allProfiles.set(Array.isArray(profiles) ? profiles : []);

    const active: AssistantProfile | null = await invoke('get_active_assistant_profile');
    activeProfile.set(active);

    // Load avatars
    const avatars: AvatarAsset[] = await invoke('list_avatar_assets');
    avatarAssets.set(Array.isArray(avatars) ? avatars : []);

    // Load sessions for active profile
    if (active) {
      await loadSessions(active.id);
      // Ensure a session exists
      const sid = get(currentSessionId);
      if (!sid) {
        const session = await createSession(active.id);
        currentSessionId.set(session.id);
        await loadAssistantSessionMessages(session.id);
      } else {
        await loadAssistantSessionMessages(sid);
      }
    }

    // Subscribe to streaming tokens. On mobile builds, some window capability
    // profiles may deny event.listen for the assistant webview; treat that as
    // non-fatal and continue without streaming tokens.
    if (streamUnlisten) streamUnlisten();
    try {
      streamUnlisten = await listen<string>('token-stream-assistant', (evt) => {
        streamingToken.update(t => t + evt.payload);
      });
    } catch {
      streamUnlisten = null;
    }

    // Listen for session auto-title updates from backend
    try {
      await listen<{ session_id: string; title: string }>('assistant-session-titled', () => {
        loadSessions(get(activeProfile)?.id ?? '').catch(() => {});
      });
    } catch { /* non-fatal */ }
  } catch (e) {
    assistantInitError.set(String(e));
  }
}

// ── Send a message ────────────────────────────────────────────────────────────

export async function sendAssistantMessage(text: string): Promise<void> {
  const sid = get(currentSessionId);
  if (!sid) return;

  const isFirstMessage = get(assistantMessages).length === 0;
  const ts = Math.floor(Date.now() / 1000);
  const userMsg: AssistantMessage = {
    id: Math.random().toString(36).slice(2),
    session_id: sid,
    role: 'user',
    content: text,
    tool_name: null,
    tool_result: null,
    tts_synthesized: false,
    created_at: ts,
  };
  assistantMessages.update(m => [...m, userMsg]);

  isAssistantThinking.set(true);
  streamingToken.set('');
  assistantError.set('');

  try {
    const reply: string = await invoke('submit_assistant_chat', {
      sessionId: sid,
      userMessage: text,
    });

    // Streaming may have already emitted tokens; replace with final reply
    const asstMsg: AssistantMessage = {
      id: Math.random().toString(36).slice(2),
      session_id: sid,
      role: 'assistant',
      content: reply,
      tool_name: null,
      tool_result: null,
      tts_synthesized: false,
      created_at: Math.floor(Date.now() / 1000),
    };
    assistantMessages.update(m => [...m, asstMsg]);
    streamingToken.set('');

    // Auto-title after first exchange (fire-and-forget)
    if (isFirstMessage && reply.trim()) {
      invoke('auto_title_session', {
        sessionId: sid,
        userMsg: text,
        replyMsg: reply,
      }).catch(() => {});
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    assistantError.set(msg);
    setTimeout(() => assistantError.set(''), 8000);
  } finally {
    isAssistantThinking.set(false);
  }
}
