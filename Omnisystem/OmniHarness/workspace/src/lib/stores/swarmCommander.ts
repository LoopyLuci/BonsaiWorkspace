import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust types in crates/swarm/src/{orchestrator,hierarchy,ledger,dag}.rs.
// Standard serde "externally tagged" enum representation: fieldless variants
// serialize as bare snake_case strings; variants with named fields serialize
// as `{ variant_name: { ...fields } }`.

export type SwarmStatus =
  | 'initialising' | 'planning' | 'running' | 'paused' | 'completing' | 'completed' | 'cancelled'
  | { failed: { reason: string } };

export type NodeStatus =
  | 'initialising' | 'idle' | 'working' | 'paused' | 'migrating' | 'stopped'
  | { error: { reason: string } };

export type TaskStatus =
  | 'pending' | 'queued' | 'completed' | 'cancelled'
  | { running: { assigned_to: string } }
  | { failed: { reason: string } };

export interface HierarchyStats {
  total: number;
  working: number;
  idle: number;
  error: number;
  total_credits: number;
  avg_load: number;
}

export interface SwarmSnapshot {
  id: string;
  name: string;
  goal: string;
  status: SwarmStatus;
  progress: number;
  eta_minutes: number;
  agent_stats: HierarchyStats;
  task_total: number;
  task_completed: number;
  task_failed: number;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
  result_summary: string | null;
  ledger_entries: number;
}

export interface HierarchyNode {
  id: string;
  swarm_id: string;
  parent_id: string | null;
  role: string;
  display_name: string;
  domain: string;
  status: NodeStatus;
  current_task: string | null;
  progress: number;
  cpu_load: number;
  ram_mb: number;
  credits_used: number;
  tasks_completed: number;
  tasks_failed: number;
  is_remote: boolean;
  device_label: string | null;
  spawned_at: string;
  last_heartbeat: string;
}

export interface TaskNode {
  id: string;
  swarm_id: string;
  description: string;
  context: string;
  status: TaskStatus;
  depends_on: string[];
  required_capabilities: { kind: string; name: string }[];
  estimated_minutes: number;
  result: string | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
}

export type LedgerEventKind =
  | 'swarm_created' | 'swarm_completed' | 'swarm_failed' | 'plan_verified'
  | { task_assigned: { agent_id: string } }
  | { task_started: { agent_id: string } }
  | { task_completed: { agent_id: string } }
  | { task_failed: { agent_id: string; reason: string } }
  | { task_retried: { agent_id: string; attempt: number } }
  | { agent_spawned: { role: string } }
  | { agent_stopped: { reason: string } }
  | { tool_invoked: { tool: string; agent_id: string } }
  | { tool_result: { tool: string; agent_id: string; success: boolean } }
  | { plan_created: { task_count: number } }
  | { plan_rejected: { reason: string } }
  | { assistant_suggestion: { agent_id: string; suggestion: string } }
  | { capability_negotiation: { requester: string; provider: string; accepted: boolean } }
  | { agent_migrated: { from_device: string; to_device: string } }
  | { user_approval: { approved: boolean } }
  | { custom: { tag: string; payload: unknown } };

export interface LedgerEntry {
  seq: number;
  swarm_id: string;
  task_id: string | null;
  event: LedgerEventKind;
  timestamp: string;
  hash: string;
  prev_hash: string;
}

/** Render any of the tagged-union statuses/events as a short human string. */
export function describeStatus(status: SwarmStatus | NodeStatus | TaskStatus): string {
  if (typeof status === 'string') return status.replace(/_/g, ' ');
  const key = Object.keys(status)[0];
  const val = (status as Record<string, { reason?: string }>)[key];
  return `${key.replace(/_/g, ' ')}${val?.reason ? `: ${val.reason}` : ''}`;
}

export function describeLedgerEvent(event: LedgerEventKind): string {
  if (typeof event === 'string') return event.replace(/_/g, ' ');
  const key = Object.keys(event)[0];
  const val = (event as Record<string, Record<string, unknown>>)[key];
  const detail = Object.entries(val ?? {}).map(([k, v]) => `${k}=${String(v)}`).join(', ');
  return `${key.replace(/_/g, ' ')}${detail ? ` (${detail})` : ''}`;
}

export function statusIsTerminal(status: SwarmStatus): boolean {
  if (typeof status === 'string') return status === 'completed' || status === 'cancelled';
  return 'failed' in status;
}

export const swarms = writable<SwarmSnapshot[]>([]);
export const swarmsError = writable<string>('');
export const selectedSwarmId = writable<string | null>(null);
export const selectedHierarchy = writable<HierarchyNode[]>([]);
export const selectedLedger = writable<LedgerEntry[]>([]);
export const selectedTasks = writable<TaskNode[]>([]);

export async function refreshSwarmList(): Promise<void> {
  try {
    const result = await invoke<SwarmSnapshot[] | null>('list_swarms');
    swarms.set(Array.isArray(result) ? result : []);
    swarmsError.set('');
  } catch (e) {
    console.error('[swarmCommander] list_swarms error:', e);
    swarms.set([]);
    swarmsError.set(String(e));
  }
}

export async function selectSwarm(swarmId: string): Promise<void> {
  selectedSwarmId.set(swarmId);
  await refreshSwarmDetail(swarmId);
}

export async function refreshSwarmDetail(swarmId: string): Promise<void> {
  try {
    const [hierarchy, ledger, dag] = await Promise.all([
      invoke<HierarchyNode[]>('get_swarm_hierarchy', { swarmId }),
      invoke<LedgerEntry[]>('get_swarm_ledger', { swarmId, lastN: 200 }),
      invoke<TaskNode[]>('get_swarm_dag', { swarmId }),
    ]);
    selectedHierarchy.set(hierarchy ?? []);
    selectedLedger.set((ledger ?? []).slice().reverse());
    selectedTasks.set(dag ?? []);
    swarmsError.set('');
  } catch (e) {
    console.error('[swarmCommander] detail fetch error:', e);
    swarmsError.set(String(e));
  }
}

/**
 * Cancels the real underlying swarm run (not just a cosmetic status flip) —
 * see `swarm_commands::swarm_cancel`'s doc comment for why both the bridged
 * registry command and the real run's cancel flags are needed.
 */
export async function cancelSwarm(swarmId: string): Promise<boolean> {
  try {
    await invoke('swarm_cancel', { swarmId });
    await refreshSwarmList();
    return true;
  } catch (e) {
    swarmsError.set(String(e));
    return false;
  }
}
