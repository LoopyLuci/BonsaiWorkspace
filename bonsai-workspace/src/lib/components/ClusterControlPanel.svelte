<script lang="ts">
  import { onMount } from 'svelte';
  import {
    clusterLastPlan,
    clusterNodes,
    clusterPolicy,
    refreshClusterNodes,
    refreshClusterPolicy,
    removeClusterNode,
    setClusterPolicy,
    upsertClusterNode,
    updateClusterMetrics,
    planClusterWorkload,
    type ClusterDeviceType,
    type ClusterNode,
    type ClusterPolicy,
    type ClusterWorkload,
    type NodeRuntimeMetrics,
  } from '$lib/stores/cluster';

  let busy = false;
  let errorMsg = '';
  let infoMsg = '';

  let nodeId = '';
  let displayName = '';
  let deviceType: ClusterDeviceType = 'desktop';
  let labelsCsv = '';
  let maxConcurrency = 2;
  let cpuSharePct = 50;
  let ramShareMb = 4096;
  let gpuSharePct = 0;
  let minBatteryPct = 20;
  let allowBackgroundHeavyJobs = true;

  let policy: ClusterPolicy = {
    strategy: 'balanced',
    max_nodes_per_workload: 3,
    overcommit_ratio: 1,
    require_label_affinity: false,
  };

  let workloadId = `workload-${Date.now()}`;
  let workloadCpuPct = 20;
  let workloadRamMb = 2048;
  let workloadGpuPct = 0;
  let workloadLatencySensitive = false;
  let workloadLabelsCsv = '';
  let workloadAllowMobile = true;
  let workloadAllowDesktop = true;

  function labelsFromCsv(csv: string): string[] {
    return csv
      .split(',')
      .map((x) => x.trim())
      .filter(Boolean);
  }

  async function refreshAll() {
    busy = true;
    errorMsg = '';
    infoMsg = '';
    try {
      await Promise.all([refreshClusterNodes(), refreshClusterPolicy()]);
      const current = $clusterPolicy;
      if (current) {
        policy = {
          strategy: current.strategy,
          max_nodes_per_workload: current.max_nodes_per_workload,
          overcommit_ratio: current.overcommit_ratio,
          require_label_affinity: current.require_label_affinity,
        };
      }
      infoMsg = 'Cluster state refreshed.';
    } catch (error) {
      errorMsg = `Refresh failed: ${String(error)}`;
    } finally {
      busy = false;
    }
  }

  async function savePolicy() {
    busy = true;
    errorMsg = '';
    infoMsg = '';
    try {
      await setClusterPolicy({
        strategy: policy.strategy,
        max_nodes_per_workload: Math.max(1, Number(policy.max_nodes_per_workload || 1)),
        overcommit_ratio: Math.max(0.1, Number(policy.overcommit_ratio || 1)),
        require_label_affinity: !!policy.require_label_affinity,
      });
      infoMsg = 'Policy updated.';
    } catch (error) {
      errorMsg = `Policy update failed: ${String(error)}`;
    } finally {
      busy = false;
    }
  }

  async function upsertNode() {
    busy = true;
    errorMsg = '';
    infoMsg = '';
    try {
      const id = nodeId.trim();
      if (!id) throw new Error('Node ID is required.');

      const node: ClusterNode = {
        node_id: id,
        display_name: displayName.trim() || id,
        device_type: deviceType,
        labels: labelsFromCsv(labelsCsv),
        share: {
          cpu_share_pct: Number(cpuSharePct || 0),
          ram_share_mb: Number(ramShareMb || 0),
          gpu_share_pct: Number(gpuSharePct || 0),
          max_concurrency: Number(maxConcurrency || 1),
          min_battery_pct: Number(minBatteryPct || 0),
          allow_background_heavy_jobs: !!allowBackgroundHeavyJobs,
        },
        metrics: {
          cpu_utilization_pct: 0,
          free_ram_mb: Number(ramShareMb || 0),
          available_gpu_pct: 100,
          battery_pct: null,
          latency_ms: 50,
        },
        is_online: true,
        active_workloads: 0,
        last_seen_ms: Date.now(),
      };

      await upsertClusterNode(node);
      infoMsg = `Node ${id} saved.`;
    } catch (error) {
      errorMsg = `Node upsert failed: ${String(error)}`;
    } finally {
      busy = false;
    }
  }

  async function removeNode(nodeIdValue: string) {
    busy = true;
    errorMsg = '';
    infoMsg = '';
    try {
      await removeClusterNode(nodeIdValue);
      infoMsg = `Node ${nodeIdValue} removed.`;
    } catch (error) {
      errorMsg = `Node removal failed: ${String(error)}`;
    } finally {
      busy = false;
    }
  }

  async function pushMetrics(node: ClusterNode) {
    busy = true;
    errorMsg = '';
    infoMsg = '';
    try {
      const metrics: NodeRuntimeMetrics = {
        cpu_utilization_pct: Number(node.metrics.cpu_utilization_pct),
        free_ram_mb: Number(node.metrics.free_ram_mb),
        available_gpu_pct: Number(node.metrics.available_gpu_pct),
        battery_pct: node.metrics.battery_pct,
        latency_ms: Number(node.metrics.latency_ms),
      };
      await updateClusterMetrics(node.node_id, metrics);
      infoMsg = `Metrics pushed for ${node.node_id}.`;
    } catch (error) {
      errorMsg = `Metrics update failed: ${String(error)}`;
    } finally {
      busy = false;
    }
  }

  async function runPlanner() {
    busy = true;
    errorMsg = '';
    infoMsg = '';
    try {
      const workload: ClusterWorkload = {
        workload_id: workloadId.trim() || `workload-${Date.now()}`,
        cpu_cost_pct: Number(workloadCpuPct || 0),
        ram_required_mb: Number(workloadRamMb || 0),
        gpu_cost_pct: Number(workloadGpuPct || 0),
        latency_sensitive: !!workloadLatencySensitive,
        required_labels: labelsFromCsv(workloadLabelsCsv),
        allow_mobile: !!workloadAllowMobile,
        allow_desktop: !!workloadAllowDesktop,
      };

      await planClusterWorkload(workload);
      infoMsg = `Planner executed for ${workload.workload_id}.`;
    } catch (error) {
      errorMsg = `Planner failed: ${String(error)}`;
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void refreshAll();
  });
