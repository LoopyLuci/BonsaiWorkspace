<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import AppCard from '../lib/AppCard.svelte';
  import SearchBar from '../lib/SearchBar.svelte';
  import StatusBar from '../lib/StatusBar.svelte';
  import DocPanel from '../lib/DocPanel.svelte';
  import DevToggle from '../lib/DevToggle.svelte';
  import CategoryTabs from '../lib/CategoryTabs.svelte';

  interface App {
    id: string;
    manifest: {
      name: string;
      tagline: string;
      description: string;
      category?: string;
      icon?: string;
    };
  }

  let apps: App[] = [];
  let filteredApps: App[] = [];
  let featured: App[] = [];
  let selectedCategory = 'all';
  let searchQuery = '';
  let devMode = false;
  let docTopic: string | null = null;
  let showDocPanel = false;
  let loading = true;
  let error: string | null = null;

  const categories = [
    { id: 'all', label: 'All' },
    { id: 'productivity', label: 'Productivity' },
    { id: 'developer', label: 'Developer' },
    { id: 'system', label: 'System' },
    { id: 'utility', label: 'Utility' }
  ];

  onMount(async () => {
    try {
      const stored = localStorage.getItem('bonsai-dev-mode');
      devMode = stored === 'true';

      featured = await invoke('get_featured_apps') as App[];
      apps = await invoke('get_apps') as App[];
      filterApps();
      loading = false;
    } catch (err) {
      error = `Failed to load apps: ${err}`;
      loading = false;
    }
  });

  function filterApps() {
    let filtered = apps;

    if (selectedCategory !== 'all') {
      filtered = filtered.filter(app => app.manifest?.category === selectedCategory);
    }

    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(app =>
        app.manifest?.name.toLowerCase().includes(query) ||
        app.manifest?.tagline.toLowerCase().includes(query)
      );
    }

    filteredApps = filtered;
  }

  function handleSearch(event: CustomEvent) {
    searchQuery = event.detail;
    filterApps();
  }

  function handleCategoryChange(event: CustomEvent) {
    selectedCategory = event.detail;
    filterApps();
  }

  function handleDevModeChange(event: CustomEvent) {
    devMode = event.detail;
    localStorage.setItem('bonsai-dev-mode', devMode ? 'true' : 'false');
  }

  function handleDocClick(topic: string) {
    docTopic = topic;
    showDocPanel = true;
  }

  function handleCloseDoc() {
    showDocPanel = false;
    docTopic = null;
  }

  async function handleAppLaunch(appId: string) {
    try {
      await invoke('launch_app', { appId });
    } catch (err) {
      error = `Failed to launch app: ${err}`;
    }
  }
</script>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #0d1117;
    color: #c9d1d9;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  }

  .launcher {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0d1117;
    color: #c9d1d9;
  }

  .launcher-header {
    background: #161b22;
    border-bottom: 1px solid #30363d;
    padding: 16px 24px;
    flex-shrink: 0;
  }

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .launcher-title {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .header-right {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .doc-button {
    width: 32px;
    height: 32px;
    border-radius: 4px;
    border: 1px solid #30363d;
    background: transparent;
    color: #c9d1d9;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.2s;
  }

  .doc-button:hover {
    background: #262d33;
    border-color: #3fb950;
  }

  .error-banner {
    background: rgba(248, 81, 73, 0.1);
    color: #f85149;
    padding: 12px 24px;
    border-bottom: 1px solid #f85149;
    font-size: 14px;
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    font-size: 16px;
    color: #8b949e;
  }

  .launcher-main {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .featured-section {
    margin-bottom: 40px;
  }

  .featured-section h2 {
    margin: 0 0 16px 0;
    font-size: 14px;
    font-weight: 600;
    text-transform: uppercase;
    color: #8b949e;
    letter-spacing: 0.5px;
  }

  .featured-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 16px;
    margin-bottom: 32px;
  }

  .apps-section h2 {
    margin: 0 0 16px 0;
    font-size: 14px;
    font-weight: 600;
    text-transform: uppercase;
    color: #8b949e;
    letter-spacing: 0.5px;
  }

  .apps-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 16px;
  }

  .no-results {
    text-align: center;
    padding: 40px 20px;
    color: #8b949e;
    font-size: 14px;
  }
</style>

<div class="launcher">
  <header class="launcher-header">
    <div class="header-content">
      <h1 class="launcher-title">🌿 BonsaiWorkspace</h1>
      <div class="header-right">
        <DevToggle bind:devMode on:bonsai-dev-mode-changed={handleDevModeChange} />
        <button class="doc-button" on:click={() => handleDocClick('launcher')} title="Help">?</button>
      </div>
    </div>
    <SearchBar on:search={handleSearch} />
  </header>

  {#if error}
    <div class="error-banner">
      ⚠️ {error}
    </div>
  {/if}

  {#if loading}
    <div class="loading">Loading apps...</div>
  {:else}
    <main class="launcher-main">
      {#if featured.length > 0}
        <section class="featured-section">
          <h2>Featured</h2>
          <div class="featured-grid">
            {#each featured as app (app.id)}
              <AppCard
                {app}
                {devMode}
                on:launch={() => handleAppLaunch(app.id)}
                on:help={() => handleDocClick(app.id)}
              />
            {/each}
          </div>
        </section>
      {/if}

      <section class="apps-section">
        <CategoryTabs
          {categories}
          selected={selectedCategory}
          on:category-change={handleCategoryChange}
        />

        <div class="apps-grid">
          {#each filteredApps as app (app.id)}
            <AppCard
              {app}
              {devMode}
              on:launch={() => handleAppLaunch(app.id)}
              on:help={() => handleDocClick(app.id)}
            />
          {/each}
        </div>

        {#if filteredApps.length === 0}
          <div class="no-results">
            No apps found. Try a different search or category.
          </div>
        {/if}
      </section>
    </main>
  {/if}

  <StatusBar {devMode} />

  {#if showDocPanel && docTopic}
    <DocPanel topic={docTopic} {devMode} on:close={handleCloseDoc} />
  {/if}
</div>
