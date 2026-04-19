<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { agentConfigs } from '$lib/stores/agents';
  import { availableModels, orchestratorStatus, refreshStatus } from '$lib/stores/models';

  const dispatch = createEventDispatcher<{ close: void }>();

  type HardwareInfo = {
    ram_total_gb: number;
    ram_available_gb: number;
    cpu_count: number;
    backend: string;
    gpu_names: string[];
  };

  let hardware: HardwareInfo | null = null;
  let loading = true;
  let error = '';
  let refreshTimer: ReturnType<typeof setInterval> | null = null;

  async function loadSnapshot() {
    try {
      const [hw] = await Promise.all([
        invoke<HardwareInfo>('get_hardware_info'),
        refreshStatus(),
      ]);
      hardware = hw;
      error = '';
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function cpuUsagePct(): number {
    if (!hardware || !$orchestratorStatus) return 0;
    const busy = $orchestratorStatus.slots.filter((s) => s.state.state === 'busy').length;
    const capacity = Math.max(1, hardware.cpu_count);
    return Math.round((busy / capacity) * 100);
  }

  function memoryUsagePct(): number {
    if (!hardware || !$orchestratorStatus || $orchestratorStatus.total_ram_mb <= 0) return 0;
    const used = $orchestratorStatus.total_ram_mb - $orchestratorStatus.free_ram_mb;
    return Math.min(100, Math.max(0, Math.round((used / $orchestratorStatus.total_ram_mb) * 100)));
  }

  function slotStateClass(state: string): string {
    if (state === 'busy') return 'busy';
    if (state === 'ready') return 'ready';
    if (state === 'loading') return 'loading';
    if (state === 'crashed') return 'crashed';
    return 'empty';
  }

  function modelLabel(modelId: string | null | undefined): string {
    if (!modelId) return 'Default';
    const found = $availableModels.find((m) => m.id === modelId);
    return found ? `${found.name} (${found.ram_label})` : modelId;
  }

  function estimatedAgentRamMb(modelId: string | null | undefined): number {
    if (!modelId) return 0;
    const found = $availableModels.find((m) => m.id === modelId);
    return found?.ram_required_mb ?? 0;
  }

  function slotStatus(slotIndex: number): { state: string; requests: number; idleSecs: number } {
    const slot = $orchestratorStatus?.slots.find((s) => s.index === slotIndex);
    if (!slot) {
      return { state: 'unassigned', requests: 0, idleSecs: 0 };
    }
    return {
      state: slot.state.state,
      requests: slot.requests,
      idleSecs: slot.idle_secs,
    };
  }

  onMount(async () => {
    await loadSnapshot();
    refreshTimer = setInterval(() => {
      void loadSnapshot();
    }, 2000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });
</script>

<div class="resources-overlay" on:click|self={() => dispatch('close')} role="presentation">
  <div class="resources-panel" role="dialog" aria-modal="true" aria-label="Resources dashboard">
    <header class="resources-header">
      <span class="resources-title">Resources</span>
      <button class="close-btn" on:click={() => dispatch('close')} aria-label="Close">✕</button>
    </header>

    <div class="resources-body">
      {#if loading}
        <div class="state">Loading resources...</div>
      {:else if error}
        <div class="state error">Unable to load resources: {error}</div>
      {:else}
        <section class="summary-grid">
          <article class="card">
            <div class="card-label">System RAM</div>
            <div class="card-value">{hardware?.ram_total_gb ?? 0} GB</div>
            <div class="meter">
              <div class="fill" style:width={`${memoryUsagePct()}%`}></div>
            </div>
            <div class="card-sub">Used {memoryUsagePct()}% · Free {$orchestratorStatus?.free_ram_mb ?? 0} MB</div>
          </article>

          <article class="card">
            <div class="card-label">CPU Workload</div>
            <div class="card-value">{cpuUsagePct()}%</div>
            <div class="meter">
              <div class="fill amber" style:width={`${cpuUsagePct()}%`}></div>
            </div>
            <div class="card-sub">Busy slots / cores heuristic</div>
          </article>

          <article class="card">
            <div class="card-label">Inference Backend</div>
            <div class="card-value">{hardware?.backend ?? 'Unknown'}</div>
            <div class="card-sub">GPU(s): {(hardware?.gpu_names ?? []).join(', ')}</div>
          </article>

          <article class="card">
            <div class="card-label">Queue Depth</div>
            <div class="card-value">{$orchestratorStatus?.queue_depth ?? 0}</div>
            <div class="card-sub">Live orchestrator queue</div>
          </article>
        </section>

        <section class="agents-section">
          <div class="section-title">Per-Agent Resource View</div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Agent</th>
                  <th>Slot</th>
                  <th>Model</th>
                  <th>Est. RAM</th>
                  <th>Slot State</th>
                  <th>Requests</th>
                  <th>Idle</th>
                </tr>
              </thead>
              <tbody>
                {#each $agentConfigs as resolved (resolved.config.id)}
                  {@const slot = slotStatus(resolved.config.slot_index)}
                  <tr>
                    <td>{resolved.config.icon_emoji} {resolved.config.label}</td>
                    <td>{resolved.config.slot_index}</td>
                    <td>{modelLabel(resolved.config.model_id)}</td>
                    <td>{estimatedAgentRamMb(resolved.config.model_id)} MB</td>
                    <td>
                      <span class={`state-pill ${slotStateClass(slot.state)}`}>{slot.state}</span>
                    </td>
                    <td>{slot.requests}</td>
                    <td>{slot.idleSecs}s</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <p class="note">Estimated RAM is model-based. Slot state/requests are live runtime telemetry.</p>
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .resources-overlay {
    position: fixed;
    inset: 0;
    z-index: 210;
    background: rgba(0, 0, 0, 0.56);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .resources-panel {
    width: min(1180px, 96vw);
    max-height: 90vh;
    border-radius: 10px;
    border: 1px solid var(--border, #333);
    background: var(--bg2, #1e1e2e);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .resources-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border, #333);
  }

  .resources-title {
    font-size: 15px;
    font-weight: 700;
  }

  .close-btn {
    border: none;
    background: none;
    color: var(--text-dim, #888);
    cursor: pointer;
    font-size: 16px;
  }

  .resources-body {
    overflow: auto;
    padding: 14px;
    display: grid;
    gap: 14px;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(160px, 1fr));
    gap: 10px;
  }

  .card {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    background: var(--bg, #141420);
    padding: 10px;
    display: grid;
    gap: 7px;
  }

  .card-label {
    font-size: 11px;
    color: var(--text-dim, #888);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .card-value {
    font-size: 16px;
    font-weight: 700;
  }

  .card-sub {
    font-size: 12px;
    color: var(--text-dim, #999);
  }

  .meter {
    height: 7px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: linear-gradient(90deg, #16a34a, #4ade80);
  }

  .fill.amber {
    background: linear-gradient(90deg, #f59e0b, #fbbf24);
  }

  .agents-section {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    background: var(--bg, #141420);
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .section-title {
    font-size: 13px;
    font-weight: 700;
  }

  .table-wrap {
    overflow: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    min-width: 760px;
  }

  th,
  td {
    text-align: left;
    border-bottom: 1px solid var(--border, #333);
    padding: 8px;
    font-size: 12px;
    white-space: nowrap;
  }

  th {
    color: var(--text-dim, #999);
    font-weight: 600;
  }

  .state-pill {
    display: inline-flex;
    border-radius: 999px;
    padding: 2px 8px;
    border: 1px solid var(--border, #333);
    font-size: 11px;
  }

  .state-pill.ready {
    color: #4ade80;
    border-color: #4ade80;
  }

  .state-pill.busy {
    color: #fbbf24;
    border-color: #fbbf24;
  }

  .state-pill.loading {
    color: #93c5fd;
    border-color: #93c5fd;
  }

  .state-pill.crashed {
    color: #f87171;
    border-color: #f87171;
  }

  .note {
    font-size: 11px;
    color: var(--text-dim, #888);
  }

  .state {
    font-size: 13px;
    color: var(--text-dim, #888);
  }

  .state.error {
    color: #f87171;
  }

  @media (max-width: 1020px) {
    .summary-grid {
      grid-template-columns: repeat(2, minmax(160px, 1fr));
    }
  }

  @media (max-width: 640px) {
    .resources-panel {
      width: 100vw;
      max-height: 100vh;
      border-radius: 0;
      border-left: none;
      border-right: none;
    }

    .summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
