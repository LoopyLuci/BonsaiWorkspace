import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export type ClusterDeviceType = 'desktop' | 'laptop' | 'mobile' | 'tablet' | 'server' | 'unknown';
export type SchedulingStrategy = 'balanced' | 'throughput' | 'lowest_latency' | 'energy_saver';

export interface NodeResourceShare {
  cpu_share_pct: number;
  ram_share_mb: number;
  gpu_share_pct: number;
  max_concurrency: number;
  min_battery_pct: number;
  allow_background_heavy_jobs: boolean;
}

export interface NodeRuntimeMetrics {
  cpu_utilization_pct: number;
  free_ram_mb: number;
  available_gpu_pct: number;
  battery_pct: number | null;
  latency_ms: number;
}

export interface ClusterNode {
  node_id: string;
  display_name: string;
  device_type: ClusterDeviceType;
  labels: string[];
  share: NodeResourceShare;
  metrics: NodeRuntimeMetrics;
  is_online: boolean;
  active_workloads: number;
  last_seen_ms: number;
}

export interface ClusterPolicy {
  strategy: SchedulingStrategy;
  max_nodes_per_workload: number;
  overcommit_ratio: number;
  require_label_affinity: boolean;
}

export interface ClusterWorkload {
  workload_id: string;
  cpu_cost_pct: number;
  ram_required_mb: number;
  gpu_cost_pct: number;
  latency_sensitive: boolean;
  required_labels: string[];
  allow_mobile: boolean;
  allow_desktop: boolean;
}

export interface ClusterDispatchPlan {
  workload_id: string;
  selected: Array<{ node_id: string; score: number; rationale: string[] }>;
  rejected: Array<{ node_id: string; score: number; rationale: string[] }>;
  strategy: SchedulingStrategy;
}

export const clusterNodes = writable<ClusterNode[]>([]);
export const clusterPolicy = writable<ClusterPolicy | null>(null);
export const clusterLastPlan = writable<ClusterDispatchPlan | null>(null);

export async function refreshClusterNodes() {
  const nodes = await invoke<ClusterNode[]>('cluster_list_nodes');
  clusterNodes.set(nodes);
  return nodes;
}

export async function refreshClusterPolicy() {
  const policy = await invoke<ClusterPolicy>('cluster_get_policy');
  clusterPolicy.set(policy);
  return policy;
}

export async function upsertClusterNode(node: ClusterNode) {
  await invoke('cluster_upsert_node', { node });
  return refreshClusterNodes();
}

export async function removeClusterNode(nodeId: string) {
  await invoke('cluster_remove_node', { nodeId });
  return refreshClusterNodes();
}

export async function updateClusterMetrics(nodeId: string, metrics: NodeRuntimeMetrics) {
  await invoke('cluster_update_node_metrics', { nodeId, metrics });
  return refreshClusterNodes();
}

export async function setClusterPolicy(policy: ClusterPolicy) {
  const updated = await invoke<ClusterPolicy>('cluster_set_policy', { policy });
  clusterPolicy.set(updated);
  return updated;
}

export async function planClusterWorkload(workload: ClusterWorkload) {
  const plan = await invoke<ClusterDispatchPlan>('cluster_plan_workload', { workload });
  clusterLastPlan.set(plan);
  return plan;
}
