<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let app: any;
  export let devMode = false;

  const dispatch = createEventDispatcher();
</script>

<div class="card">
  <div class="icon">{app.manifest?.icon || '📦'}</div>
  <div class="name">{app.manifest?.name || app.id}</div>
  <div class="tagline">{app.manifest?.tagline || ''}</div>

  <div class="footer">
    <button class="btn-launch" on:click={() => dispatch('launch')}>
      Launch
    </button>
    <button class="btn-help" on:click={() => dispatch('help')} title="Help">
      ?
    </button>
  </div>

  {#if devMode}
    <div class="dev-info">
      <div class="dev-row">
        <span class="dev-label">ID:</span>
        <span class="dev-value">{app.id}</span>
      </div>
      {#if app.manifest?.version}
        <div class="dev-row">
          <span class="dev-label">v{app.manifest.version}</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    padding: 16px;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .card:hover {
    border-color: #3fb950;
    box-shadow: 0 0 12px rgba(63, 185, 80, 0.1);
  }

  .icon {
    font-size: 28px;
    text-align: center;
  }

  .name {
    font-weight: 600;
    font-size: 14px;
    line-height: 1.4;
  }

  .tagline {
    font-size: 12px;
    color: #8b949e;
    line-height: 1.4;
    flex-grow: 1;
  }

  .footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    margin-top: auto;
  }

  .btn-launch {
    flex: 1;
    padding: 6px 12px;
    background: #3fb950;
    color: #0d1117;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: background 0.2s;
  }

  .btn-launch:hover {
    background: #35c743;
  }

  .btn-help {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.2s;
  }

  .btn-help:hover {
    color: #3fb950;
    border-color: #3fb950;
  }

  .dev-info {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid #30363d;
    font-size: 11px;
    color: #8b949e;
  }

  .dev-row {
    display: flex;
    justify-content: space-between;
    gap: 4px;
    margin: 2px 0;
    word-break: break-all;
  }

  .dev-label {
    font-weight: 500;
    color: #58a6ff;
  }

  .dev-value {
    font-family: 'Courier New', monospace;
    color: #79c0ff;
  }
</style>
