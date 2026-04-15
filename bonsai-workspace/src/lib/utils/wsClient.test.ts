/**
 * Tests for wsClient.ts message routing and store update logic.
 *
 * Strategy: import the module once, use a global MockWebSocket, and
 * call wsClientStop() between tests to reset module-level state.
 * vi.resetModules() is not used because query-string re-imports bypass TS.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';

// ── Minimal WebSocket mock ────────────────────────────────────────────────────

let mockWs: MockWs | null = null;

class MockWs {
  static OPEN    = 1;
  static CLOSING = 2;
  static CLOSED  = 3;

  readyState = MockWs.OPEN;
  sent: string[] = [];

  onopen:    ((e: Event) => void) | null = null;
  onmessage: ((e: { data: string }) => void) | null = null;
  onclose:   ((e: Event) => void) | null = null;
  onerror:   ((e: Event) => void) | null = null;

  send(data: string) { this.sent.push(data); }
  close() { this.readyState = MockWs.CLOSED; }

  // Helpers for test scenarios.
  simulateOpen()              { this.onopen?.(new Event('open')); }
  simulateClose()             { this.readyState = MockWs.CLOSED; this.onclose?.(new Event('close')); }
  receive(data: unknown)      { this.onmessage?.({ data: JSON.stringify(data) }); }
}

// Install global mock before modules are imported.
vi.stubGlobal('WebSocket', class {
  static OPEN    = MockWs.OPEN;
  static CLOSING = MockWs.CLOSING;
  static CLOSED  = MockWs.CLOSED;
  constructor(_url: string) {
    const instance = new MockWs();
    mockWs = instance;
    return instance as unknown as WebSocket;
  }
});

// Import modules once — the mock is already installed.
import { wsClientStart, wsClientStop, sendVscodeCmd } from './wsClient';
import {
  vscodeFileTree, vscodeEditor, vscodeDiagnostics,
  vscodeConnected, vscodeRoot,
} from '../stores/vscodeState';

// ── Reset state between tests ─────────────────────────────────────────────────

beforeEach(() => {
  wsClientStop();          // close any open socket, set stopped = true
  mockWs = null;
  // Reset all stores to initial values.
  vscodeConnected.set(false);
  vscodeFileTree.set([]);
  vscodeEditor.set(null);
  vscodeDiagnostics.set([]);
  vscodeRoot.set(null);
});

afterEach(() => {
  wsClientStop();
});

// Helper: start + auth handshake.
function connect(token = 'MYTOKEN') {
  wsClientStart(token);
  mockWs!.simulateOpen();
}

function auth() {
  mockWs!.receive({ type: 'auth_ok', payload: {} });
}

// ── Auth handshake ────────────────────────────────────────────────────────────

describe('wsClient — auth handshake', () => {
  it('sends auth message with token on open', () => {
    connect('MYTOKEN');
    expect(mockWs!.sent).toHaveLength(1);
    const msg = JSON.parse(mockWs!.sent[0]);
    expect(msg.type).toBe('auth');
    expect(msg.payload.token).toBe('MYTOKEN');
  });

  it('sets vscodeConnected to true on auth_ok', () => {
    connect();
    auth();
    expect(get(vscodeConnected)).toBe(true);
  });

  it('does not set vscodeConnected before receiving auth_ok', () => {
    connect();
    // No auth_ok sent yet.
    expect(get(vscodeConnected)).toBe(false);
  });
});

// ── File tree routing ─────────────────────────────────────────────────────────

describe('wsClient — file tree routing', () => {
  it('updates vscodeFileTree on vscode_file_tree message', () => {
    connect(); auth();
    const entries = [{ name: 'src', path: '/project/src', kind: 'dir' }];
    mockWs!.receive({ type: 'vscode_file_tree', payload: { root: '/project', entries } });
    expect(get(vscodeFileTree)).toEqual(entries);
    expect(get(vscodeRoot)).toBe('/project');
  });

  it('handles empty entries list', () => {
    connect(); auth();
    mockWs!.receive({ type: 'vscode_file_tree', payload: { root: null, entries: [] } });
    expect(get(vscodeFileTree)).toEqual([]);
    expect(get(vscodeRoot)).toBeNull();
  });
});

// ── Editor state routing ──────────────────────────────────────────────────────

describe('wsClient — editor state routing', () => {
  it('sets vscodeEditor on vscode_editor_open', () => {
    connect(); auth();
    const payload = { path: '/a.rs', language: 'rust', content: 'fn main() {}', cursor: { line: 0, col: 0 } };
    mockWs!.receive({ type: 'vscode_editor_open', payload });
    expect(get(vscodeEditor)).toMatchObject(payload);
  });

  it('applies delta to existing editor content on vscode_editor_delta', () => {
    connect(); auth();
    mockWs!.receive({
      type: 'vscode_editor_open',
      payload: { path: '/a.txt', language: 'text', content: 'hello world', cursor: { line: 0, col: 0 } },
    });
    mockWs!.receive({
      type: 'vscode_editor_delta',
      payload: {
        path: '/a.txt',
        ops: [{ range: { startLine: 0, startCol: 6, endLine: 0, endCol: 11 }, newText: 'there' }],
      },
    });
    expect(get(vscodeEditor)?.content).toBe('hello there');
  });

  it('ignores delta for a different file path', () => {
    connect(); auth();
    mockWs!.receive({
      type: 'vscode_editor_open',
      payload: { path: '/a.txt', language: 'text', content: 'original', cursor: { line: 0, col: 0 } },
    });
    mockWs!.receive({
      type: 'vscode_editor_delta',
      payload: { path: '/b.txt', ops: [{ range: { startLine: 0, startCol: 0, endLine: 0, endCol: 8 }, newText: 'changed' }] },
    });
    expect(get(vscodeEditor)?.content).toBe('original');
  });

  it('clears editor on null vscode_editor_open payload', () => {
    connect(); auth();
    mockWs!.receive({
      type: 'vscode_editor_open',
      payload: { path: '/a.rs', language: 'rust', content: 'x', cursor: { line: 0, col: 0 } },
    });
    mockWs!.receive({ type: 'vscode_editor_open', payload: null });
    expect(get(vscodeEditor)).toBeNull();
  });
});

// ── Cursor routing ────────────────────────────────────────────────────────────

describe('wsClient — cursor routing', () => {
  it('updates cursor in vscodeEditor on vscode_cursor', () => {
    connect(); auth();
    mockWs!.receive({
      type: 'vscode_editor_open',
      payload: { path: '/x.ts', language: 'typescript', content: 'const x = 1;', cursor: { line: 0, col: 0 } },
    });
    mockWs!.receive({ type: 'vscode_cursor', payload: { path: '/x.ts', line: 5, col: 12 } });
    expect(get(vscodeEditor)?.cursor).toEqual({ line: 5, col: 12 });
  });

  it('ignores cursor for a different file path', () => {
    connect(); auth();
    mockWs!.receive({
      type: 'vscode_editor_open',
      payload: { path: '/x.ts', language: 'typescript', content: '', cursor: { line: 0, col: 0 } },
    });
    mockWs!.receive({ type: 'vscode_cursor', payload: { path: '/y.ts', line: 99, col: 99 } });
    expect(get(vscodeEditor)?.cursor).toEqual({ line: 0, col: 0 });
  });
});

// ── Diagnostics routing ───────────────────────────────────────────────────────

describe('wsClient — diagnostics routing', () => {
  it('replaces vscodeDiagnostics on vscode_diagnostics message', () => {
    connect(); auth();
    const items = [{ line: 1, col: 0, severity: 'error', message: 'missing semicolon', source: 'ts' }];
    mockWs!.receive({ type: 'vscode_diagnostics', payload: { path: '/a.ts', items } });
    const diags = get(vscodeDiagnostics);
    expect(diags).toHaveLength(1);
    expect(diags[0].message).toBe('missing semicolon');
    expect(diags[0].path).toBe('/a.ts');
  });

  it('clears diagnostics when items is empty', () => {
    connect(); auth();
    mockWs!.receive({ type: 'vscode_diagnostics', payload: { path: '/a.ts', items: [{ line:0, col:0, severity:'error', message:'x', source:'ts' }] } });
    mockWs!.receive({ type: 'vscode_diagnostics', payload: { path: '/a.ts', items: [] } });
    expect(get(vscodeDiagnostics)).toHaveLength(0);
  });
});

// ── sendVscodeCmd ─────────────────────────────────────────────────────────────

describe('wsClient — sendVscodeCmd', () => {
  it('sends a vscode_cmd JSON message over the WebSocket', () => {
    connect(); auth();
    mockWs!.sent = []; // clear auth message

    sendVscodeCmd('open_file', { path: '/project/main.rs' });

    expect(mockWs!.sent).toHaveLength(1);
    const msg = JSON.parse(mockWs!.sent[0]);
    expect(msg.type).toBe('vscode_cmd');
    expect(msg.payload.cmd).toBe('open_file');
    expect(msg.payload.args.path).toBe('/project/main.rs');
  });

  it('sends cursor_set command with correct args', () => {
    connect(); auth();
    mockWs!.sent = [];
    sendVscodeCmd('cursor_set', { path: '/a.ts', line: 10, col: 5 });
    const msg = JSON.parse(mockWs!.sent[0]);
    expect(msg.payload.cmd).toBe('cursor_set');
    expect(msg.payload.args).toMatchObject({ line: 10, col: 5 });
  });

  it('does not throw if called when not connected', () => {
    // wsClientStop() was called in beforeEach — no active socket.
    expect(() => sendVscodeCmd('open_file', { path: '/x' })).not.toThrow();
  });
});

// ── Disconnection ─────────────────────────────────────────────────────────────

describe('wsClient — disconnection', () => {
  it('sets vscodeConnected to false after wsClientStop', () => {
    connect(); auth();
    expect(get(vscodeConnected)).toBe(true);
    wsClientStop();
    expect(get(vscodeConnected)).toBe(false);
  });
});
