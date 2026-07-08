import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface AssistantSession {
  id: string;
  profile_id: string | null;
  title: string;
  created_at: number;
  updated_at: number;
}

export const sessions = writable<AssistantSession[]>([]);
export const currentSessionId = writable<string | null>(null);

export async function loadSessions(profileId?: string): Promise<void> {
  const result: AssistantSession[] = await invoke('list_assistant_sessions', {
    profileId: profileId ?? null,
  });
  sessions.set(result);
}

export async function createSession(profileId: string | null, title = 'New conversation'): Promise<AssistantSession> {
  const session: AssistantSession = await invoke('create_assistant_session', {
    profileId: profileId ?? null,
    title,
  });
  sessions.update(s => [session, ...s.filter(x => x.id !== session.id)]);
  currentSessionId.set(session.id);
  return session;
}

export async function deleteSession(id: string): Promise<void> {
  await invoke('delete_assistant_session', { id });
  sessions.update(s => s.filter(x => x.id !== id));
  if (get(currentSessionId) === id) {
    currentSessionId.set(null);
  }
}
