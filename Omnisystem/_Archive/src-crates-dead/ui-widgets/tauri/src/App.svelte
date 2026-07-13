<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import AppList from './components/AppList.svelte';
  import SearchBar from './components/SearchBar.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import AppDetails from './components/AppDetails.svelte';
  import QuickPanel from './components/QuickPanel.svelte';

  let apps = [];
  let filteredApps = [];
  let searchQuery = '';
  let selectedApp = null;
  let loading = false;
  let error = null;
  let systemStatus = null;
  let view = 'list'; // 'list' or 'details'

  onMount(async () => {
    try {
      loading = true;
      // Load apps
      const appList = await invoke('list_apps');
      apps = appList;
      filteredApps = apps;

      // Load system status
      const status = await invoke('get_system_status');
      systemStatus = status;

      console.log('Omnisystem Launcher initialized');
    } catch (err) {
      error = 'Failed to load applications: ' + err;
      console.error('Error loading apps:', err);
    } finally {
      loading = false;
    }
  });

  function handleSearch(event) {
    searchQuery = event.detail;
    filterApps();
  }

  function filterApps() {
    if (!searchQuery.trim()) {
      filteredApps = apps;
    } else {
      const query = searchQuery.toLowerCase();
      filteredApps = apps.filter(app =>
        app.name.toLowerCase().includes(query) ||
        app.description.toLowerCase().includes(query) ||
        app.tags.some(tag => tag.toLowerCase().includes(query))
      );
    }
  }

  async function handleAppClick(event) {
    selectedApp = event.detail;
    view = 'details';
  }

  async function handleLaunch(event) {
    try {
      loading = true;
      const result = await invoke('launch_app', { appId: event.detail.id });
      if (result.success) {
        // Show success notification
        console.log('Launched:', result.message);
        // Reload instances
        const status = await invoke('get_system_status');
        systemStatus = status;
      } else {
        error = result.error || 'Failed to launch app';
      }
    } catch (err) {
      error = 'Error launching app: ' + err;
    } finally {
      loading = false;
    }
  }

  function backToList() {
    view = 'list';
    selectedApp = null;
  }
</script>

<main class="launcher-app">
  <div class="app-container">
    {#if view === 'list'}
      <div class="main-view">
        <div class="header">
          <h1>🚀 Omnisystem Launcher</h1>
          <p class="subtitle">Launch and manage your applications</p>
        </div>

        {#if error}
          <div class="error-banner">
            <strong>Error:</strong> {error}
            <button on:click={() => (error = null)}>✕</button>
          </div>
        {/if}

        <SearchBar {searchQuery} on:search={handleSearch} />

        {#if loading}
          <div class="loading">
            <div class="spinner"></div>
            <p>Loading applications...</p>
          </div>
        {:else}
          <AppList
            apps={filteredApps}
            on:click={handleAppClick}
            on:launch={handleLaunch}
          />
        {/if}
      </div>

      <StatusBar {systemStatus} />
    {:else if view === 'details' && selectedApp}
      <AppDetails
        app={selectedApp}
        on:back={backToList}
        on:launch={handleLaunch}
      />
    {/if}
  </div>
</main>

<style global>
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen,
      Ubuntu, Cantarell, 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
      sans-serif;
    background: #1e1e1e;
    color: #e0e0e0;
    overflow: hidden;
  }

  :root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #2d2d2d;
    --bg-tertiary: #3a3a3a;
    --text-primary: #e0e0e0;
    --text-secondary: #888;
    --border-color: #444;
    --accent-color: #0d47a1;
    --accent-light: #42a5f5;
    --success-color: #4caf50;
    --error-color: #f44336;
    --warning-color: #ff9800;
  }

  .launcher-app {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .app-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
  }

  .main-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .header {
    padding: 24px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .header h1 {
    font-size: 28px;
    font-weight: 600;
    margin-bottom: 4px;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 14px;
  }

  .error-banner {
    padding: 12px 24px;
    background: var(--error-color);
    color: white;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 14px;
  }

  .error-banner button {
    background: none;
    border: none;
    color: white;
    cursor: pointer;
    font-size: 18px;
    padding: 0;
  }

  .loading {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid var(--bg-tertiary);
    border-top: 4px solid var(--accent-light);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Scrollbar styling */
  ::-webkit-scrollbar {
    width: 8px;
  }

  ::-webkit-scrollbar-track {
    background: var(--bg-primary);
  }

  ::-webkit-scrollbar-thumb {
    background: var(--bg-tertiary);
    border-radius: 4px;
  }

  ::-webkit-scrollbar-thumb:hover {
    background: var(--border-color);
  }
</style>
