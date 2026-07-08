/**
 * IPC health store — tracks command failures visible in SystemHealthPanel.
 *
 * Separate from the toast system: failures here are aggregated and surfaced
 * as a subtle health indicator, not a disruptive popup.
 */

import { writable, derived } from 'svelte/store';

export interface IpcFailure {
  command:   string;
  error:     string;
  timestamp: number;
  /** true = permanent (all retries exhausted), false = transient (in retry) */
  permanent: boolean;
  attempt?:  number;
}

const MAX_HISTORY = 50;

function createIpcHealth() {
  const { subscribe, update } = writable<IpcFailure[]>([]);

  return {
    subscribe,

    recordFailure(command: string, error: string) {
      update(list => [
        { command, error, timestamp: Date.now(), permanent: true },
        ...list,
      ].slice(0, MAX_HISTORY));
    },

    recordTransient(command: string, error: string, attempt: number) {
      update(list => [
        { command, error, timestamp: Date.now(), permanent: false, attempt },
        ...list,
      ].slice(0, MAX_HISTORY));
    },

    clearError(command: string) {
      update(list => list.filter(f => !(f.command === command && f.permanent)));
    },

    clear() {
      update(() => []);
    },
  };
}

export const ipcHealth = createIpcHealth();

/** Count of permanent (unrecovered) failures. */
export const permanentFailureCount = derived(
  ipcHealth,
  ($h) => $h.filter(f => f.permanent).length,
);
