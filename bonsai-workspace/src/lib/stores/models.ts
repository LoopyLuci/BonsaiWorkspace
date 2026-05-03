import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { apiBaseUrl, apiHost, apiPort } from '$lib/stores/settings';
import { DEFAULT_API_HOST, DEFAULT_API_PORT } from '$lib/constants/network';
import { swarmEnabled } from '$lib/stores/agents';
import type {
  ModelData,
  ModelDataSummary,
  GenerateModelDataInput,
} from '$lib/types/model_data';

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
  index:              number;
  port:               number;
  state:              { state: string; model_id?: string; load_pct?: number; error?: string };
  requests:           number;
  idle_secs:          number;
  load_elapsed_secs?: number;
}

export interface ModelLoadProgress {
  slot:          number;
  model_id:      string;
  pct:           number;
  elapsed_secs:  number;
}

export const modelLoadProgress = writable<ModelLoadProgress | null>(null);

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

// ── Model Data stores ─────────────────────────────────────────────────────────

export const modelDataList      = writable<ModelDataSummary[]>([]);
export const modelDataDetail    = writable<Record<string, ModelData>>({});
export const modelDataGenerating = writable(false);
export const modelDataError     = writable<string | null>(null);

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
  // Browser fallback — try the runtime HTTP API. If the configured API
  // endpoint is unreachable, attempt to probe the default port range
  // (preferred port +1..+4) and update the settings store when found.
  try {
    await discoverApiEndpointIfNeeded();
    const base = get(apiBaseUrl) || `http://${DEFAULT_API_HOST}:${DEFAULT_API_PORT}`;
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
    await discoverApiEndpointIfNeeded();
    const base = get(apiBaseUrl) || `http://${DEFAULT_API_HOST}:${DEFAULT_API_PORT}`;
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

// ── Helper: probe local API ports when running in a browser (no Tauri invoke)
async function fetchWithTimeout(input: RequestInfo, init: RequestInit = {}, timeoutMs = 800) {
  const controller = new AbortController();
  const id = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const resp = await fetch(input, { ...init, signal: controller.signal });
    return resp;
  } finally {
    clearTimeout(id);
  }
}

async function probeHealth(host: string, port: number): Promise<boolean> {
  try {
    const url = `http://${host}:${port}/health`;
    const resp = await fetchWithTimeout(url, {}, 700);
    return resp.ok;
  } catch {
    return false;
  }
}

async function discoverApiEndpointIfNeeded() {
  // If Tauri invoke is available and has already set the config, skip probing.
  // Otherwise, try the currently configured port first, then default+1..+4.
  try {
    const currentHost = get(apiHost) || DEFAULT_API_HOST;
    const currentPort = Number(get(apiPort) || DEFAULT_API_PORT);
    if (await probeHealth(currentHost, currentPort)) return;

    // Prefer a persisted port file if available (Tauri host only).
    try {
      // `read_persisted_bot_port` returns an optional port number when running under Tauri.
      // Use invoke when available; in a pure browser environment this will throw and be ignored.
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      const persisted: number | null = await invoke('read_persisted_bot_port').catch(() => null);
      if (persisted && Number.isFinite(persisted)) {
        if (await probeHealth(DEFAULT_API_HOST, persisted)) {
          apiHost.set(DEFAULT_API_HOST);
          apiPort.set(persisted);
          return;
        }
      }
    } catch (e) {
      // ignore — invoke not available in browser
    }

    const tried = new Set<number>();
    const candidates: number[] = [];
    // Prefer current port then the default workspace range
    candidates.push(currentPort);
    for (let i = 0; i <= 4; i++) candidates.push(DEFAULT_API_PORT + i);
    // Also try common local bot/buddy ports (11421, 11420) which tools sometimes use
    const COMMON_BOT_PORTS = [11421, 11420];
    for (const p of COMMON_BOT_PORTS) candidates.push(p);

    for (const p of candidates) {
      if (!Number.isFinite(p) || tried.has(p)) continue;
      tried.add(p);
      // Try both explicit loopback and hostname 'localhost' (covers IPv4/IPv6 cases)
      const hostsToTry = [DEFAULT_API_HOST, 'localhost'];
      for (const h of hostsToTry) {
        if (await probeHealth(h, p)) {
          // update settings so apiBaseUrl reflects reachable endpoint
          console.info(`[models] discovered reachable API at http://${h}:${p}`);
          apiHost.set(h);
          apiPort.set(p);
          return;
        }
      }
    }
  } catch (e) {
    // Non-fatal — leave defaults in place
    console.debug('[models] API discovery failed:', e);
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

// ── Model Data actions ────────────────────────────────────────────────────────

export async function refreshModelData(): Promise<void> {
  try {
    const list = await invoke<ModelDataSummary[]>('list_model_data');
    modelDataList.set(list);
  } catch (e) {
    console.error('[model-data] list failed:', e);
  }
}

export async function fetchModelData(id: string): Promise<ModelData | null> {
  try {
    const data = await invoke<ModelData | null>('get_model_data', { id });
    if (data) modelDataDetail.update(prev => ({ ...prev, [id]: data }));
    return data;
  } catch (e) {
    console.error('[model-data] get failed:', e);
    return null;
  }
}

export async function saveModelData(data: ModelData): Promise<string | null> {
  try {
    const id = await invoke<string>('save_model_data', { data });
    await refreshModelData();
    return id;
  } catch (e) {
    modelDataError.set(String(e));
    return null;
  }
}

export async function deleteModelData(id: string): Promise<boolean> {
  try {
    await invoke('delete_model_data', { id });
    modelDataDetail.update(prev => { const n = { ...prev }; delete n[id]; return n; });
    await refreshModelData();
    return true;
  } catch (e) {
    modelDataError.set(String(e));
    return false;
  }
}

export async function generateModelData(
  input: GenerateModelDataInput,
): Promise<ModelData | null> {
  modelDataGenerating.set(true);
  modelDataError.set(null);
  try {
    return await invoke<ModelData>('generate_model_data', { input });
  } catch (e) {
    modelDataError.set(String(e));
    return null;
  } finally {
    modelDataGenerating.set(false);
  }
}

export async function syncRegistryToModelData(): Promise<number> {
  try {
    const count = await invoke<number>('sync_registry_to_model_data');
    if (count > 0) await refreshModelData();
    return count;
  } catch (e) {
    console.error('[model-data] sync failed:', e);
    return 0;
  }
}

export async function rankModelsForSkill(skillId: string): Promise<ModelDataSummary[]> {
  try {
    return await invoke<ModelDataSummary[]>('rank_models_for_skill', { skillId });
  } catch (e) {
    console.error('[model-data] rank failed:', e);
    return [];
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
  listen('registry-updated', () => { refreshModels(); syncRegistryToModelData(); });
  listen('orchestrator-status', ({ payload }) => {
    orchestratorStatus.set(payload as OrchestratorStatus);
  });
  listen<ModelLoadProgress>('model-load-progress', ({ payload }) => {
    modelLoadProgress.set(payload);
  });
  listen('model-ready', () => {
    modelLoadProgress.set(null);
    refreshModels();
    refreshStatus();
  });

  // Initial load
  refreshModels();
  refreshStatus();
  refreshModelData();
}
