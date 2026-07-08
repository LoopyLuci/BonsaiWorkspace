<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();

  interface Lane {
    name: string;
    kind: string;
    healthy: boolean;
    rtt_ms?: number;
  }

  let lanes: Lane[] = [];
  let loading = false;
  let error = '';

  async function refresh() {
    loading = true;
    error = '';
    try {
      const resp = await invoke<{ lanes: Lane[] }>('rpc', { method: 'p2p.list_lanes', params: {} });
      lanes = resp.lanes ?? [];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function closeLane(name: string) {
    try {
      await invoke('rpc', { method: 'p2p.close_lane', params: { name } });
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => { void refresh(); });
</script>

<div class="panel">
  <div class="panel-header">
    <h2>P2P Peers</h2>
    <div class="header-actions">
      <button class="btn-sm" on:click={refresh} disabled={loading}>
        {loading ? '…' : 'Refresh'}
      </button>
      <button class="close-btn" on:click={() => dispatch('close')}>✕</button>
    </div>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if lanes.length === 0 && !loading}
    <p class="empty">No active lanes. Use p2p.start_webrtc / p2p.start_swarm / p2p.start_onion to connect.</p>
  {:else}
    <table class="lanes-table">
      <thead>
        <tr><th>Name</th><th>Kind</th><th>Status</th><th>RTT</th><th></th></tr>
      </thead>
      <tbody>
        {#each lanes as lane}
          <tr>
            <td class="mono">{lane.name}</td>
            <td>{lane.kind}</td>
            <td class:healthy={lane.healthy} class:unhealthy={!lane.healthy}>
              {lane.healthy ? 'healthy' : 'degraded'}
            </td>
            <td>{lane.rtt_ms != null ? `${lane.rtt_ms} ms` : '—'}</td>
            <td>
              <button class="btn-danger" on:click={() => closeLane(lane.name)}>Close</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .panel {
    position: fixed; right: 0; top: 44px; bottom: 0; width: 440px;
    background: #18181b; border-left: 1px solid #3f3f46;
    display: flex; flex-direction: column; z-index: 500;
    font-size: 13px; color: #e4e4e7;
  }
  .panel-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px; border-bottom: 1px solid #3f3f46;
    flex-shrink: 0;
  }
  h2 { font-size: 14px; font-weight: 600; color: #fff; margin: 0; }
  .header-actions { display: flex; gap: 8px; align-items: center; }
  .close-btn {
    background: none; border: none; color: #71717a; cursor: pointer;
    font-size: 16px; line-height: 1; padding: 2px 4px;
  }
  .close-btn:hover { color: #e4e4e7; }
  .btn-sm {
    padding: 3px 10px; border-radius: 5px; font-size: 12px; cursor: pointer;
    background: #27272a; border: 1px solid #3f3f46; color: #e4e4e7;
  }
  .btn-sm:hover { background: #3f3f46; }
  .btn-sm:disabled { opacity: 0.5; cursor: default; }
  .btn-danger {
    padding: 2px 8px; border-radius: 4px; font-size: 11px; cursor: pointer;
    background: #7f1d1d; border: 1px solid #ef4444; color: #fca5a5;
  }
  .btn-danger:hover { background: #991b1b; }
  .error { margin: 8px 16px; padding: 8px; background: #450a0a; border: 1px solid #b91c1c; border-radius: 6px; color: #fca5a5; font-size: 12px; }
  .empty { padding: 20px 16px; color: #71717a; font-size: 12px; line-height: 1.5; }
  .lanes-table { width: 100%; border-collapse: collapse; font-size: 12px; }
  .lanes-table th { padding: 6px 12px; text-align: left; color: #71717a; border-bottom: 1px solid #27272a; font-weight: 500; }
  .lanes-table td { padding: 6px 12px; border-bottom: 1px solid #27272a; }
  .mono { font-family: monospace; }
  .healthy { color: #4ade80; }
  .unhealthy { color: #f87171; }
</style>
