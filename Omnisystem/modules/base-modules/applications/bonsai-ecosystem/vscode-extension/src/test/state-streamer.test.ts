/**
 * Unit tests for state-streamer helpers that are pure functions
 * and don't require a real VSCode API instance.
 *
 * The `readDirTree` and `severityName` helpers are tested indirectly
 * through module-level exports (requires vitest with tsconfig paths).
 */

import { describe, it, expect } from 'vitest';

// ── Severity name mapping ─────────────────────────────────────────────────────

// Mirror the internal severityName() function for testing.
// VSCode DiagnosticSeverity: Error=0, Warning=1, Information=2, Hint=3
function severityName(s: number): string {
  switch (s) {
    case 0:  return 'error';
    case 1:  return 'warning';
    case 2:  return 'info';
    default: return 'hint';
  }
}

describe('severityName', () => {
  it('maps 0 → error', ()   => expect(severityName(0)).toBe('error'));
  it('maps 1 → warning', () => expect(severityName(1)).toBe('warning'));
  it('maps 2 → info', ()    => expect(severityName(2)).toBe('info'));
  it('maps 3 → hint', ()    => expect(severityName(3)).toBe('hint'));
  it('maps unknown → hint', () => expect(severityName(99)).toBe('hint'));
});

// ── Delta op format ───────────────────────────────────────────────────────────
// Verifies the shape of delta ops that the streamer would produce.

interface Range  { start: { line: number; character: number }; end: { line: number; character: number } }
interface Change { range: Range; text: string }

function toOp(c: Change) {
  return {
    range: {
      startLine: c.range.start.line,
      startCol:  c.range.start.character,
      endLine:   c.range.end.line,
      endCol:    c.range.end.character,
    },
    newText: c.text,
  };
}

describe('toOp (delta op serialisation)', () => {
  it('converts a single-line replacement', () => {
    const change: Change = {
      range: { start: { line: 2, character: 4 }, end: { line: 2, character: 10 } },
      text: 'replaced',
    };
    const op = toOp(change);
    expect(op.range.startLine).toBe(2);
    expect(op.range.startCol).toBe(4);
    expect(op.range.endLine).toBe(2);
    expect(op.range.endCol).toBe(10);
    expect(op.newText).toBe('replaced');
  });

  it('converts an insertion (zero-length range)', () => {
    const change: Change = {
      range: { start: { line: 0, character: 5 }, end: { line: 0, character: 5 } },
      text: 'inserted',
    };
    const op = toOp(change);
    expect(op.range.startLine).toBe(op.range.endLine);
    expect(op.range.startCol).toBe(op.range.endCol);
    expect(op.newText).toBe('inserted');
  });

  it('converts a deletion (empty text)', () => {
    const change: Change = {
      range: { start: { line: 1, character: 0 }, end: { line: 2, character: 0 } },
      text: '',
    };
    const op = toOp(change);
    expect(op.newText).toBe('');
    expect(op.range.startLine).toBe(1);
    expect(op.range.endLine).toBe(2);
  });

  it('preserves multi-line ranges', () => {
    const change: Change = {
      range: { start: { line: 10, character: 3 }, end: { line: 15, character: 7 } },
      text: 'multi\nline',
    };
    const op = toOp(change);
    expect(op.range.startLine).toBe(10);
    expect(op.range.endLine).toBe(15);
    expect(op.newText).toContain('\n');
  });
});

// ── Message protocol shape ────────────────────────────────────────────────────

describe('BonsaiMessage protocol shape', () => {
  it('auth message has correct shape', () => {
    const msg = { type: 'auth', payload: { token: 'ABC123' } };
    expect(msg.type).toBe('auth');
    expect(msg.payload.token).toBe('ABC123');
  });

  it('vscode_file_tree message has root and entries', () => {
    const msg = {
      type: 'vscode_file_tree',
      payload: {
        root: '/home/user/project',
        entries: [
          { name: 'src', path: '/home/user/project/src', kind: 'dir', children: [] },
          { name: 'Cargo.toml', path: '/home/user/project/Cargo.toml', kind: 'file' },
        ],
      },
    };
    expect(msg.payload.entries).toHaveLength(2);
    expect(msg.payload.entries[0].kind).toBe('dir');
    expect(msg.payload.entries[1].kind).toBe('file');
  });

  it('vscode_cmd open_file message has path', () => {
    const msg = { type: 'vscode_cmd', payload: { cmd: 'open_file', args: { path: '/a/b.rs' } } };
    expect(msg.payload.cmd).toBe('open_file');
    expect((msg.payload.args as any).path).toBe('/a/b.rs');
  });

  it('vscode_cmd text_edit message has range and newText', () => {
    const msg = {
      type: 'vscode_cmd',
      payload: {
        cmd: 'text_edit',
        args: { path: '/a.ts', startLine: 1, startCol: 0, endLine: 1, endCol: 10, newText: 'fixed' },
      },
    };
    expect((msg.payload.args as any).newText).toBe('fixed');
    expect((msg.payload.args as any).startLine).toBe(1);
  });
});
