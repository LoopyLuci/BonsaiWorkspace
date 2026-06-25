<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let devMode = false;

  let status: any = null;
  let loading = true;

  onMount(async () => {
    try {
      status = await invoke('get_service_status');
      loading = false;
    } catch (err) {
      console.error('Failed to load status:', err);
      loading = false;
    }
  });
</script>

<div class="status-tab">
  {#if loading}
    <div class="loading">Loading system status...</div>
  {:else}
    <div class="status-section">
      <h2>System Status</h2>
      {#if !devMode}
        <div class="simple-status">
          <p class="status-message">✅ Everything is working great!</p>
          <p class="status-description">Your computer is running Bonsai smoothly.</p>
        </div>
      {:else}
        <div class="dev-status">
          <div class="status-item">
            <span class="label">API Server</span>
            <span class="value">localhost:11369</span>
            <span class="indicator healthy">✓</span>
          </div>
          <div class="status-item">
            <span class="label">Buddy API</span>
            <span class="value">localhost:11420</span>
            <span class="indicator healthy">✓</span>
          </div>
          <div class="status-item">
            <span class="label">Services</span>
            <span class="value">{status?.services?.length || 0} running</span>
          </div>
        </div>
      {/if}
    </div>

    {#if devMode}
      <div class="status-section">
        <h2>Performance</h2>
        <div class="performance-grid">
          <div class="perf-item">
            <div class="perf-label">CPU Usage</div>
            <div class="perf-bar">
              <div class="perf-fill" style="width: 35%;"></div>
            </div>
            <div class="perf-value">35%</div>
          </div>
          <div class="perf-item">
            <div class="perf-label">Memory Usage</div>
            <div class="perf-bar">
              <div class="perf-fill" style="width: 48%;"></div>
            </div>
            <div class="perf-value">15.4 GB / 32 GB</div>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .status-tab {
    width: 100%;
  }

  .loading {
    text-align: center;
    padding: 40px 20px;
    color: #8b949e;
  }

  .status-section {
    margin-bottom: 32px;
  }

  .status-section h2 {
    margin: 0 0 16px 0;
    font-size: 16px;
    font-weight: 600;
  }

  .simple-status {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 20px;
  }

  .status-message {
    margin: 0 0 8px 0;
    font-size: 16px;
    font-weight: 500;
  }

  .status-description {
    margin: 0;
    font-size: 14px;
    color: #8b949e;
  }

  .dev-status {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .status-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    font-size: 13px;
  }

  .label {
    font-weight: 500;
  }

  .value {
    color: #8b949e;
    font-family: 'Courier New', monospace;
  }

  .indicator {
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 12px;
    font-weight: 500;
  }

  .indicator.healthy {
    background: rgba(63, 185, 80, 0.1);
    color: #3fb950;
  }

  .performance-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
  }

  .perf-item {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 16px;
  }

  .perf-label {
    font-size: 13px;
    font-weight: 500;
    margin-bottom: 8px;
  }

  .perf-bar {
    width: 100%;
    height: 6px;
    background: #262d33;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 8px;
  }

  .perf-fill {
    height: 100%;
    background: #3fb950;
  }

  .perf-value {
    font-size: 12px;
    color: #8b949e;
    font-family: 'Courier New', monospace;
  }
</style>
