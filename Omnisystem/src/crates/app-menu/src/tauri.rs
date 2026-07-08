/// Tauri + Svelte desktop application structure
///
/// This module provides the complete Svelte component templates
/// for the desktop launcher UI built with Tauri.

/// Main App.svelte component - Root of the application
pub fn app_component() -> String {
    r#"
<script>
  import AppList from './components/AppList.svelte';
  import SearchBar from './components/SearchBar.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import { onMount } from 'svelte';

  let searchQuery = '';
  let selectedApps = [];

  onMount(async () => {
    // Initialize app on mount
    console.log('Launcher app initialized');
  });

  function handleSearch(event) {
    searchQuery = event.detail;
  }

  function handleLaunch(event) {
    console.log('Launching app:', event.detail);
  }
</script>

<main>
  <div class="window-header">
    <h1>🚀 Application Launcher</h1>
  </div>

  <SearchBar on:search={handleSearch} />

  <AppList {searchQuery} on:launch={handleLaunch} />

  <StatusBar />
</main>

<style global>
  :root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #2d2d2d;
    --bg-tertiary: #3a3a3a;
    --text-primary: #e0e0e0;
    --text-secondary: #888;
    --accent: #007acc;
    --accent-hover: #0098ff;
  }

  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--bg-primary);
    color: var(--text-primary);
    overflow: hidden;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  .window-header {
    background: var(--bg-secondary);
    padding: 16px 20px;
    border-bottom: 1px solid #444;
    user-select: none;
  }

  .window-header h1 {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.5px;
  }
</style>
    "#.to_string()
}

