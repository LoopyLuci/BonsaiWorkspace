import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Mirrors workspace/src-tauri/src/orchestrator_bridge.rs — optional
// visibility into the Python OmniHarness orchestrator's cloud model catalog
// (anthropic/openai/etc., default http://127.0.0.1:8080). Workspace's own
// local hardware-aware model routing (see stores/models.ts) is completely
// separate and unaffected; this only surfaces what the orchestrator knows
// about, when it happens to be running.

export interface OrchestratorModel {
  id: string;
  provider: string;
  context_window: number;
  supports_tools: boolean;
  supports_vision: boolean;
  description: string;
}

export const orchestratorModels = writable<OrchestratorModel[]>([]);

export async function refreshOrchestratorModels(): Promise<void> {
  try {
    const result = await invoke<OrchestratorModel[]>('list_orchestrator_models');
    orchestratorModels.set(Array.isArray(result) ? result : []);
  } catch (e) {
    console.error('[orchestratorBridge] list_orchestrator_models error:', e);
    orchestratorModels.set([]);
  }
}