</script>

<div class="cluster-panel">
  <div class="cluster-actions">
    <button type="button" class="action-btn blue" on:click={refreshAll} disabled={busy}>Refresh Cluster</button>
  </div>

  {#if infoMsg}
    <div class="cluster-msg cluster-msg--ok">{infoMsg}</div>
  {/if}
  {#if errorMsg}
    <div class="cluster-msg cluster-msg--error">{errorMsg}</div>
  {/if}

  <div class="cluster-grid">
    <div class="cluster-card">
      <h4>Policy Editor</h4>
      <label>
        Strategy
        <select bind:value={policy.strategy}>
          <option value="balanced">balanced</option>
          <option value="throughput">throughput</option>
          <option value="lowest_latency">lowest_latency</option>
          <option value="energy_saver">energy_saver</option>
        </select>
      </label>
      <label>
        Max nodes per workload
        <input type="number" min="1" max="64" bind:value={policy.max_nodes_per_workload} />
      </label>
      <label>
        Overcommit ratio
        <input type="number" step="0.1" min="0.1" max="4" bind:value={policy.overcommit_ratio} />
      </label>
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={policy.require_label_affinity} />
        Require label affinity
      </label>
      <button type="button" class="action-btn green" on:click={savePolicy} disabled={busy}>Save Policy</button>
    </div>

    <div class="cluster-card">
      <h4>Node Upsert</h4>
      <label>
        Node ID
        <input type="text" bind:value={nodeId} placeholder="desktop-01" />
      </label>
      <label>
        Display name
        <input type="text" bind:value={displayName} placeholder="Desktop 01" />
      </label>
      <label>
        Device type
        <select bind:value={deviceType}>
          <option value="desktop">desktop</option>
          <option value="laptop">laptop</option>
          <option value="mobile">mobile</option>
          <option value="tablet">tablet</option>
          <option value="server">server</option>
          <option value="unknown">unknown</option>
        </select>
      </label>
      <label>
        Labels (comma-separated)
        <input type="text" bind:value={labelsCsv} placeholder="gpu, trusted, office" />
      </label>
      <div class="inline-grid">
        <label>
          CPU share %
          <input type="number" min="0" max="100" bind:value={cpuSharePct} />
        </label>
        <label>
          RAM share MB
          <input type="number" min="256" bind:value={ramShareMb} />
        </label>
      </div>
      <div class="inline-grid">
        <label>
          GPU share %
          <input type="number" min="0" max="100" bind:value={gpuSharePct} />
        </label>
        <label>
          Max concurrency
          <input type="number" min="1" max="128" bind:value={maxConcurrency} />
        </label>
      </div>
      <label>
        Min battery %
        <input type="number" min="0" max="100" bind:value={minBatteryPct} />
      </label>
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={allowBackgroundHeavyJobs} />
        Allow background heavy jobs
      </label>
      <button type="button" class="action-btn green" on:click={upsertNode} disabled={busy}>Save Node</button>
    </div>
  </div>

  <div class="cluster-card">
    <h4>Node Table</h4>
    {#if $clusterNodes.length === 0}
      <div class="hint">No nodes registered yet.</div>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Name</th>
              <th>Type</th>
              <th>Online</th>
              <th>CPU%</th>
              <th>Free RAM MB</th>
              <th>Latency</th>
              <th>Labels</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each $clusterNodes as node (node.node_id)}
              <tr>
                <td><code>{node.node_id}</code></td>
                <td>{node.display_name}</td>
                <td>{node.device_type}</td>
                <td>{node.is_online ? 'yes' : 'no'}</td>
                <td>{Math.round(node.metrics.cpu_utilization_pct)}</td>
                <td>{node.metrics.free_ram_mb}</td>
                <td>{node.metrics.latency_ms}ms</td>
                <td>{node.labels.join(', ') || '-'}</td>
                <td>
                  <div class="table-actions">
                    <button type="button" class="btn-sm" on:click={() => pushMetrics(node)} disabled={busy}>Push Metrics</button>
                    <button type="button" class="btn-sm btn-danger" on:click={() => removeNode(node.node_id)} disabled={busy}>Remove</button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>

  <div class="cluster-card">
    <h4>Live Planner Test</h4>
    <div class="inline-grid">
      <label>
        Workload ID
        <input type="text" bind:value={workloadId} />
      </label>
      <label>
        CPU cost %
        <input type="number" min="0" max="100" bind:value={workloadCpuPct} />
      </label>
    </div>
    <div class="inline-grid">
      <label>
        RAM required MB
        <input type="number" min="128" bind:value={workloadRamMb} />
      </label>
      <label>
        GPU cost %
        <input type="number" min="0" max="100" bind:value={workloadGpuPct} />
      </label>
    </div>
    <label>
      Required labels (comma-separated)
      <input type="text" bind:value={workloadLabelsCsv} placeholder="gpu, trusted" />
    </label>
    <div class="inline-grid">
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={workloadLatencySensitive} />
        Latency sensitive
      </label>
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={workloadAllowMobile} />
        Allow mobile
      </label>
    </div>
    <label class="checkbox-row">
      <input type="checkbox" bind:checked={workloadAllowDesktop} />
      Allow desktop/server
    </label>
    <button type="button" class="action-btn blue" on:click={runPlanner} disabled={busy}>Run Planner</button>

    {#if $clusterLastPlan}
      <div class="plan-output">
        <div><strong>Strategy:</strong> {$clusterLastPlan.strategy}</div>
        <div><strong>Selected nodes:</strong> {$clusterLastPlan.selected.length}</div>
        <div><strong>Rejected nodes:</strong> {$clusterLastPlan.rejected.length}</div>
        <div class="plan-block">
          <div class="plan-title">Selected</div>
          {#if $clusterLastPlan.selected.length === 0}
            <div class="hint">No selected nodes.</div>
          {:else}
            {#each $clusterLastPlan.selected as item}
              <div class="plan-item">
                <div><code>{item.node_id}</code> score {item.score.toFixed(3)}</div>
                <div class="hint">{item.rationale.join(' | ')}</div>
              </div>
            {/each}
          {/if}
        </div>
        <div class="plan-block">
          <div class="plan-title">Rejected</div>
          {#if $clusterLastPlan.rejected.length === 0}
            <div class="hint">No rejected nodes.</div>
          {:else}
            {#each $clusterLastPlan.rejected as item}
              <div class="plan-item">
                <div><code>{item.node_id}</code> score {item.score.toFixed(3)}</div>
                <div class="hint">{item.rationale.join(' | ')}</div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .cluster-panel {
    display: grid;
    gap: 12px;
  }

  .cluster-actions {
    display: flex;
    justify-content: flex-end;
  }

  .cluster-grid {
    display: grid;
    gap: 10px;
    grid-template-columns: 1fr 1fr;
  }

  .cluster-card {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .cluster-card h4 {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-dim);
  }

  label {
    display: grid;
    gap: 5px;
    font-size: 12px;
    color: var(--text-dim);
  }

  input,
  select {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    background: var(--bg2);
    color: var(--text);
  }

  .inline-grid {
    display: grid;
    gap: 8px;
    grid-template-columns: 1fr 1fr;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .checkbox-row input[type='checkbox'] {
    width: auto;
    margin: 0;
    padding: 0;
  }

  .cluster-msg {
    border-radius: 8px;
    padding: 9px 10px;
    font-size: 12px;
  }

  .cluster-msg--ok {
    background: rgba(34, 197, 94, 0.08);
    border: 1px solid rgba(34, 197, 94, 0.3);
    color: #86efac;
  }

  .cluster-msg--error {
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fca5a5;
  }

  .table-wrap {
    overflow: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  th,
  td {
    border-bottom: 1px solid rgba(128, 128, 128, 0.18);
    padding: 6px;
    text-align: left;
    vertical-align: top;
  }

  th {
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 10px;
  }

  .table-actions {
    display: flex;
    gap: 6px;
  }

  .btn-sm {
    font-size: 11px;
    background: var(--bg2);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 8px;
    border-radius: 5px;
    cursor: pointer;
  }

  .btn-sm.btn-danger {
    color: #fecaca;
    border-color: rgba(239, 68, 68, 0.4);
  }

  .plan-output {
    margin-top: 8px;
    display: grid;
    gap: 8px;
    font-size: 12px;
  }

  .plan-block {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg2);
    padding: 8px;
    display: grid;
    gap: 6px;
  }

  .plan-title {
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 10px;
  }

  .plan-item {
    border-bottom: 1px solid rgba(128, 128, 128, 0.15);
    padding-bottom: 6px;
  }

  .plan-item:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .hint {
    color: var(--text-dim);
    font-size: 11px;
  }

  @media (max-width: 940px) {
    .cluster-grid {
      grid-template-columns: 1fr;
    }

    .inline-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
