import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { apiBaseUrl } from '$lib/stores/settings';
import { swarmEnabled } from '$lib/stores/agents';

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

export const CUSTOM_SWARM_MODEL_ID = '__custom_swarm__';

const CUSTOM_SWARM_MODEL: ModelInfo = {
  id: CUSTOM_SWARM_MODEL_ID,
  name: 'Custom Swarm',
  path: '',
  architecture: 'swarm',
  parameter_count: 0,
  context_length: 0,
  quant: 'multi',
  ram_required_mb: 0,
  ram_label: 'Dynamic',
  valid: true,
};

export const isBootstrapping   = writable(false);
export const bootstrapProgress = writable<Record<string, BootstrapProgress>>({});
export const bootstrapError    = writable<string | null>(null);

// Derived: the active model is either the user-selected model or the first Ready slot.
export const activeModel = derived(
  [availableModels, orchestratorStatus, activeModelId, swarmEnabled],
  ([$models, $status, $activeModelId, $swarmEnabled]) => {
    if ($swarmEnabled || $activeModelId === CUSTOM_SWARM_MODEL_ID) {
      return CUSTOM_SWARM_MODEL;
    }
    if ($activeModelId) {
      return $models.find(m => m.id === $activeModelId) ?? null;
    }
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
    return;
  } catch (e) {
    console.warn('[models] list_models_registry invoke failed, falling back to HTTP:', e);
  }

  // Browser fallback — try the runtime HTTP API
  try {
    const base = get(apiBaseUrl) || 'http://127.0.0.1:11369';
    const resp = await fetch(`${base}/v1/models`);
    const body = await resp.json().catch(() => ({}));
    if (resp.ok && Array.isArray(body.data)) {
      availableModels.set(body.data as ModelInfo[]);
    } else if (Array.isArray(body)) {
      availableModels.set(body as ModelInfo[]);
    } else {
      console.warn('[models] HTTP models response not in expected shape', body);
    }
  } catch (e) {
    console.error('[models] HTTP refresh failed:', e);
  }
}

export async function refreshStatus() {
  try {
    const s = await invoke<OrchestratorStatus>('get_orchestrator_status');
    orchestratorStatus.set(s);
    return;
  } catch (e) {
    console.warn('[models] get_orchestrator_status invoke failed, falling back to HTTP:', e);
  }

  try {
    const base = get(apiBaseUrl) || 'http://127.0.0.1:11369';
    const resp = await fetch(`${base}/v1/orchestrator/status`);
    const body = await resp.json().catch(() => ({}));
    if (resp.ok) {
      orchestratorStatus.set(body as OrchestratorStatus);
    } else {
      console.warn('[models] HTTP orchestrator status error', body);
    }
  } catch (e) {
    console.error('[models] HTTP status fetch failed:', e);
  }
}

export async function loadModel(modelId: string) {
  activeModelId.set(modelId);
  try {
    await invoke('load_model', { modelId });
  } catch (e) {
    // Fallback: try HTTP trigger
    try {
      const base = get(apiBaseUrl) || 'http://127.0.0.1:11369';
      await fetch(`${base}/v1/models/load`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ modelId }) });
    } catch (err) {
      console.error('[models] loadModel failed:', err);
    }
  }
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
