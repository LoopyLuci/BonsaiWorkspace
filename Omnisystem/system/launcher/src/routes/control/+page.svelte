<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import DevToggle from '../../lib/DevToggle.svelte';
  import StatusTab from '../../lib/tabs/StatusTab.svelte';
  import ServicesTab from '../../lib/tabs/ServicesTab.svelte';
  import CapabilitiesTab from '../../lib/tabs/CapabilitiesTab.svelte';
  import SettingsTab from '../../lib/tabs/SettingsTab.svelte';

  let activeTab = 'status';
  let devMode = false;

  const tabs = [
    { id: 'status', label: '📊 Status' },
    { id: 'services', label: '⚡ Services' },
    { id: 'capabilities', label: '🔧 Capabilities' },
    { id: 'settings', label: '⚙ Settings' }
  ];

  onMount(() => {
    const stored = localStorage.getItem('bonsai-dev-mode');
    devMode = stored === 'true';
  });

  function handleDevModeChange(event: CustomEvent) {
    devMode = event.detail;
    localStorage.setItem('bonsai-dev-mode', devMode ? 'true' : 'false');
  }
</script>

<div class="control-panel">
  <header class="panel-header">
    <h1 class="panel-title">🌿 Control Panel</h1>
    <div class="header-right">
      <DevToggle bind:devMode on:bonsai-dev-mode-changed={handleDevModeChange} />
    </div>
  </header>

  <nav class="tab-nav">
    {#each tabs as tab (tab.id)}
      <button
        class="tab-button"
        class:active={activeTab === tab.id}
        on:click={() => (activeTab = tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </nav>

  <div class="panel-content">
    {#if activeTab === 'status'}
      <StatusTab {devMode} />
    {:else if activeTab === 'services'}
      <ServicesTab {devMode} />
    {:else if activeTab === 'capabilities'}
      <CapabilitiesTab {devMode} />
    {:else if activeTab === 'settings'}
      <SettingsTab {devMode} />
    {/if}
  </div>
</div>

<style>
  .control-panel {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0d1117;
    color: #c9d1d9;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 24px;
    border-bottom: 1px solid #30363d;
    background: #161b22;
  }

  .panel-title {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .header-right {
    display: flex;
    gap: 8px;
  }

  .tab-nav {
    display: flex;
    gap: 0;
    padding: 0 24px;
    border-bottom: 1px solid #30363d;
    background: #161b22;
  }

  .tab-button {
    padding: 12px 16px;
    background: transparent;
    color: #8b949e;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .tab-button:hover {
    color: #c9d1d9;
  }

  .tab-button.active {
    color: #3fb950;
    border-bottom-color: #3fb950;
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }
</style>
