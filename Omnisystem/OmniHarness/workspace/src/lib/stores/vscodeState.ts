import { writable } from 'svelte/store';

// ── File tree ─────────────────────────────────────────────────────────────────

export interface VscodeFileEntry {
  name: string;
  path: string;
  kind: 'file' | 'dir';
  children?: VscodeFileEntry[];
}

// ── Editor ───────────────────────────────────────────────────────────────────

export interface VscodeEditorState {
  path:     string;
  language: string;
  content:  string;
  cursor:   { line: number; col: number };
}

// ── Diagnostics ───────────────────────────────────────────────────────────────

export interface VscodeDiagnostic {
  path:     string;
  line:     number;
  col:      number;
  severity: 'error' | 'warning' | 'info' | 'hint';
  message:  string;
  source:   string;
}

// ── Stores ────────────────────────────────────────────────────────────────────

export const vscodeFileTree    = writable<VscodeFileEntry[]>([]);
export const vscodeEditor      = writable<VscodeEditorState | null>(null);
export const vscodeDiagnostics = writable<VscodeDiagnostic[]>([]);
export const vscodeCopilot     = writable<string>('');
export const vscodeConnected   = writable<boolean>(false);
export const vscodeRoot        = writable<string | null>(null);

// ── Delta application ─────────────────────────────────────────────────────────

interface DeltaOp {
  range: { startLine: number; startCol: number; endLine: number; endCol: number };
  newText: string;
}

/**
 * Apply a VSCode-style ranged delta to the in-memory editor content.
 * Operates on the string directly — no OT library needed because VSCode
 * already provides absolute character ranges.
 */
export function applyDelta(content: string, ops: DeltaOp[]): string {
  // Sort ops in reverse order so applying one doesn't shift offsets for others.
  const sorted = [...ops].sort((a, b) => {
    if (b.range.startLine !== a.range.startLine) return b.range.startLine - a.range.startLine;
    return b.range.startCol - a.range.startCol;
  });

  const lines = content.split('\n');

  for (const op of sorted) {
    const { startLine, startCol, endLine, endCol } = op.range;
    const before = lines.slice(0, startLine);
    const after  = lines.slice(endLine + 1);
    const prefix = (lines[startLine] ?? '').slice(0, startCol);
    const suffix = (lines[endLine]   ?? '').slice(endCol);
    const newLines = (prefix + op.newText + suffix).split('\n');
    lines.splice(0, lines.length, ...before, ...newLines, ...after);
  }

  return lines.join('\n');
}
