import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust types in src-tauri/src/swarm_config.rs.

// Rust's `WorkerRole` enum marks its `Custom(String)` variant
// `#[serde(untagged)]`, so on the wire EVERY variant (built-in or custom) is
// just a bare JSON string — the 6 known snake_case names, or any other
// string for a custom role. There is no `{ custom: "..." }` wrapper.
export type WorkerRole = string;

export type ChainStrategy =
  | 'parallel_then_synthesize' | 'sequential_gate' | 'parallel_vote' | 'dev_pipeline';

export interface SwarmWorkerConfig {
  slot: number;
  role: WorkerRole;
  model: string | null;
  adapter: string | null;
  gpu_layers: number;
  system_prompt: string | null;
  allowed_tools: string[] | null;
  priority: number;
}

export interface SwarmConfig {
  id: string;
  name: string;
  description: string;
  workers: SwarmWorkerConfig[];
  chain_strategy: ChainStrategy;
  synthesis_model: string | null;
  enabled: boolean;
  created_at: string;
  last_used_at: string | null;
}

export const BUILT_IN_ROLES: { value: WorkerRole; label: string }[] = [
  { value: 'implementer', label: 'Implementer' },
  { value: 'reviewer', label: 'Reviewer' },
  { value: 'tester', label: 'Tester' },
  { value: 'researcher', label: 'Researcher' },
  { value: 'skeptic', label: 'Skeptic' },
  { value: 'synthesizer', label: 'Synthesizer' },
];

export const CHAIN_STRATEGIES: { value: ChainStrategy; label: string; description: string }[] = [
  { value: 'parallel_then_synthesize', label: 'Parallel → Synthesize', description: 'All workers run together; leader synthesizes the outputs.' },
  { value: 'sequential_gate', label: 'Sequential Gate', description: 'Workers run in order, each gating on the previous result.' },
  { value: 'parallel_vote', label: 'Parallel Vote', description: 'Workers run together; majority vote determines the output.' },
  { value: 'dev_pipeline', label: 'Dev Pipeline', description: 'Fixed Implement → Review → Test → Synthesize pipeline.' },
];

export function roleLabel(role: WorkerRole): string {
  const found = BUILT_IN_ROLES.find((r) => r.value === role);
  return found?.label ?? role;
}

export function emptyWorker(slot: number): SwarmWorkerConfig {
  return {
    slot,
    role: 'implementer',
    model: null,
    adapter: null,
    gpu_layers: -1,
    system_prompt: null,
    allowed_tools: null,
    priority: 1,
  };
}

export function emptyTemplate(): SwarmConfig {
  return {
    id: '',
    name: '',
    description: '',
    workers: [emptyWorker(0), emptyWorker(1)],
    chain_strategy: 'parallel_then_synthesize',
    synthesis_model: null,
    enabled: true,
    created_at: '',
    last_used_at: null,
  };
}

export const swarmTemplates = writable<SwarmConfig[]>([]);
export const swarmTemplatesError = writable<string>('');
export const activatingTemplateId = writable<string | null>(null);

export async function loadSwarmTemplates(): Promise<void> {
  try {
    const result = await invoke<SwarmConfig[] | null>('list_swarm_configs');
    swarmTemplates.set(Array.isArray(result) ? result : []);
    swarmTemplatesError.set('');
  } catch (e) {
    console.error('[swarmTemplates] load error:', e);
    swarmTemplates.set([]);
    swarmTemplatesError.set(String(e));
  }
}

export async function createSwarmTemplate(cfg: SwarmConfig): Promise<SwarmConfig | null> {
  try {
    const created = await invoke<SwarmConfig>('create_swarm_config', { cfg });
    await loadSwarmTemplates();
    swarmTemplatesError.set('');
    return created;
  } catch (e) {
    swarmTemplatesError.set(String(e));
    return null;
  }
}

export async function updateSwarmTemplate(cfg: SwarmConfig): Promise<boolean> {
  try {
    await invoke('update_swarm_config', { cfg });
    await loadSwarmTemplates();
    swarmTemplatesError.set('');
    return true;
  } catch (e) {
    swarmTemplatesError.set(String(e));
    return false;
  }
}

export async function deleteSwarmTemplate(id: string): Promise<boolean> {
  try {
    await invoke('delete_swarm_config', { id });
    await loadSwarmTemplates();
    swarmTemplatesError.set('');
    return true;
  } catch (e) {
    swarmTemplatesError.set(String(e));
    return false;
  }
}

/**
 * Activating a template replaces the live agent topology
 * (agent_store/AgentConfig+Persona rows) that the real Custom Swarm feature
 * runs — see `swarm_config.rs::apply_swarm_config_to_agents`. This is a
 * destructive replace of whatever agents are currently configured, so
 * callers should confirm with the user first.
 */
export async function activateSwarmTemplate(id: string): Promise<boolean> {
  activatingTemplateId.set(id);
  try {
    await invoke('activate_swarm', { id });
    await loadSwarmTemplates();
    swarmTemplatesError.set('');
    return true;
  } catch (e) {
    swarmTemplatesError.set(String(e));
    return false;
  } finally {
    activatingTemplateId.set(null);
  }
}
