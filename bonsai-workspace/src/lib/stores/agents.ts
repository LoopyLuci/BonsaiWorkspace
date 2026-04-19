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
  shared_ram_required_mb?: number;
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
  chain_strategy: 'sequential_gate' | 'parallel_vote' | 'parallel_then_delegate' | 'sequential_then_delegate';
  stop_on_first_satisfactory: boolean;
  satisfaction_threshold: number;
  preferred_primary_slot: number;
  force_all_workers_before_decision: boolean;
  heavy_work_delegate_mode: 'none' | 'selected' | 'configured';
  configured_heavy_worker_slot: number;
  heavy_work_delegate_auto_fallback: boolean;
  auto_repair_delegate_routing: boolean;
  agent_chain_policies: AgentChainPolicy[];
}

export interface AgentChainPolicy {
  slot_index: number;
  execution_tier: number;
  always_run: boolean;
  can_be_early_exit_gate: boolean;
  early_exit_confidence_threshold: number;
  response_weight: number;
  can_review_from_slots: number[];
  can_delegate_to_slots: number[];
  allow_heavy_work: boolean;
}

// ── Stores ────────────────────────────────────────────────────────────────────

export const agentConfigs      = writable<ResolvedAgent[]>([]);
export const personas          = writable<Persona[]>([]);
export const resourceEstimate  = writable<SwarmResourceEstimate | null>(null);
export const activeSwarmRunId  = writable<string | null>(null);
export const agentStreams       = writable<Map<string, string>>(new Map());
export const chatStreamEnabledByAgent = writable<Record<string, boolean>>({});
export const swarmRuntimeSettings = writable<SwarmRuntimeSettings>({
  leader_plan_required: true,
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
  chain_strategy: 'parallel_then_delegate',
  stop_on_first_satisfactory: false,
  satisfaction_threshold: 78,
  preferred_primary_slot: 1,
  force_all_workers_before_decision: true,
  heavy_work_delegate_mode: 'selected',
  configured_heavy_worker_slot: 2,
  heavy_work_delegate_auto_fallback: false,
  auto_repair_delegate_routing: false,
  agent_chain_policies: [],
});

const SWARM_SETTINGS_KEY = 'bonsai-swarm-runtime-settings-v1';
const CHAT_STREAM_PREFS_KEY = 'bonsai-chat-stream-per-agent-v1';

export const swarmEnabled = derived(agentConfigs, ($configs) =>
  (Array.isArray($configs) ? $configs : []).filter(a => a?.config?.enabled).length >= 2
);

