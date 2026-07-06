import { invoke } from '@tauri-apps/api/core';
import { derived, get, writable } from 'svelte/store';

export type CanvasInteractionMode = 'select' | 'pan' | 'connect' | 'note' | 'editing-text' | 'resizing';

export interface CanvasNode {
  id: string;
  type: 'file' | 'note' | 'chat';
  x: number;
  y: number;
  width: number;
  height: number;
  title?: string;
  filePath?: string;
  noteText?: string;
  chatContent?: string;
  color?: string;
  collapsed?: boolean;
  zIndex?: number;
}

export interface CanvasConnection {
  id: string;
  fromNodeId: string;
  toNodeId: string;
  label?: string;
}

export interface CanvasViewport {
  x: number;
  y: number;
  zoom: number;
}

export interface CanvasLayout {
  schema_version: number;
  saved_at: string | null;
  viewport: CanvasViewport;
  nodes: CanvasNode[];
  connections: CanvasConnection[];
}

export interface CanvasLoadResult {
  layout: CanvasLayout;
  recovered_corrupt_file?: string;
}

export interface FileConflictState {
  filePath: string;
  status: 'stale' | 'conflict';
  message: string;
  detectedAt: string;
}

export interface PendingConnection {
  fromNodeId: string;
  x: number;
  y: number;
}

const DEFAULT_LAYOUT: CanvasLayout = {
  schema_version: 1,
  saved_at: null,
  viewport: { x: 0, y: 0, zoom: 1 },
  nodes: [],
  connections: [],
};

export const canvasLayout = writable<CanvasLayout>(DEFAULT_LAYOUT);
export const canvasMode = writable<CanvasInteractionMode>('select');
export const canvasConflictMap = writable<Record<string, FileConflictState>>({});
export const canvasCorruptionNotice = writable<string>('');
export const selectedNodeIds = writable<string[]>([]);
export const pendingConnection = writable<PendingConnection | null>(null);
export const maxNodeZ = derived(canvasLayout, ($layout) => {
  if (!$layout.nodes.length) return 0;
  return Math.max(...$layout.nodes.map((node) => node.zIndex ?? 0));
});

let activeGestureMode: CanvasInteractionMode | null = null;

export function setCanvasMode(next: CanvasInteractionMode) {
  if (activeGestureMode && next !== activeGestureMode && next !== 'select') {
    return;
  }
  canvasMode.set(next);
}

export function beginCanvasGesture(mode: CanvasInteractionMode) {
  if (activeGestureMode && activeGestureMode !== mode) {
    return false;
  }
  activeGestureMode = mode;
  canvasMode.set(mode);
  return true;
}

export function endCanvasGesture() {
  activeGestureMode = null;
  canvasMode.set('select');
}

export function cancelCanvasTransientMode() {
  activeGestureMode = null;
  canvasMode.set('select');
  pendingConnection.set(null);
}

