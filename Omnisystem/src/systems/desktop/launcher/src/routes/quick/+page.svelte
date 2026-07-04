<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import AppCard from '../../lib/AppCard.svelte';
  import SearchBar from '../../lib/SearchBar.svelte';
  import StatusBar from '../../lib/StatusBar.svelte';
  import DevToggle from '../../lib/DevToggle.svelte';

  interface App {
    id: string;
    manifest: {
      name: string;
      tagline: string;
      description: string;
    };
  }

  let apps: App[] = [];
  let featured: App[] = [];
  let running: App[] = [];
  let searchQuery = '';
  let devMode = false;
  let filteredApps: App[] = [];

  onMount(async () => {
    try {
      const stored = localStorage.getItem('bonsai-dev-mode');
      devMode = stored === 'true';

      featured = await invoke('get_featured_apps') as App[];
      apps = await invoke('get_apps') as App[];
      running = featured.slice(0, 2); // Show top 2 as "running"
    } catch (err) {
      console.error('Failed to load apps:', err);
    }
  });

  function handleSearch(event: CustomEvent) {
    searchQuery = event.detail;
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filteredApps = apps.filter(app =>
        app.manifest?.name.toLowerCase().includes(query) ||
        app.manifest?.tagline.toLowerCase().includes(query)
      );
    } else {
      filteredApps = [];
    }
  }

  async function handleAppLaunch(appId: string) {
    try {
      await invoke('launch_app', { appId });
    } catch (err) {
      console.error('Failed to launch app:', err);
    }
  }

  function handleDevModeChange(event: CustomEvent) {
    devMode = event.detail;
    localStorage.setItem('bonsai-dev-mode', devMode ? 'true' : 'false');
  }
</script>

<div class="quick-panel">
  <header class="panel-header">
    <h1 class="panel-title">🌿 Bonsai</h1>
    <div class="header-right">
      <DevToggle bind:devMode on:bonsai-dev-mode-changed={handleDevModeChange} />
    </div>
  </header>

  <SearchBar on:search={handleSearch} />

  <div class="panel-content">
    {#if searchQuery && filteredApps.length > 0}
      <div class="search-results">
        {#each filteredApps.slice(0, 4) as app (app.id)}
          <AppCard
            {app}
            {devMode}
            on:launch={() => handleAppLaunch(app.id)}
          />
        {/each}
      </div>
    {:else}
      <section class="running-section">
        <h2>Running</h2>
        <div class="app-list">
          {#each running as app (app.id)}
            <div class="app-item">
              <span class="app-name">{app.manifest?.name}</span>
              <span class="status-dot running" />
            </div>
          {/each}
        </div>
      </section>

      <section class="quick-launch">
        <h2>Quick Launch</h2>
        <div class="quick-buttons">
          {#each featured.slice(0, 3) as app (app.id)}
            <button
              class="quick-btn"
              on:click={() => handleAppLaunch(app.id)}
              title={app.manifest?.name}
            >
              {app.manifest?.name.slice(0, 1)}
            </button>
          {/each}
        </div>
      </section>
    {/if}
  </div>

  <StatusBar {devMode} />

  <div class="panel-actions">
    <button class="action-btn">All Apps →</button>
    <button class="action-btn">Docs →</button>
  </div>
</div>

<style>
  .quick-panel {
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
    padding: 12px 16px;
    border-bottom: 1px solid #30363d;
    background: #161b22;
  }

  .panel-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .header-right {
    display: flex;
    gap: 8px;
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .search-results {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .running-section,
  .quick-launch {
    margin-bottom: 16px;
  }

  .running-section h2,
  .quick-launch h2 {
    margin: 0 0 8px 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    color: #8b949e;
    letter-spacing: 0.5px;
  }

  .app-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .app-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    background: #161b22;
    border-radius: 4px;
    border: 1px solid #30363d;
    font-size: 12px;
  }

  .app-name {
    flex: 1;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #3fb950;
  }

  .quick-buttons {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .quick-btn {
    aspect-ratio: 1;
    background: #161b22;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.2s;
  }

  .quick-btn:hover {
    border-color: #3fb950;
    background: #262d33;
  }

  .panel-actions {
    display: flex;
    gap: 8px;
    padding: 12px;
    border-top: 1px solid #30363d;
  }

  .action-btn {
    flex: 1;
    padding: 8px 12px;
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .action-btn:hover {
    border-color: #3fb950;
    color: #3fb950;
  }
</style>