function readChatStreamingPrefs(): Record<string, boolean> {
  if (typeof window === 'undefined') return {};
  try {
    const raw = window.localStorage.getItem(CHAT_STREAM_PREFS_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, boolean>;
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function saveChatStreamingPrefs(prefs: Record<string, boolean>): void {
  if (typeof window === 'undefined') return;
  window.localStorage.setItem(CHAT_STREAM_PREFS_KEY, JSON.stringify(prefs));
}

function syncChatStreamingPrefs(agents: ResolvedAgent[]): void {
  const persisted = readChatStreamingPrefs();
  const next: Record<string, boolean> = {};
  for (const resolved of agents) {
    const agentId = resolved.config.id;
    const persistedValue = persisted[agentId];
    next[agentId] = persistedValue === false ? false : true;
  }
  chatStreamEnabledByAgent.set(next);
  saveChatStreamingPrefs(next);
}

export function setAgentChatStreaming(agentId: string, enabled: boolean): void {
  chatStreamEnabledByAgent.update((current) => {
    const next = { ...current, [agentId]: enabled };
    saveChatStreamingPrefs(next);
    return next;
  });
}

export function setAllAgentChatStreaming(agents: ResolvedAgent[], enabled: boolean): void {
  const next: Record<string, boolean> = {};
  for (const resolved of agents) {
    next[resolved.config.id] = enabled;
  }
  chatStreamEnabledByAgent.set(next);
  saveChatStreamingPrefs(next);
}

// ── Actions ───────────────────────────────────────────────────────────────────

export async function loadAgentConfigs(): Promise<void> {
  try {
    const result = await invoke<ResolvedAgent[] | null>('list_agent_configs');
    const list = Array.isArray(result) ? result : [];
    agentConfigs.set(list);
    syncChatStreamingPrefs(list);
  } catch (e) {
    console.error('[agents] loadAgentConfigs error:', e);
    agentConfigs.set([]);
    syncChatStreamingPrefs([]);
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
    swarmRuntimeSettings.update((current) => ({
      ...current,
      ...parsed,
      leader_plan_required: true,
      parallel_workers: true,
      chain_strategy: 'parallel_then_delegate',
      stop_on_first_satisfactory: false,
      force_all_workers_before_decision: true,
    }));
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

export function normalizeChainPolicies(settings: SwarmRuntimeSettings, agents: ResolvedAgent[]): SwarmRuntimeSettings {
  const existing = new Map((settings.agent_chain_policies ?? []).map((p) => [p.slot_index, p]));
  const normalized: AgentChainPolicy[] = agents
    .map((resolved) => resolved.config.slot_index)
    .sort((a, b) => a - b)
    .map((slot) => {
      const prev = existing.get(slot);
      return {
        slot_index: slot,
        execution_tier: prev?.execution_tier ?? slot,
        always_run: prev?.always_run ?? (slot === 0),
        can_be_early_exit_gate: prev?.can_be_early_exit_gate ?? (slot === 1),
        early_exit_confidence_threshold: prev?.early_exit_confidence_threshold ?? 78,
        response_weight: prev?.response_weight ?? (slot === 0 ? 1 : 2),
        can_review_from_slots: prev?.can_review_from_slots ?? (slot <= 1 ? [0] : [0, 1]),
        can_delegate_to_slots: prev?.can_delegate_to_slots ?? agents
          .map((r) => r.config.slot_index)
          .filter((s) => s !== slot),
        allow_heavy_work: prev?.allow_heavy_work ?? (slot !== 0),
      };
    });

  const currentJson = JSON.stringify(settings.agent_chain_policies ?? []);
  const nextJson = JSON.stringify(normalized);
  if (currentJson === nextJson) {
    return settings;
  }
  return { ...settings, agent_chain_policies: normalized };
}

export function ensureChainPoliciesForAgents(agents: ResolvedAgent[]): void {
  swarmRuntimeSettings.update((current) => {
    const next = normalizeChainPolicies(current, agents);
    if (next === current) return current;
    saveSwarmRuntimeSettings(next);
    return next;
  });
}

export function patchAgentChainPolicy(slotIndex: number, patch: Partial<AgentChainPolicy>): void {
  swarmRuntimeSettings.update((current) => {
    const list = [...(current.agent_chain_policies ?? [])];
    const idx = list.findIndex((p) => p.slot_index === slotIndex);
    if (idx === -1) return current;
    list[idx] = { ...list[idx], ...patch };
    const next = { ...current, agent_chain_policies: list };
    saveSwarmRuntimeSettings(next);
    return next;
  });
}

export function resetSwarmRuntimeSettings(): void {
  const defaults: SwarmRuntimeSettings = {
    leader_plan_required: true,
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
    chain_strategy: 'parallel_then_delegate',
    stop_on_first_satisfactory: false,
    satisfaction_threshold: 78,
    preferred_primary_slot: 1,
    force_all_workers_before_decision: true,
    heavy_work_delegate_mode: 'selected',
    configured_heavy_worker_slot: 2,
    heavy_work_delegate_auto_fallback: false,
    auto_repair_delegate_routing: false,
    agent_chain_policies: [],
  };
  swarmRuntimeSettings.set(defaults);
  saveSwarmRuntimeSettings(defaults);
}
