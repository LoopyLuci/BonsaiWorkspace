<script>
  import { onMount } from 'svelte';
  import { wsConnected, createWebSocket, fetchModules, fetchDatasets, fetchTrainingJobs } from './stores.js';
  import ModuleLibrary from './lib/ModuleLibrary.svelte';
  import DatasetManager from './lib/DatasetManager.svelte';
  import ModelDesigner from './lib/ModelDesigner.svelte';
  import ModelBuilder from './lib/ModelBuilder.svelte';
  import ModelConverter from './lib/ModelConverter.svelte';
  import TrainingMonitor from './lib/TrainingMonitor.svelte';

  let activeTab = 'modules';
  const tabs = [
    { id: 'modules', label: '📚 Modules', icon: '📚' },
    { id: 'datasets', label: '📊 Datasets', icon: '📊' },
    { id: 'designer', label: '🎨 Designer', icon: '🎨' },
    { id: 'builder', label: '🏋️ Builder', icon: '🏋️' },
    { id: 'converter', label: '🔄 Converter', icon: '🔄' },
    { id: 'monitor', label: '📈 Monitor', icon: '📈' },
  ];

  onMount(() => {
    createWebSocket();
    fetchModules();
    fetchDatasets();
    fetchTrainingJobs();
  });
</script>

<div class="workshop">
  <header class="header">
    <div class="title">
      <h1>🧬 Bonsai Model Workshop</h1>
      <span class="subtitle">Design • Build • Train • Convert</span>
    </div>
    <div class="status">
      <span class="ws-indicator" class:connected={$wsConnected}>
        {$wsConnected ? '🟢 Live' : '🔴 Connecting...'}
      </span>
    </div>
  </header>

  <nav class="tabs">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={activeTab === tab.id}
        on:click={() => activeTab = tab.id}
        title={tab.label}
      >
        {tab.icon}
        <span>{tab.label}</span>
      </button>
    {/each}
  </nav>

  <main class="content">
    {#if activeTab === 'modules'}
      <ModuleLibrary />
    {:else if activeTab === 'datasets'}
      <DatasetManager />
    {:else if activeTab === 'designer'}
      <ModelDesigner />
    {:else if activeTab === 'builder'}
      <ModelBuilder />
    {:else if activeTab === 'converter'}
      <ModelConverter />
    {:else if activeTab === 'monitor'}
      <TrainingMonitor />
    {/if}
  </main>
</div>

<style>
  .workshop {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
    color: #e0e0e0;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    background: linear-gradient(135deg, #0f3460 0%, #16213e 100%);
    border-bottom: 2px solid #e94560;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  .title {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title h1 {
    margin: 0;
    font-size: 28px;
    color: #e94560;
    font-weight: 700;
  }

  .subtitle {
    font-size: 12px;
    color: #888;
    letter-spacing: 2px;
    text-transform: uppercase;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .ws-indicator {
    font-size: 13px;
    color: #666;
    animation: pulse 2s infinite;
  }

  .ws-indicator.connected {
    color: #00b894;
    animation: none;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .tabs {
    display: flex;
    background: #16213e;
    border-bottom: 2px solid #0f3460;
    overflow-x: auto;
    gap: 0;
  }

  .tab {
    padding: 14px 20px;
    background: none;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 14px;
    white-space: nowrap;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    gap: 6px;
    border-bottom: 3px solid transparent;
  }

  .tab:hover {
    color: #e0e0e0;
    background: rgba(233, 69, 96, 0.1);
  }

  .tab.active {
    color: #e94560;
    border-bottom-color: #e94560;
    background: rgba(233, 69, 96, 0.15);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  :global(body) {
    margin: 0;
    padding: 0;
  }

  ::-webkit-scrollbar {
    width: 8px;
  }

  ::-webkit-scrollbar-track {
    background: #16213e;
  }

  ::-webkit-scrollbar-thumb {
    background: #0f3460;
    border-radius: 4px;
  }

  ::-webkit-scrollbar-thumb:hover {
    background: #1a1a2e;
  }
</style>