/// SearchBar.svelte component
pub fn search_bar_component() -> String {
    r#"
<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();
  let query = '';
  let inputElement;

  $: {
    if (query.length > 0) {
      dispatch('search', query);
    }
  }

  function clearSearch() {
    query = '';
    inputElement.focus();
    dispatch('search', '');
  }

  function onKeyDown(event) {
    if (event.key === 'Escape') {
      clearSearch();
    }
  }
</script>

<div class="search-container">
  <div class="search-box">
    <span class="search-icon">🔍</span>
    <input
      bind:this={inputElement}
      type="text"
      placeholder="Search applications..."
      bind:value={query}
      on:keydown={onKeyDown}
      class="search-input"
    />
    {#if query}
      <button on:click={clearSearch} class="clear-btn">✕</button>
    {/if}
  </div>
</div>

<style>
  .search-container {
    padding: 16px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid #444;
  }

  .search-box {
    display: flex;
    align-items: center;
    position: relative;
  }

  .search-icon {
    position: absolute;
    left: 12px;
    font-size: 16px;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 10px 12px 10px 40px;
    border: 1px solid #444;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-radius: 4px;
    font-size: 14px;
    transition: border-color 0.2s;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px rgba(0, 122, 204, 0.1);
  }

  .clear-btn {
    position: absolute;
    right: 10px;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 16px;
    padding: 4px;
    transition: color 0.2s;
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }
</style>
    "#.to_string()
}

/// AppList.svelte component - Display all apps
pub fn app_list_component() -> String {
    r#"
<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import AppCard from './AppCard.svelte';

  export let searchQuery = '';

  const dispatch = createEventDispatcher();
  let apps = [];
  let filteredApps = [];
  let loading = true;

  onMount(async () => {
    // In real implementation, fetch from launcher daemon
    apps = [
      {
        id: 'editor',
        name: 'Text Editor',
        version: '1.0.0',
        description: 'Edit text files',
        icon: '📝',
        executable: '/usr/bin/nano',
      },
      {
        id: 'browser',
        name: 'Web Browser',
        version: '2.0.0',
        description: 'Browse the web',
        icon: '🌐',
        executable: '/usr/bin/firefox',
      },
      {
        id: 'files',
        name: 'File Manager',
        version: '1.5.0',
        description: 'Manage files',
        icon: '📁',
        executable: '/usr/bin/nautilus',
      },
      {
        id: 'terminal',
        name: 'Terminal',
        version: '3.0.0',
        description: 'Command line interface',
        icon: '⌨️',
        executable: '/usr/bin/gnome-terminal',
      },
    ];
    loading = false;
  });

  $: {
    if (searchQuery.length === 0) {
      filteredApps = apps;
    } else {
      const q = searchQuery.toLowerCase();
      filteredApps = apps.filter(
        (app) =>
          app.name.toLowerCase().includes(q) ||
          app.description.toLowerCase().includes(q)
      );
    }
  }

  function handleLaunch(event) {
    dispatch('launch', event.detail);
  }
</script>

<div class="app-list">
  {#if loading}
    <div class="loading">Loading applications...</div>
  {:else if filteredApps.length === 0}
    <div class="empty">
      {#if searchQuery}
        No results for "{searchQuery}"
      {:else}
        No applications available
      {/if}
    </div>
  {:else}
    <div class="grid">
      {#each filteredApps as app (app.id)}
        <AppCard {app} on:launch={handleLaunch} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .app-list {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 16px;
  }

  .loading,
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    font-size: 14px;
  }
</style>
    "#.to_string()
}

/// AppCard.svelte component - Individual app card
pub fn app_card_component() -> String {
    r#"
<script>
  import { createEventDispatcher } from 'svelte';

  export let app = {};

  const dispatch = createEventDispatcher();
  let hovering = false;

  function launch() {
    dispatch('launch', { appId: app.id, appName: app.name });
  }
</script>

<div class="card" on:mouseenter={() => (hovering = true)} on:mouseleave={() => (hovering = false)}>
  <div class="icon">{app.icon}</div>
  <div class="info">
    <h3>{app.name}</h3>
    <p class="version">v{app.version}</p>
    <p class="description">{app.description}</p>
  </div>
  <button class="launch-btn" class:hovering on:click={launch}>
    Launch
  </button>
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border: 1px solid #444;
    border-radius: 8px;
    padding: 12px;
    text-align: center;
    transition: all 0.2s;
    cursor: pointer;
  }

  .card:hover {
    background: var(--bg-tertiary);
    border-color: var(--accent);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .icon {
    font-size: 48px;
    margin: 8px 0;
  }

  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    margin: 8px 0;
  }

  h3 {
    font-size: 13px;
    font-weight: 600;
    margin: 4px 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .version {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 2px 0;
  }

  .description {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 4px 0;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .launch-btn {
    width: 100%;
    padding: 6px;
    margin-top: 8px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    font-size: 12px;
    transition: background 0.2s;
  }

  .launch-btn:hover {
    background: var(--accent-hover);
  }

  .launch-btn:active {
    transform: scale(0.98);
  }
</style>
    "#.to_string()
}

/// StatusBar.svelte component - System status display
pub fn status_bar_component() -> String {
    r#"
<script>
  import { onMount } from 'svelte';

  let status = {
    healthy: true,
    activeInstances: 0,
    totalApps: 0,
    uptime: 0,
  };

  onMount(async () => {
    // Refresh status every 5 seconds
    const interval = setInterval(async () => {
      // In real implementation, fetch from launcher daemon
      // status = await fetchStatus();
    }, 5000);

    return () => clearInterval(interval);
  });
</script>

<div class="status-bar">
  <div class="status-item">
    <span class="label">System:</span>
    <span class="value">
      {#if status.healthy}
        <span class="indicator healthy">🟢 Healthy</span>
      {:else}
        <span class="indicator unhealthy">🔴 Unhealthy</span>
      {/if}
    </span>
  </div>

  <div class="separator" />

  <div class="status-item">
    <span class="label">Active:</span>
    <span class="value">{status.activeInstances}</span>
  </div>

  <div class="separator" />

  <div class="status-item">
    <span class="label">Apps:</span>
    <span class="value">{status.totalApps}</span>
  </div>
</div>

<style>
  .status-bar {
    background: var(--bg-secondary);
    padding: 10px 20px;
    border-top: 1px solid #444;
    display: flex;
    justify-content: flex-start;
    align-items: center;
    gap: 16px;
    user-select: none;
    font-size: 12px;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .label {
    color: var(--text-secondary);
  }

  .value {
    color: var(--text-primary);
    font-weight: 500;
  }

  .indicator {
    display: inline-block;
    font-size: 11px;
  }

  .healthy {
    color: #4ade80;
  }

  .unhealthy {
    color: #f87171;
  }

  .separator {
    width: 1px;
    height: 16px;
    background: #444;
  }
</style>
    "#.to_string()
}

/// Generate Svelte package.json
pub fn svelte_package_json() -> String {
    r#"{
  "name": "launcher-desktop",
  "version": "1.0.0",
  "description": "Launcher Desktop Application",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^2.4.2",
    "@tauri-apps/api": "^1.5.3",
    "@tauri-apps/cli": "^1.5.10",
    "svelte": "^4.2.0",
    "vite": "^5.0.0"
  }
}
"#.to_string()
}

/// Generate tauri.conf.json
pub fn tauri_config() -> String {
    r#"{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devPath": "http://localhost:5173",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Launcher",
        "width": 1024,
        "height": 768,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "execute": true
      }
    }
  }
}
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_component_generated() {
        let component = app_component();
        assert!(component.contains("<script>"));
        assert!(component.contains("AppList"));
        assert!(component.contains("SearchBar"));
    }

    #[test]
    fn test_search_bar_generated() {
        let component = search_bar_component();
        assert!(component.contains("search-input"));
        assert!(component.contains("placeholder"));
    }

    #[test]
    fn test_app_list_generated() {
        let component = app_list_component();
        assert!(component.contains("AppCard"));
        assert!(component.contains("filteredApps"));
    }

    #[test]
    fn test_app_card_generated() {
        let component = app_card_component();
        assert!(component.contains("launch-btn"));
        assert!(component.contains("icon"));
    }

    #[test]
    fn test_status_bar_generated() {
        let component = status_bar_component();
        assert!(component.contains("status-bar"));
        assert!(component.contains("Healthy"));
    }

    #[test]
    fn test_package_json_valid() {
        let json = svelte_package_json();
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_tauri_config_valid() {
        let json = tauri_config();
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }
}
