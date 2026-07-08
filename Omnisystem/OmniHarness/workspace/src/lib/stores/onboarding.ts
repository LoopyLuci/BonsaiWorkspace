/**
 * onboarding — tracks whether the first-run guided setup wizard has been
 * completed. Persisted with plain `localStorage` (same pattern as
 * `theme.ts`) rather than the CAS-backed `PersistentStore`, since this flag
 * must be readable synchronously before the Tauri backend is necessarily
 * warmed up, and losing it on a crash has no real consequence (worst case:
 * the wizard shows again).
 */

import { writable } from 'svelte/store';

const STORAGE_KEY = 'workspace_onboarding_done';

function loadDone(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem(STORAGE_KEY) === '1';
}

/** Whether the user has ever completed (or skipped) the guided setup wizard. */
export const onboardingDone = writable<boolean>(loadDone());

/** Whether the wizard is currently visible. Not persisted — always starts closed. */
export const showOnboarding = writable<boolean>(false);

onboardingDone.subscribe((done) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, done ? '1' : '0');
  }
});

/** Marks the wizard as done (or skipped) and hides it. */
export function completeOnboarding() {
  onboardingDone.set(true);
  showOnboarding.set(false);
}

/** Reopens the wizard on demand (Command Palette / toolbar "?" button). */
export function restartOnboarding() {
  showOnboarding.set(true);
}
