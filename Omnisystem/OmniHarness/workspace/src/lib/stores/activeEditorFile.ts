import { writable } from 'svelte/store';

export interface ActiveEditorFile {
  path:    string;
  content: string;
}

/** The file currently open in the Monaco editor, or null if none. */
export const activeEditorFile = writable<ActiveEditorFile | null>(null);