function uid(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`;
}

function nextZ(layout: CanvasLayout) {
  if (!layout.nodes.length) return 1;
  return Math.max(...layout.nodes.map((node) => node.zIndex ?? 0)) + 1;
}

function updateLayout(mutator: (layout: CanvasLayout) => CanvasLayout) {
  canvasLayout.update((layout) => mutator(layout));
}

export function setViewport(viewport: CanvasViewport) {
  updateLayout((layout) => ({ ...layout, viewport }));
}

export function addFileNode(filePath: string, label: string, x: number, y: number) {
  updateLayout((layout) => ({
    ...layout,
    nodes: [
      ...layout.nodes,
      {
        id: uid('file'),
        type: 'file',
        x,
        y,
        width: 520,
        height: 340,
        filePath,
        title: label,
        color: '#4a9eff',
        collapsed: false,
        zIndex: nextZ(layout),
      },
    ],
  }));
}

export function addStickyNote(x: number, y: number) {
  updateLayout((layout) => ({
    ...layout,
    nodes: [
      ...layout.nodes,
      {
        id: uid('note'),
        type: 'note',
        x,
        y,
        width: 320,
        height: 220,
        title: 'Sticky Note',
        noteText: '',
        color: '#fbbf24',
        collapsed: false,
        zIndex: nextZ(layout),
      },
    ],
  }));
}

export function addChatNode(content: string, x: number, y: number) {
  updateLayout((layout) => ({
    ...layout,
    nodes: [
      ...layout.nodes,
      {
        id: uid('chat'),
        type: 'chat',
        x,
        y,
        width: 420,
        height: 240,
        title: 'Chat Snapshot',
        chatContent: content,
        color: '#16a34a',
        collapsed: false,
        zIndex: nextZ(layout),
      },
    ],
  }));
}

export function updateNode(id: string, patch: Partial<CanvasNode>) {
  updateLayout((layout) => ({
    ...layout,
    nodes: layout.nodes.map((node) => (node.id === id ? { ...node, ...patch } : node)),
  }));
}

export function bringNodeToFront(id: string) {
  updateLayout((layout) => ({
    ...layout,
    nodes: layout.nodes.map((node) =>
      node.id === id ? { ...node, zIndex: nextZ(layout) } : node,
    ),
  }));
}

export function deleteNode(id: string) {
  updateLayout((layout) => ({
    ...layout,
    nodes: layout.nodes.filter((node) => node.id !== id),
    connections: layout.connections.filter(
      (connection) => connection.fromNodeId !== id && connection.toNodeId !== id,
    ),
  }));
  selectedNodeIds.update((ids) => ids.filter((nodeId) => nodeId !== id));
}

export function deleteSelectedNodes() {
  const ids = get(selectedNodeIds);
  if (!ids.length) return;
  for (const id of ids) {
    deleteNode(id);
  }
  selectedNodeIds.set([]);
}

export function addConnection(fromNodeId: string, toNodeId: string) {
  if (fromNodeId === toNodeId) return;
  const layout = get(canvasLayout);
  if (
    layout.connections.some(
      (connection) =>
        connection.fromNodeId === fromNodeId && connection.toNodeId === toNodeId,
    )
  ) {
    return;
  }
  updateLayout((nextLayout) => ({
    ...nextLayout,
    connections: [
      ...nextLayout.connections,
      {
        id: uid('link'),
        fromNodeId,
        toNodeId,
        label: '',
      },
    ],
  }));
}

export function updateConnection(id: string, patch: Partial<CanvasConnection>) {
  updateLayout((layout) => ({
    ...layout,
    connections: layout.connections.map((connection) =>
      connection.id === id ? { ...connection, ...patch } : connection,
    ),
  }));
}

export function deleteConnection(id: string) {
  updateLayout((layout) => ({
    ...layout,
    connections: layout.connections.filter((connection) => connection.id !== id),
  }));
}

export function beginConnectionDrag(fromNodeId: string, x: number, y: number) {
  pendingConnection.set({ fromNodeId, x, y });
}

export function updatePendingConnection(x: number, y: number) {
  pendingConnection.update((pending) => (pending ? { ...pending, x, y } : pending));
}

export function completePendingConnection(toNodeId: string) {
  const pending = get(pendingConnection);
  if (!pending) return;
  addConnection(pending.fromNodeId, toNodeId);
  pendingConnection.set(null);
}

export function clearPendingConnection() {
  pendingConnection.set(null);
}

export function setSelectedNode(nodeId: string | null, append = false) {
  if (!nodeId) {
    selectedNodeIds.set([]);
    return;
  }
  if (!append) {
    selectedNodeIds.set([nodeId]);
    return;
  }
  selectedNodeIds.update((ids) => {
    if (ids.includes(nodeId)) return ids;
    return [...ids, nodeId];
  });
}

export function fitCanvasView(viewWidth: number, viewHeight: number) {
  const layout = get(canvasLayout);
  if (!layout.nodes.length) {
    setViewport({ x: 80, y: 80, zoom: 1 });
    return;
  }

  const minX = Math.min(...layout.nodes.map((node) => node.x));
  const minY = Math.min(...layout.nodes.map((node) => node.y));
  const maxX = Math.max(...layout.nodes.map((node) => node.x + node.width));
  const maxY = Math.max(...layout.nodes.map((node) => node.y + node.height));

  const worldWidth = Math.max(1, maxX - minX);
  const worldHeight = Math.max(1, maxY - minY);
  const padding = 80;
  const zoom = Math.max(
    0.08,
    Math.min(2.6, Math.min((viewWidth - padding) / worldWidth, (viewHeight - padding) / worldHeight)),
  );

  const centeredWidth = worldWidth * zoom;
  const centeredHeight = worldHeight * zoom;
  const x = (viewWidth - centeredWidth) / 2 - minX * zoom;
  const y = (viewHeight - centeredHeight) / 2 - minY * zoom;
  setViewport({ x, y, zoom });
}

export function markCanvasFileConflict(filePath: string, status: 'stale' | 'conflict', message: string) {
  canvasConflictMap.update((conflicts) => ({
    ...conflicts,
    [filePath]: {
      filePath,
      status,
      message,
      detectedAt: new Date().toISOString(),
    },
  }));
}

export function clearCanvasFileConflict(filePath: string) {
  canvasConflictMap.update((conflicts) => {
    if (!(filePath in conflicts)) return conflicts;
    const next = { ...conflicts };
    delete next[filePath];
    return next;
  });
}

export async function loadCanvasLayout(workspacePath: string): Promise<CanvasLayout> {
  const result = await invoke<CanvasLoadResult>('load_canvas_layout', { workspacePath });
  if (result.recovered_corrupt_file) {
    canvasCorruptionNotice.set(`Recovered corrupt canvas file: ${result.recovered_corrupt_file}`);
  } else {
    canvasCorruptionNotice.set('');
  }
  const loaded = result.layout ?? DEFAULT_LAYOUT;
  canvasLayout.set(loaded);
  selectedNodeIds.set([]);
  pendingConnection.set(null);
  return loaded;
}

export async function saveCanvasLayout(workspacePath: string, layout: CanvasLayout): Promise<void> {
  await invoke('save_canvas_layout', { workspacePath, layout });
  canvasLayout.set(layout);
}

export async function saveCanvas(workspacePath: string): Promise<void> {
  const layout = get(canvasLayout);
  await saveCanvasLayout(workspacePath, layout);
}

export function resetCanvasLayout() {
  canvasLayout.set(DEFAULT_LAYOUT);
  canvasConflictMap.set({});
  canvasCorruptionNotice.set('');
  selectedNodeIds.set([]);
  pendingConnection.set(null);
  cancelCanvasTransientMode();
}
