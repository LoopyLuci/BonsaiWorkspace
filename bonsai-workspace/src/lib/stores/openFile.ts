import { writable } from 'svelte/store';

/**
 * When FileTree wants the editor to open a file it writes the path here.
 * MonacoEditor subscribes and reacts.
 *
 * Using a store avoids the broken bind:this / exported-function patterns
 * from the original blueprint.
 */
export const openFileRequest = writable<string | null>(null);

export function requestOpenFile(path: string) {
  openFileRequest.set(path);
}
