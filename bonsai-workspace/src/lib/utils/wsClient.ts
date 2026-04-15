/**
 * Desktop-side WebSocket client for Bonsai → VSCode bidirectional relay.
 *
 * The desktop app connects to its own WS server (ws://127.0.0.1:11369/ws)
 * so that it can receive state pushed by the VSCode extension and forward
 * commands from the Bonsai UI back to VSCode.
 *
 * This module also accepts messages from the Android app (same server,
 * different client role).
 */

import {
  vscodeFileTree,
  vscodeEditor,
  vscodeDiagnostics,
  vscodeConnected,
  vscodeRoot,
  applyDelta,
} from '$lib/stores/vscodeState';
import type { VscodeDiagnostic } from '$lib/stores/vscodeState';
import { get } from 'svelte/store';

type VscodeCmd = 'open_file' | 'cursor_set' | 'text_edit' | 'execute_command' | 'show_diff';

let ws: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1000;
const MAX_DELAY = 30_000;
let stopped = false;

export function wsClientStart(token: string): void {
  stopped = false;
  tryConnect(token);
}

export function wsClientStop(): void {
  stopped = true;
  clearReconnect();
  ws?.close();
  ws = null;
  vscodeConnected.set(false);
}

export function sendVscodeCmd(cmd: VscodeCmd, args: Record<string, unknown>): void {
  if (!ws || ws.readyState !== WebSocket.OPEN) return;
  ws.send(JSON.stringify({ type: 'vscode_cmd', payload: { cmd, args } }));
}

// ── Internal ──────────────────────────────────────────────────────────────────

function tryConnect(token: string): void {
  if (stopped) return;

  const url = 'ws://127.0.0.1:11369/ws';
  try {
    ws = new WebSocket(url);
  } catch {
    scheduleReconnect(token);
    return;
  }

  ws.onopen = () => {
    ws!.send(JSON.stringify({ type: 'auth', payload: { token } }));
  };

  ws.onmessage = (ev) => {
    let msg: { type: string; payload: unknown };
    try { msg = JSON.parse(ev.data as string); } catch { return; }
    handleMessage(msg);
  };

  ws.onclose = () => {
    vscodeConnected.set(false);
    scheduleReconnect(token);
  };

  ws.onerror = () => {
    // onclose fires right after, which handles reconnect.
  };
}

function handleMessage(msg: { type: string; payload: unknown }): void {
  switch (msg.type) {
    case 'auth_ok':
      vscodeConnected.set(true);
      reconnectDelay = 1000;
      break;

    case 'vscode_file_tree': {
      const p = msg.payload as { root: string | null; entries: unknown[] };
      vscodeRoot.set(p.root);
      vscodeFileTree.set(p.entries as any);
      break;
    }

    case 'vscode_editor_open':
      vscodeEditor.set(msg.payload as any);
      break;

    case 'vscode_editor_delta': {
      const p  = msg.payload as { path: string; ops: any[] };
      const cur = get(vscodeEditor);
      if (cur && cur.path === p.path) {
        const newContent = applyDelta(cur.content, p.ops);
        vscodeEditor.set({ ...cur, content: newContent });
      }
      break;
    }

    case 'vscode_cursor': {
      const p   = msg.payload as { path: string; line: number; col: number };
      const cur = get(vscodeEditor);
      if (cur && cur.path === p.path) {
        vscodeEditor.set({ ...cur, cursor: { line: p.line, col: p.col } });
      }
      break;
    }

    case 'vscode_diagnostics': {
      const p = msg.payload as { path: string; items: VscodeDiagnostic[] };
      vscodeDiagnostics.set(p.items.map((d) => ({ ...d, path: p.path })));
      break;
    }

    default:
      break;
  }
}

function scheduleReconnect(token: string): void {
  if (stopped) return;
  clearReconnect();
  reconnectTimer = setTimeout(() => {
    reconnectDelay = Math.min(reconnectDelay * 2, MAX_DELAY);
    tryConnect(token);
  }, reconnectDelay);
}

function clearReconnect(): void {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}
