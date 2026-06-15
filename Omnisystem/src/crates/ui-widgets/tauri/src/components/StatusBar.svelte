<script>
  export let systemStatus = null;

  function formatUptime(seconds) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  }

  function formatMemory(mb) {
    if (mb > 1024) {
      return `${(mb / 1024).toFixed(1)}GB`;
    }
    return `${mb}MB`;
  }
</script>

<div class="status-bar">
  {#if systemStatus}
    <div class="status-item">
      <span class="label">Status:</span>
      <span class="value">
        {#if systemStatus.healthy}
          <span class="status-indicator healthy">●</span>
          Healthy
        {:else}
          <span class="status-indicator unhealthy">●</span>
          Unhealthy
        {/if}
      </span>
    </div>

    <div class="status-separator"></div>

    <div class="status-item">
      <span class="label">Apps:</span>
      <span class="value">{systemStatus.total_apps} total</span>
    </div>

    <div class="status-separator"></div>

    <div class="status-item">
      <span class="label">Running:</span>
      <span class="value">{systemStatus.active_instances} instances</span>
    </div>

    <div class="status-separator"></div>

    <div class="status-item">
      <span class="label">Memory:</span>
      <span class="value">
        {formatMemory(systemStatus.memory_used_mb)} / {formatMemory(systemStatus.memory_available_mb)}
      </span>
    </div>

    <div class="status-separator"></div>

    <div class="status-item">
      <span class="label">Uptime:</span>
      <span class="value">{formatUptime(systemStatus.uptime_seconds)}</span>
    </div>
  {:else}
    <div class="status-item">
      <span class="label">Loading status...</span>
    </div>
  {/if}
</div>

<style>
  .status-bar {
    padding: 8px 24px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .label {
    font-weight: 500;
    color: var(--text-secondary);
  }

  .value {
    color: var(--text-primary);
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
  }

  .status-separator {
    width: 1px;
    height: 16px;
    background: var(--border-color);
  }

  .status-indicator {
    margin-right: 4px;
    font-size: 10px;
  }

  .status-indicator.healthy {
    color: var(--success-color);
  }

  .status-indicator.unhealthy {
    color: var(--error-color);
  }
</style>
