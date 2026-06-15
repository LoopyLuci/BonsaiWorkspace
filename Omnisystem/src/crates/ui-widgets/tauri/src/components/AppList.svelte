<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let apps = [];

  function handleAppClick(app) {
    dispatch('click', app);
  }

  function handleLaunch(e, app) {
    e.stopPropagation();
    dispatch('launch', app);
  }
</script>

<div class="app-list">
  {#if apps.length === 0}
    <div class="empty-state">
      <p class="empty-icon">📭</p>
      <h3>No applications found</h3>
      <p>Try adjusting your search or check back later</p>
    </div>
  {:else}
    <div class="apps-grid">
      {#each apps as app (app.id)}
        <div
          class="app-card"
          on:click={() => handleAppClick(app)}
          role="button"
          tabindex="0"
        >
          <div class="app-icon">{app.icon || '📦'}</div>
          <div class="app-info">
            <h3 class="app-name">{app.name}</h3>
            <p class="app-desc">{app.description}</p>
            <div class="app-meta">
              <span class="category">{app.category}</span>
              <span class="version">v{app.version}</span>
            </div>
          </div>
          <button
            class="launch-btn"
            on:click={(e) => handleLaunch(e, app)}
            title="Launch {app.name}"
          >
            ▶
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .app-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
    color: var(--text-secondary);
  }

  .empty-icon {
    font-size: 64px;
    margin-bottom: 16px;
  }

  .empty-state h3 {
    font-size: 18px;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .apps-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 12px;
    padding: 8px;
  }

  .app-card {
    padding: 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 12px;
    transition: all 0.2s;
  }

  .app-card:hover {
    background: var(--bg-tertiary);
    border-color: var(--accent-light);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .app-icon {
    font-size: 32px;
    min-width: 48px;
    text-align: center;
  }

  .app-info {
    flex: 1;
    min-width: 0;
  }

  .app-name {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 4px 0;
    color: var(--text-primary);
  }

  .app-desc {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 8px 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .app-meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
  }

  .category,
  .version {
    padding: 2px 8px;
    background: var(--bg-tertiary);
    border-radius: 4px;
    color: var(--text-secondary);
  }

  .launch-btn {
    min-width: 44px;
    width: 44px;
    height: 44px;
    background: var(--accent-color);
    border: none;
    border-radius: 6px;
    color: white;
    cursor: pointer;
    font-size: 18px;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .launch-btn:hover {
    background: var(--accent-light);
    transform: scale(1.05);
  }

  .launch-btn:active {
    transform: scale(0.95);
  }
</style>
