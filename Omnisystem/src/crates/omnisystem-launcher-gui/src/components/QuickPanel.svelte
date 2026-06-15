<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let apps = [];
  export let recentApps = [];

  function handleLaunch(app) {
    dispatch('launch', app);
  }

  function handleShowMain() {
    dispatch('show-main');
  }
</script>

<div class="quick-panel">
  <div class="panel-header">
    <h3>Quick Launch</h3>
    <button class="main-btn" on:click={handleShowMain} title="Show main window">
      ⬆
    </button>
  </div>

  <div class="panel-content">
    {#if recentApps && recentApps.length > 0}
      <div class="recent-section">
        <h4>Recent</h4>
        <div class="app-buttons">
          {#each recentApps.slice(0, 5) as app}
            <button
              class="quick-app-btn"
              on:click={() => handleLaunch(app)}
              title={app.name}
            >
              <span class="icon">{app.icon || '📦'}</span>
              <span class="name">{app.name}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    {#if apps && apps.length > 0}
      <div class="favorites-section">
        <h4>Favorites</h4>
        <div class="app-buttons">
          {#each apps.slice(0, 5) as app}
            <button
              class="quick-app-btn"
              on:click={() => handleLaunch(app)}
              title={app.name}
            >
              <span class="icon">{app.icon || '📦'}</span>
              <span class="name">{app.name}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .quick-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .panel-header {
    padding: 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .panel-header h3 {
    font-size: 14px;
    margin: 0;
    font-weight: 600;
  }

  .main-btn {
    background: none;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    font-size: 16px;
    padding: 4px 8px;
    transition: color 0.2s;
  }

  .main-btn:hover {
    color: var(--accent-light);
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .recent-section,
  .favorites-section {
    margin-bottom: 16px;
  }

  .recent-section h4,
  .favorites-section h4 {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0 0 8px 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .app-buttons {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .quick-app-btn {
    padding: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    transition: all 0.2s;
    text-align: left;
  }

  .quick-app-btn:hover {
    background: var(--bg-tertiary);
    border-color: var(--accent-light);
  }

  .icon {
    font-size: 16px;
    min-width: 24px;
    text-align: center;
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
