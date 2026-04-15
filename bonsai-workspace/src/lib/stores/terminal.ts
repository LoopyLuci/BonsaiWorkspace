import { writable } from 'svelte/store';

export const showTerminal = writable(false);

export function toggleTerminal() {
  showTerminal.update((v) => !v);
}
