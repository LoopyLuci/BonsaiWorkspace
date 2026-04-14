import { writable } from 'svelte/store';

export interface ChatMessage {
  id:          string;
  role:        'user' | 'assistant';
  content:     string;
  timestamp:   Date;
  attachments?: string[];
}

export interface PermissionCardData {
  id:              string;
  type?:           string;
  description?:    string;
  rationale:       string;
  paths_affected:  string[];
  command?:        string;
}

export const messages       = writable<ChatMessage[]>([]);
export const permissionCards = writable<PermissionCardData[]>([]);
export const isThinking      = writable(false);
export const tokenSpeed      = writable<number>(0);

export function addUserMessage(content: string, attachments: string[] = []) {
  messages.update((m) => [
    ...m,
    { id: crypto.randomUUID(), role: 'user', content, timestamp: new Date(), attachments },
  ]);
}

export function addAssistantMessage(content: string) {
  messages.update((m) => [
    ...m,
    { id: crypto.randomUUID(), role: 'assistant', content, timestamp: new Date() },
  ]);
}

export function addPermissionCard(data: Omit<PermissionCardData, 'id'>) {
  permissionCards.update((cards) => [...cards, { ...data, id: crypto.randomUUID() }]);
}

export function removePermissionCard(id: string) {
  permissionCards.update((cards) => cards.filter((c) => c.id !== id));
}

export function clearChat() {
  messages.set([]);
}
