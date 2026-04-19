# Bonsai Production Cluster Orchestrator

## Goals

1. Scale to large heterogeneous fleets (desktop, server, laptop, tablet, mobile).
2. Distribute workloads by real-time resource headroom, policy constraints, and workload intent.
3. Keep onboarding and operations simple enough for non-expert users.
4. Preserve local-first behavior while enabling optional shared execution pools.
5. Provide deterministic, auditable dispatch decisions.

## Current Implementation Added

The backend now includes a first functional scheduling surface in:
- `src-tauri/src/cluster_orchestrator.rs`

And commands exposed through Tauri in:
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`

Frontend API wrappers are available in:
- `src/lib/stores/cluster.ts`

### Implemented Capabilities (Phase 1)

1. Node registration and update (`cluster_upsert_node`).
2. Node removal (`cluster_remove_node`).
3. Runtime metrics update (`cluster_update_node_metrics`).
4. Policy set/get (`cluster_set_policy`, `cluster_get_policy`).
5. Workload planning (`cluster_plan_workload`) with ranked candidates and rejection rationale.
6. Node listing (`cluster_list_nodes`).

## System Architecture (Target Production Shape)

### Components

1. **Cluster Control Plane (this app instance)**
- Holds cluster state and scheduling policy.
- Computes dispatch plans.
- Emits placement rationale and health events.

2. **Node Agent (desktop/mobile runtime)**
- Reports heartbeat and resource telemetry.
- Declares capabilities and resource-share limits.
- Executes assigned jobs and reports status.

3. **Workload Gateway**
- Accepts workload submissions and constraints.
- Maps tasks to scheduler queues.

4. **Execution Fabric**
- Model execution, tool calls, and sidecar jobs.
- Streaming output channels back to origin session.

5. **Observability Plane**
- Event log, metrics, and dispatch timeline.
- Health dashboards and operator alerts.

### Data Model

1. **Node model**
- Identity: `node_id`, `display_name`, type, labels.
- Quota/limits: CPU/RAM/GPU shares, battery floor, concurrency cap.
- Live telemetry: CPU utilization, free RAM, GPU availability, battery, latency.

2. **Workload model**
- Resource requests: CPU, RAM, GPU.
- Constraints: mobile allowed, desktop allowed, required labels.
- Behavior hints: latency-sensitive, throughput-heavy.

3. **Policy model**
- Scheduling strategy (`balanced`, `throughput`, `lowest_latency`, `energy_saver`).
- Max fan-out nodes per workload.
- Overcommit ratio.
- Label affinity strictness.

## Scheduling Approach

### Candidate Filtering

A node is ineligible if any hard constraint fails:
- Offline.
- At max concurrency.
- Disallowed by platform constraints.
- Missing required labels when strict affinity enabled.
- Battery below per-node minimum.
- Insufficient CPU, RAM, or GPU budgets.

### Scoring

Eligible nodes receive a weighted score from:
- CPU headroom.
- RAM headroom.
- GPU headroom.
- Network/dispatch latency.
- Optional battery bonus in energy-saver mode.

Latency-sensitive workloads get extra latency weighting.

### Selection

- Rank descending by score.
- Select top N (`max_nodes_per_workload`).
- Return both selected and rejected candidates with rationale.

This creates deterministic, inspectable decisions suitable for operator trust.

## Reliability and Failover Model

### Heartbeat/TTL

1. Add per-node heartbeat timestamps and configured TTL.
2. Mark stale nodes unavailable automatically.
3. Trigger replan when selected nodes become stale mid-run.

### Dispatch Lifecycle

1. `planned` -> `dispatched` -> `running` -> `succeeded/failed/cancelled`.
2. Keep assignment ledger for reconciliation after restart.

### Rebalance

1. Periodic rebalance pass for long-running tasks.
2. Migrate only workloads that are migration-safe.
3. Respect anti-thrashing windows and cooldowns.

## Security and Trust Model

1. Mutual auth between coordinator and nodes (device certificates or signed tokens).
2. Scoped capability tokens per workload class.
3. Encrypted transport for control and data channels.
4. Resource-share defaults deny heavy background jobs on battery-constrained nodes.
5. Signed workload metadata for tamper-evident dispatch chain.

## UX and Setup Strategy

### Fast Path (Simple)

1. User enables "Cluster Mode".
2. LAN-discovered devices appear as pending nodes.
3. User approves and picks preset policy:
- Balanced
- Performance
- Battery Saver
4. Scheduler is active immediately.

### Advanced Path

1. Per-node resource share caps.
2. Label-based routing (for example: `gpu`, `low-latency`, `trusted`).
3. Affinity/anti-affinity rules.
4. Workload class quotas and reserve pools.

## Operational Telemetry

Expose metrics and events:

1. Queue depth by workload class.
2. Placement latency and dispatch success rate.
3. Node saturation and starvation indicators.
4. Rejection reasons cardinality (CPU, RAM, battery, labels).
5. Mean time to recovery after node loss.

## Rollout Plan

### Phase 1 (Completed in this change)

1. In-memory node registry.
2. Policy control and scoring planner.
3. Tauri command surface and frontend wrappers.

### Phase 2

1. Persistent cluster state (WAL-backed).
2. Node heartbeat protocol.
3. Dispatch lifecycle state machine.

### Phase 3

1. Real execution bindings to swarm/tool runners.
2. Retry/fallback and checkpoint-aware rescheduling.
3. Operator dashboard views and alarms.

### Phase 4

1. Multi-coordinator HA mode.
2. Cross-network federation and trust domains.
3. Cost-aware optimization and predictive pre-placement.

## Validation Plan

1. Unit tests for scoring and hard-filter rules.
2. Property tests ensuring no placement violates hard constraints.
3. Simulations for burst load, skewed telemetry, and churn.
4. Android + desktop mixed-fleet smoke tests using real APK and strict launcher flow.

## Notes on Your Existing Requests

1. Mobile clipping/safe-area and calibration support has been implemented in Svelte layout/settings changes.
2. Model switching timeout behavior has been hardened in both frontend and Rust orchestrator paths.
3. Strict launcher `pidof` race has retry-window hardening in `android-usb-regression.mjs`.
4. Cluster orchestration has now moved from concept into code-level commandable primitives, ready for UI integration and execution-fabric binding.
