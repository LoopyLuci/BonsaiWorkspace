// ── Bonsai model catalog ───────────────────────────────────────────────────────
// Static list of available Bonsai model variants with HuggingFace download URLs.
// The registry (scanned from disk) uses hash-based IDs; we match by filename.

import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { availableModels, orchestratorStatus, refreshModels, modelSwitchStatus, activeModelId } from './models';
import type { ModelInfo } from './models';

export interface CatalogEntry {
  catalogId:   string;
  name:        string;
  description: string;
  params:      string;
  quant:       string;
  ramGb:       number;
  downloadUrl: string;
  fileName:    string;
  isDefault:   boolean;
}

export const BONSAI_CATALOG: CatalogEntry[] = [
  {
    catalogId:   'bonsai-1.7b',
    name:        'Bonsai 1.7B',
    description: 'Fast & lightweight · BitNet 1-bit · ~237 MB',
    params:      '1.7B',
    quant:       '1-bit',
    ramGb:       1,
    downloadUrl: 'https://huggingface.co/prism-ml/Bonsai-1.7B-gguf/resolve/main/Bonsai-1.7B.gguf',
    fileName:    'Bonsai-1.7B.gguf',
    isDefault:   true,
  },
  {
    catalogId:   'bonsai-4b',
    name:        'Bonsai 4B',
    description: 'Balanced code reasoning · BitNet 1-bit · ~546 MB',
    params:      '4B',
    quant:       '1-bit',
    ramGb:       1,
    downloadUrl: 'https://huggingface.co/prism-ml/Bonsai-4B-gguf/resolve/main/Bonsai-4B.gguf',
    fileName:    'Bonsai-4B.gguf',
    isDefault:   false,
  },
  {
    catalogId:   'bonsai-8b',
    name:        'Bonsai 8B',
    description: 'Most capable · BitNet 1-bit · ~1.1 GB',
    params:      '8B',
    quant:       '1-bit',
    ramGb:       2,
    downloadUrl: 'https://huggingface.co/prism-ml/Bonsai-8B-gguf/resolve/main/Bonsai-8B.gguf',
    fileName:    'Bonsai-8B.gguf',
    isDefault:   false,
  },
];

// ── Stores ────────────────────────────────────────────────────────────────────

export const autoMode          = writable(false);
export const downloadingId     = writable<string | null>(null);
export const downloadPct       = writable(0);
export const downloadError     = writable<string | null>(null);

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Find the registry ModelInfo for a catalog entry by filename. */
export function findRegistryModel(entry: CatalogEntry, models: ModelInfo[]): ModelInfo | null {
  return models.find(m => {
    const base = m.path.split(/[/\\]/).pop() ?? '';
    return base.toLowerCase() === entry.fileName.toLowerCase();
  }) ?? null;
}

/** Pick the largest catalog model whose RAM requirement fits within free RAM. */
export function bestModelForRam(freeRamMb: number, models: ModelInfo[]): ModelInfo | null {
  const byRam = [...models].sort((a, b) => b.ram_required_mb - a.ram_required_mb);
  return byRam.find(m => m.ram_required_mb <= freeRamMb * 0.85) ?? null;
}

// ── Actions ───────────────────────────────────────────────────────────────────

let _progressUnlisten: (() => void) | null = null;

export async function downloadCatalogModel(entry: CatalogEntry): Promise<void> {
  downloadingId.set(entry.catalogId);
  downloadPct.set(0);
  downloadError.set(null);

  if (!_progressUnlisten) {
    _progressUnlisten = await listen<{ progress: number }>(
      'download-progress',
      (e) => downloadPct.set(e.payload.progress),
    );
  }

  try {
    await invoke('download_gguf_model', {
      url:      entry.downloadUrl,
      fileName: entry.fileName,
    });
    await refreshModels();
  } catch (e) {
    downloadError.set(String(e));
  } finally {
    downloadingId.set(null);
    downloadPct.set(0);
  }
}

export async function switchToCatalogModel(entry: CatalogEntry): Promise<void> {
  const models = get(availableModels);
  const reg = findRegistryModel(entry, models);
  if (!reg) {
    downloadError.set(`${entry.name} is not downloaded yet.`);
    return;
  }
  activeModelId.set(reg.id);
  modelSwitchStatus.set(`Switching to ${entry.name}…`);
  try {
    const msg = await invoke<string>('switch_model', { modelId: reg.id });
    modelSwitchStatus.set(msg);
  } catch (e) {
    modelSwitchStatus.set(`Switch failed: ${e}`);
  }
}

export async function triggerAutoSelect(): Promise<void> {
  const status = get(orchestratorStatus);
  const models = get(availableModels);
  const freeRam = status?.free_ram_mb ?? 0;

  if (models.length === 0) {
    // No local models — default to 1.7B download suggestion
    downloadError.set('No models available. Download Bonsai 1.7B to get started.');
    return;
  }

  const best = bestModelForRam(freeRam, models);
  if (!best) {
    downloadError.set('Not enough free RAM for any loaded model.');
    return;
  }

  modelSwitchStatus.set(`Auto: switching to ${best.name}…`);
  try {
    const msg = await invoke<string>('switch_model', { modelId: best.id });
    modelSwitchStatus.set(`Auto: ${msg}`);
  } catch (e) {
    modelSwitchStatus.set(`Auto switch failed: ${e}`);
  }
}
