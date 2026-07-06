import { writable } from 'svelte/store';

export type Theme = 'dark' | 'light';

function detectSystemTheme(): Theme {
  if (typeof window === 'undefined') return 'dark';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function loadTheme(): Theme {
  if (typeof localStorage === 'undefined') return detectSystemTheme();
  const stored = localStorage.getItem('workspace_theme');
  if (stored === 'dark' || stored === 'light') return stored;
  return detectSystemTheme();
}

export const theme = writable<Theme>(loadTheme());

// Apply theme to <html data-theme="..."> and persist.
theme.subscribe((t) => {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', t);
  }
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('workspace_theme', t);
  }
});

// Sync with OS-level changes.
if (typeof window !== 'undefined') {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    if (!localStorage.getItem('workspace_theme')) {
      theme.set(e.matches ? 'dark' : 'light');
    }
  });
}

export function toggleTheme() {
  theme.update(t => t === 'dark' ? 'light' : 'dark');
}
