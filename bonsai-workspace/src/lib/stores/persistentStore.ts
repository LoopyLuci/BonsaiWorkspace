/**
 * persistentStore — CAS-backed Svelte writable that survives crashes.
 *
 * On construction the store loads its last checkpoint from the Tauri CAS
 * (via `cas_get`). On every write it serialises to JSON and pushes to CAS
 * (via `cas_put`), then updates the Svelte store so subscribers react.
 *
 * Usage:
 *   const myStore = await PersistentStore.open<MyType>('myKey', defaultValue);
 *   myStore.store.subscribe(v => console.log(v));
 *   await myStore.set({ ...newValue });
 */

import { writable, type Writable } from 'svelte/store';
import { resilientInvoke } from '$lib/utils/ipc';

export class PersistentStore<T> {
  readonly store: Writable<T>;
  private key: string;

  private constructor(key: string, initial: T) {
    this.key   = key;
    this.store = writable<T>(initial);
  }

  /** Open (or create) a persistent store. Loads from CAS if available. */
  static async open<T>(key: string, defaultValue: T): Promise<PersistentStore<T>> {
    const ps = new PersistentStore<T>(key, defaultValue);
    try {
      const raw = await resilientInvoke<string | null>('cas_get', { key });
      if (raw) {
        const parsed = JSON.parse(raw) as T;
        ps.store.set(parsed);
      }
    } catch {
      // CAS unavailable — start with default, checkpoint will happen on first write.
    }
    return ps;
  }

  /** Update the store and persist to CAS. */
  async set(value: T): Promise<void> {
    this.store.set(value);
    try {
      await resilientInvoke('cas_put', {
        key:  this.key,
        data: JSON.stringify(value),
        mime: 'application/json',
      });
    } catch {
      // Non-fatal: in-memory store is updated; persistence will retry next write.
    }
  }

  /** Apply a partial update and persist. */
  async update(fn: (current: T) => T): Promise<void> {
    return new Promise<void>((resolve) => {
      this.store.update((current) => {
        const next = fn(current);
        this.set(next).then(resolve);
        return next;
      });
    });
  }
}
