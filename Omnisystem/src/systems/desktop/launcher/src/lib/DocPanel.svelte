<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let topic = '';
  export let devMode = false;

  const dispatch = createEventDispatcher();

  const docData: Record<string, { simple: string; dev: string }> = {
    launcher: {
      simple: 'The launcher is your starting point for opening any app on your computer.',
      dev: 'BonsaiLauncher is a Tauri 2.x desktop app that manages the app registry, service discovery, and app lifecycle.'
    },
    'app-menu': {
      simple: 'This menu shows all the apps you have installed. Click Launch to open one.',
      dev: 'Apps are registered via AppManifest in app.bonsai.toml. The registry maintains a DashMap of AppEntry records.'
    },
    services: {
      simple: 'Services are programs running in the background to help Bonsai work.',
      dev: 'Services are managed by the Service Lifecycle Manager (SLM) and expose health checks via HTTP endpoints.'
    },
    capabilities: {
      simple: 'Capabilities are permissions. When an app needs access to something, it asks for a capability.',
      dev: 'The capability broker implements fine-grained access control using UOSC syscalls and cryptographic tokens.'
    }
  };

  const content = docData[topic] || {
    simple: 'Information about this topic is not available.',
    dev: 'This topic has no developer documentation yet.'
  };

  function handleClose() {
    dispatch('close');
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }
</script>

<div class="backdrop" on:click={handleBackdropClick}>
  <div class="panel">
    <div class="panel-header">
      <h2 class="panel-title">📖 {topic}</h2>
      <button class="close-button" on:click={handleClose} title="Close">×</button>
    </div>

    <div class="panel-content">
      {#if !devMode}
        <div class="simple-mode">
          <p>{content.simple}</p>
        </div>
      {:else}
        <div class="dev-mode">
          <p>{content.dev}</p>
        </div>
      {/if}
    </div>

    <div class="panel-footer">
      <a href="#" class="doc-link">Learn more →</a>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: flex-end;
    z-index: 1000;
  }

  .panel {
    width: 320px;
    height: 100vh;
    background: #161b22;
    border-left: 1px solid #30363d;
    display: flex;
    flex-direction: column;
    animation: slideIn 0.3s ease-out;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px;
    border-bottom: 1px solid #30363d;
  }

  .panel-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .close-button {
    width: 32px;
    height: 32px;
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 20px;
    transition: all 0.2s;
  }

  .close-button:hover {
    background: #262d33;
    color: #c9d1d9;
  }

  .panel-content {
    flex: 1;
    padding: 16px;
    overflow-y: auto;
  }

  .simple-mode p,
  .dev-mode p {
    margin: 0 0 12px 0;
    font-size: 14px;
    line-height: 1.6;
    color: #c9d1d9;
  }

  .simple-mode p {
    font-size: 16px;
  }

  .dev-mode p {
    font-family: 'Courier New', monospace;
    font-size: 12px;
    color: #79c0ff;
  }

  .panel-footer {
    padding: 16px;
    border-top: 1px solid #30363d;
  }

  .doc-link {
    color: #58a6ff;
    text-decoration: none;
    font-size: 12px;
    transition: color 0.2s;
  }

  .doc-link:hover {
    color: #79c0ff;
  }
</style>
