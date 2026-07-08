import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface RemoteCursor {
  peerId: string;
  displayName: string;
  file: string;
  line: number;
  column: number;
  color: string;
}

export interface CrdtOp {
  file: string;
  op_type: 'insert' | 'delete';
  position: number;
  text?: string;
  length?: number;
  peer_id: string;
  timestamp: number;
}

export const remoteCursors = writable<Map<string, RemoteCursor>>(new Map());
export const crdtOperations = writable<CrdtOp[]>([]);
export const activeFile = writable<string>('');

const PEER_COLORS = ['#f87171', '#fb923c', '#facc15', '#4ade80', '#38bdf8', '#a78bfa', '#f472b6'];
const colorMap = new Map<string, string>();
let colorIdx = 0;

function peerColor(peerId: string): string {
  if (!colorMap.has(peerId)) {
    colorMap.set(peerId, PEER_COLORS[colorIdx % PEER_COLORS.length]);
    colorIdx++;
  }
  return colorMap.get(peerId)!;
}

let unlisteners: Array<() => void> = [];
let initialized = false;

export async function initCrdtListeners() {
  if (initialized) return;
  initialized = true;

  unlisteners.push(await listen('collab-crdt-operation', (event: any) => {
    const op = event.payload.operation as CrdtOp;
    crdtOperations.update(ops => [...ops, op]);
  }));

  unlisteners.push(await listen('collab-cursor-update', (event: any) => {
    const { peer_id, file, line, column } = event.payload;
    remoteCursors.update(m => {
      const updated = new Map(m);
      const existing = updated.get(peer_id);
      updated.set(peer_id, {
        peerId: peer_id,
        displayName: existing?.displayName ?? peer_id.slice(0, 8),
        file,
        line,
        column,
        color: peerColor(peer_id),
      });
      return updated;
    });
  }));

  unlisteners.push(await listen('collab-participant-left', (event: any) => {
    const { peer_id } = event.payload;
    remoteCursors.update(m => { const n = new Map(m); n.delete(peer_id); return n; });
  }));
}

export function destroyCrdtListeners() {
  unlisteners.forEach(fn => fn());
  unlisteners = [];
  initialized = false;
}

export async function sendCursorPosition(sessionId: string, peerId: string, file: string, line: number, column: number) {
  await invoke('send_cursor_position', { sessionId, peerId, file, line, column }).catch(() => {});
}

export async function sendCrdtOperation(sessionId: string, op: Omit<CrdtOp, 'timestamp'>) {
  const full: CrdtOp = { ...op, timestamp: Date.now() };
  await invoke('send_crdt_operation', { sessionId, operation: full }).catch(() => {});
}
