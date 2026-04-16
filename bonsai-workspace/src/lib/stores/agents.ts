import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface Persona {
  id: string;
  name: string;
  system_prompt: string;
  model_id: string | null;
  color: string;
  icon_emoji: string;
  created_at: number;
  updated_at: number;
}

export interface AgentConfig {
  id: string;
  slot_index: number;
  label: string;
  persona_id: string | null;
  model_id: string | null;
  color: string;
  icon_emoji: string;
  enabled: boolean;
  max_tokens: number;
  created_at: number;
  updated_at: number;
}

export interface ResolvedAgent {
  config: AgentConfig;
  persona: Persona | null;
  system_prompt: string;
  effective_model_id: string | null;
  ram_required_mb: number;
}

export interface AgentResourceCost {
  agent_id: string;
  slot_index: number;
  model_id: string | null;
  ram_required_mb: number;
}

export interface SwarmResourceEstimate {
  total_ram_required_mb: number;
  free_ram_mb: number;
  fits: boolean;
  per_agent: AgentResourceCost[];
}

export interface SwarmRuntimeSettings {
  leader_plan_required: boolean;
  max_worker_subtasks: number;
  allow_worker_tools: boolean;
  enable_worker_cross_review: boolean;
  parallel_workers: boolean;
  include_worker_summaries: boolean;
  synthesis_style: 'balanced' | 'concise' | 'detailed' | 'strict';
  retry_failed_workers: boolean;
  worker_timeout_ms: number;
  stream_worker_tokens: boolean;
  emit_debug_events: boolean;
  max_worker_response_chars: number;
  include_original_prompt_in_worker_context: boolean;
  allow_leader_as_worker: boolean;
}

// ── Stores ────────────────────────────────────────────────────────────────────

export const agentConfigs      = writable<ResolvedAgent[]>([]);
export const personas          = writable<Persona[]>([]);
export const resourceEstimate  = writable<SwarmResourceEstimate | null>(null);
export const activeSwarmRunId  = writable<string | null>(null);
export const agentStreams       = writable<Map<string, string>>(new Map());
export const swarmRuntimeSettings = writable<SwarmRuntimeSettings>({
  leader_plan_required: false,
  max_worker_subtasks: 8,
  allow_worker_tools: true,
  enable_worker_cross_review: false,
  parallel_workers: true,
  include_worker_summaries: true,
  synthesis_style: 'balanced',
  retry_failed_workers: true,
  worker_timeout_ms: 120000,
  stream_worker_tokens: true,
  emit_debug_events: true,
  max_worker_response_chars: 5000,
  include_original_prompt_in_worker_context: true,
  allow_leader_as_worker: true,
});

const SWARM_SETTINGS_KEY = 'bonsai-swarm-runtime-settings-v1';

export const swarmEnabled = derived(agentConfigs, ($configs) =>
  (Array.isArray($configs) ? $configs : []).filter(a => a?.config?.enabled).length >= 2
);

// ── Actions ───────────────────────────────────────────────────────────────────

export async function loadAgentConfigs(): Promise<void> {
  try {
    const result = await invoke<ResolvedAgent[] | null>('list_agent_configs');
    agentConfigs.set(Array.isArray(result) ? result : []);
  } catch (e) {
    console.error('[agents] loadAgentConfigs error:', e);
    agentConfigs.set([]);
  }
}

export async function loadPersonas(): Promise<void> {
  try {
    const result = await invoke<Persona[] | null>('list_personas');
    personas.set(Array.isArray(result) ? result : []);
  } catch (e) {
    console.error('[agents] loadPersonas error:', e);
    personas.set([]);
  }
}

export async function refreshResourceEstimate(): Promise<void> {
  try {
    const result = await invoke<SwarmResourceEstimate | null>('estimate_swarm_resources');
    resourceEstimate.set(result ?? null);
  } catch (e) {
    console.error('[agents] refreshResourceEstimate error:', e);
    resourceEstimate.set(null);
  }
}

export async function upsertAgentConfig(config: AgentConfig): Promise<void> {
  await invoke('upsert_agent_config', { config });
  await loadAgentConfigs();
  await refreshResourceEstimate();
}

export async function deleteAgentConfig(id: string): Promise<void> {
  await invoke('delete_agent_config', { id });
  await loadAgentConfigs();
  await refreshResourceEstimate();
}

export async function upsertPersona(persona: Persona): Promise<void> {
  await invoke('upsert_persona', { persona });
  await loadPersonas();
}

export async function deletePersona(id: string): Promise<void> {
  await invoke('delete_persona', { id });
  await loadPersonas();
}

export function loadSwarmRuntimeSettings(): void {
  if (typeof window === 'undefined') return;
  try {
    const raw = window.localStorage.getItem(SWARM_SETTINGS_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Partial<SwarmRuntimeSettings>;
    swarmRuntimeSettings.update((current) => ({ ...current, ...parsed }));
  } catch (e) {
    console.warn('[agents] loadSwarmRuntimeSettings error:', e);
  }
}

export function saveSwarmRuntimeSettings(settings: SwarmRuntimeSettings): void {
  if (typeof window === 'undefined') return;
  window.localStorage.setItem(SWARM_SETTINGS_KEY, JSON.stringify(settings));
}

export function patchSwarmRuntimeSettings(patch: Partial<SwarmRuntimeSettings>): void {
  swarmRuntimeSettings.update((current) => {
    const next = { ...current, ...patch };
    saveSwarmRuntimeSettings(next);
    return next;
  });
}

export function resetSwarmRuntimeSettings(): void {
  const defaults: SwarmRuntimeSettings = {
    leader_plan_required: false,
    max_worker_subtasks: 8,
    allow_worker_tools: true,
    enable_worker_cross_review: false,
    parallel_workers: true,
    include_worker_summaries: true,
    synthesis_style: 'balanced',
    retry_failed_workers: true,
    worker_timeout_ms: 120000,
    stream_worker_tokens: true,
    emit_debug_events: true,
    max_worker_response_chars: 5000,
    include_original_prompt_in_worker_context: true,
    allow_leader_as_worker: true,
  };
  swarmRuntimeSettings.set(defaults);
  saveSwarmRuntimeSettings(defaults);
}
