import { writable, derived } from 'svelte/store';

export interface Workspace {
  path:       string;
  name:       string;
  branch:     string;
  isGitClean: boolean;
}

export const currentWorkspace  = writable<Workspace | null>(null);
export const isWorkspaceOpen   = derived(currentWorkspace, ($ws) => $ws !== null);

/** Bump this to trigger FileTree refresh */
export const fileTreeRefresh   = writable<number>(0);

export function setWorkspace(path: string, branch = 'main') {
  const name = path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? 'Untitled';
  currentWorkspace.set({ path, name, branch, isGitClean: true });
  fileTreeRefresh.set(Date.now());
}
