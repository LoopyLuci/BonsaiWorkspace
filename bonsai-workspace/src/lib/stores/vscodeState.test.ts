/**
 * Tests for vscodeState.ts — focusing on applyDelta() since it is
 * pure logic with no browser / Tauri dependencies.
 */

import { describe, it, expect } from 'vitest';
import { applyDelta } from './vscodeState';

// ── applyDelta ────────────────────────────────────────────────────────────────

describe('applyDelta', () => {
  it('inserts text at a position (zero-length range)', () => {
    const content = 'hello world';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 5, endLine: 0, endCol: 5 }, newText: ' beautiful' },
    ]);
    expect(result).toBe('hello beautiful world');
  });

  it('replaces a range within a single line', () => {
    const content = 'foo bar baz';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 4, endLine: 0, endCol: 7 }, newText: 'qux' },
    ]);
    expect(result).toBe('foo qux baz');
  });

  it('deletes a range (empty newText)', () => {
    const content = 'hello world';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 5, endLine: 0, endCol: 11 }, newText: '' },
    ]);
    expect(result).toBe('hello');
  });

  it('handles multi-line replacement', () => {
    const content = 'line1\nline2\nline3';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 4, endLine: 1, endCol: 5 }, newText: 'X\nY' },
    ]);
    expect(result).toBe('lineX\nY\nline3');
  });

  it('applies multiple non-overlapping ops in correct order', () => {
    const content = 'abc\ndef\nghi';
    // Two ops on different lines — must be sorted in reverse line order
    // so applying one does not shift the offset of the other.
    const result = applyDelta(content, [
      { range: { startLine: 2, startCol: 0, endLine: 2, endCol: 3 }, newText: 'GHI' },
      { range: { startLine: 0, startCol: 0, endLine: 0, endCol: 3 }, newText: 'ABC' },
    ]);
    expect(result).toBe('ABC\ndef\nGHI');
  });

  it('replaces entire content with a single op', () => {
    const content = 'old content\nspanning lines';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 0, endLine: 1, endCol: 14 }, newText: 'new' },
    ]);
    expect(result).toBe('new');
  });

  it('handles empty input and empty op', () => {
    const result = applyDelta('', [
      { range: { startLine: 0, startCol: 0, endLine: 0, endCol: 0 }, newText: 'hello' },
    ]);
    expect(result).toBe('hello');
  });

  it('no-op delta returns same content', () => {
    const content = 'unchanged';
    const result = applyDelta(content, []);
    expect(result).toBe('unchanged');
  });

  it('preserves surrounding context on single-char insert', () => {
    // 'fn foo() {}': ( is col 6, ) is col 7. Insert before ) at col 7.
    const content = 'fn foo() {}';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 7, endLine: 0, endCol: 7 }, newText: 'x: i32' },
    ]);
    expect(result).toBe('fn foo(x: i32) {}');
  });

  it('handles newline insertion (splitting a line)', () => {
    const content = 'ab';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 1, endLine: 0, endCol: 1 }, newText: '\n' },
    ]);
    expect(result).toBe('a\nb');
  });

  it('handles newline deletion (joining lines)', () => {
    const content = 'a\nb';
    const result = applyDelta(content, [
      { range: { startLine: 0, startCol: 1, endLine: 1, endCol: 0 }, newText: '' },
    ]);
    expect(result).toBe('ab');
  });
});

// ── Store exports are writable Svelte stores ──────────────────────────────────

describe('vscodeState stores', () => {
  it('exports are defined', async () => {
    const mod = await import('./vscodeState');
    expect(mod.vscodeFileTree).toBeDefined();
    expect(mod.vscodeEditor).toBeDefined();
    expect(mod.vscodeDiagnostics).toBeDefined();
    expect(mod.vscodeConnected).toBeDefined();
    expect(mod.vscodeRoot).toBeDefined();
    expect(mod.vscodeCopilot).toBeDefined();
  });

  it('vscodeConnected starts false', async () => {
    const { vscodeConnected } = await import('./vscodeState');
    let value: boolean = true;
    const unsub = vscodeConnected.subscribe((v) => (value = v));
    expect(value).toBe(false);
    unsub();
  });

  it('vscodeEditor starts null', async () => {
    const { vscodeEditor } = await import('./vscodeState');
    let value: unknown = 'sentinel';
    const unsub = vscodeEditor.subscribe((v) => (value = v));
    expect(value).toBeNull();
    unsub();
  });

  it('vscodeFileTree starts empty', async () => {
    const { vscodeFileTree } = await import('./vscodeState');
    let value: unknown[] = [{}];
    const unsub = vscodeFileTree.subscribe((v) => (value = v));
    expect(value).toEqual([]);
    unsub();
  });
});
