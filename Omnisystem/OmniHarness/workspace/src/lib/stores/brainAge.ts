import { writable, derived } from 'svelte/store';

export const PHASE_KEYS = [
  'safety', 'survival', 'tool_use', 'code',
  'chat', 'reason', 'final', 'convert',
] as const;

export type PhaseKey = typeof PHASE_KEYS[number];

export type BrainLevel =
  | 'Newborn'   // 0 lessons
  | 'Curious'   // 1–2
  | 'Learning'  // 3–4
  | 'Smart'     // 5–6
  | 'Genius'    // 7
  | 'Flawless'; // 8 (all)

export interface BrainMetadata {
  lessons_completed: number;
  phases_done:       string[];
  last_training:     string | null;
}

// ── Persistent store ───────────────────────────────────────────────────────────

function loadFromStorage(): string[] {
  try {
    const raw = localStorage.getItem('workspace_brain_phases');
    if (raw) return JSON.parse(raw) as string[];
  } catch { /* ignore */ }
  return [];
}

function saveToStorage(phases: string[]) {
  try { localStorage.setItem('workspace_brain_phases', JSON.stringify(phases)); } catch { /* ignore */ }
}

export const completedPhases = writable<string[]>(loadFromStorage());

// Keep localStorage in sync whenever the store changes.
completedPhases.subscribe(phases => saveToStorage(phases));

// ── Derived level ──────────────────────────────────────────────────────────────

export const brainLevel = derived(completedPhases, (phases): BrainLevel => {
  const n = new Set(phases.filter(p => PHASE_KEYS.includes(p as PhaseKey))).size;
  if (n >= 8)   return 'Flawless';
  if (n >= 7)   return 'Genius';
  if (n >= 5)   return 'Smart';
  if (n >= 3)   return 'Learning';
  if (n >= 1)   return 'Curious';
  return 'Newborn';
});

export const brainLevelEmoji: Record<BrainLevel, string> = {
  Newborn:  '🥚',
  Curious:  '🐣',
  Learning: '📚',
  Smart:    '💡',
  Genius:   '🧠',
  Flawless: '✨',
};

export const brainLevelDesc: Record<BrainLevel, string> = {
  Newborn:  'No lessons completed yet.',
  Curious:  'Completed 1–2 lessons. Starting to understand the world.',
  Learning: 'Completed 3–4 lessons. Knowledge is growing quickly.',
  Smart:    'Completed 5–6 lessons. Capable and reliable.',
  Genius:   'Completed 7 lessons. Near peak ability.',
  Flawless: 'All 8 lessons complete. OmniAI is at its best.',
};

// ── Helpers ────────────────────────────────────────────────────────────────────

/** Call after a training phase completes. */
export function markPhaseComplete(phaseKey: string) {
  completedPhases.update(phases => {
    if (!phases.includes(phaseKey)) return [...phases, phaseKey];
    return phases;
  });
}

/** Hydrate from backend response (overrides localStorage). */
export function hydrateFromBackend(meta: BrainMetadata) {
  completedPhases.set(meta.phases_done ?? []);
}
