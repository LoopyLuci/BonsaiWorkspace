<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let devMode = false;

  let status: any = null;
  let statusText = 'Initializing...';
  let loading = true;

  onMount(async () => {
    try {
      status = await invoke('get_service_status');
      statusText = status?.healthy ? '✅ All systems nominal' : '⚠️ Some services degraded';
      loading = false;
    } catch (error) {
      statusText = '❌ Unable to get status';
      loading = false;
    }
  });
</script>

<div class="status-bar">
  <div class="status-left">
    <span class="status-dot running" title="All systems running" />
    <span class="status-text">{statusText}</span>
  </div>

  {#if devMode && status}
    <div class="status-dev">
      <span class="dev-item">API: {status.api || 'offline'}</span>
      {#if status.services}
        <span class="dev-item">{Object.keys(status.services).length} services</span>
      {/if}
    </div>
  {/if}

  <div class="status-right">
    <span class="version">v1.0.0</span>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 24px;
    border-top: 1px solid #30363d;
    background: #161b22;
    font-size: 12px;
    flex-shrink: 0;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    animation: breathing 2s ease-in-out infinite;
  }

  .status-dot.running {
    background: #3fb950;
    box-shadow: 0 0 8px rgba(63, 185, 80, 0.5);
  }

  .status-text {
    color: #8b949e;
  }

  .status-dev {
    display: flex;
    gap: 12px;
    color: #6e7681;
    font-family: 'Courier New', monospace;
    font-size: 11px;
  }

  .dev-item {
    padding: 2px 6px;
    background: rgba(88, 166, 255, 0.05);
    border-radius: 3px;
  }

  .status-right {
    color: #6e7681;
  }

  .version {
    font-size: 11px;
  }

  @keyframes breathing {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }
</style>
