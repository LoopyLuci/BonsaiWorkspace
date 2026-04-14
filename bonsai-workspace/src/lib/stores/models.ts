import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface ModelInfo {
  id:              string;
  name:            string;
  path:            string;
  architecture:    string;
  parameter_count: number;
  context_length:  number;
  quant:           string;
  ram_required_mb: number;
  ram_label:       string;
  valid:           boolean;
}

export interface SlotStatus {
  index:     number;
  port:      number;
  state:     { state: string; model_id?: string; error?: string };
  requests:  number;
  idle_secs: number;
}

export interface OrchestratorStatus {
  slots:       SlotStatus[];
  queue_depth: number;
  total_ram_mb: number;
  free_ram_mb:  number;
}

export interface BootstrapProgress {
  step: string;
  pct:  number;
  msg:  string;
}

// ── Stores ────────────────────────────────────────────────────────────────────

export const availableModels   = writable<ModelInfo[]>([]);
export const orchestratorStatus = writable<OrchestratorStatus | null>(null);
export const activeModelId     = writable<string | null>(null);
export const modelSwitchStatus = writable<string>('');

export const isBootstrapping   = writable(false);
export const bootstrapProgress = writable<Record<string, BootstrapProgress>>({});
export const bootstrapError    = writable<string | null>(null);

// Derived: the first Ready slot's model_id
export const activeModel = derived(
  [availableModels, orchestratorStatus],
  ([$models, $status]) => {
    if (!$status) return null;
    const readySlot = $status.slots.find(s => s.state.state === 'ready');
    if (!readySlot?.state.model_id) return null;
    return $models.find(m => m.id === readySlot.state.model_id) ?? null;
  },
);

// ── Actions ───────────────────────────────────────────────────────────────────

export async function refreshModels() {
  try {
    const models = await invoke<ModelInfo[]>('list_models_registry');
    availableModels.set(models);
  } catch (e) {
    console.error('[models] refresh failed:', e);
  }
}

export async function refreshStatus() {
  try {
    const s = await invoke<OrchestratorStatus>('get_orchestrator_status');
    orchestratorStatus.set(s);
  } catch (e) {
    console.error('[models] status failed:', e);
  }
}

export async function loadModel(modelId: string) {
  activeModelId.set(modelId);
  await invoke('load_model', { model_id: modelId });
}

// ── Event listeners ───────────────────────────────────────────────────────────

let _initialized = false;

export function initModelStores() {
  if (_initialized) return;
  _initialized = true;

  // Bootstrap events
  listen<Record<string, boolean>>('bootstrap-needed', () => {
    isBootstrapping.set(true);
    bootstrapError.set(null);
  });

  listen<BootstrapProgress>('bootstrap-progress', ({ payload }) => {
    bootstrapProgress.update(prev => ({ ...prev, [payload.step]: payload }));
  });

  listen('bootstrap-complete', () => {
    isBootstrapping.set(false);
    bootstrapProgress.set({});
    // Refresh models after bootstrap finishes
    refreshModels();
    refreshStatus();
  });

  listen<string>('bootstrap-error', ({ payload }) => {
    isBootstrapping.set(false);
    bootstrapError.set(payload);
  });

  // Model/orchestrator events
  listen('registry-updated', () => refreshModels());
  listen('orchestrator-status', ({ payload }) => {
    orchestratorStatus.set(payload as OrchestratorStatus);
  });
  listen('model-ready', () => {
    refreshModels();
    refreshStatus();
  });

  // Initial load
  refreshModels();
  refreshStatus();
}
