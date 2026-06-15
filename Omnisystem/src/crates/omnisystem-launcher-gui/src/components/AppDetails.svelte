<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let app = null;

  function handleBack() {
    dispatch('back');
  }

  function handleLaunch() {
    dispatch('launch', app);
  }
</script>

{#if app}
  <div class="app-details">
    <div class="details-header">
      <button class="back-btn" on:click={handleBack}>← Back</button>
      <h2>{app.name}</h2>
    </div>

    <div class="details-content">
      <div class="details-icon">{app.icon || '📦'}</div>

      <div class="details-section">
        <h3>About</h3>
        <p class="description">{app.description}</p>
      </div>

      <div class="details-grid">
        <div class="details-item">
          <label>Version</label>
          <value>{app.version}</value>
        </div>

        <div class="details-item">
          <label>Category</label>
          <value>{app.category}</value>
        </div>

        <div class="details-item">
          <label>Executable</label>
          <value class="mono">{app.executable}</value>
        </div>

        {#if app.working_dir}
          <div class="details-item">
            <label>Working Directory</label>
            <value class="mono">{app.working_dir}</value>
          </div>
        {/if}
      </div>

      {#if app.tags && app.tags.length > 0}
        <div class="details-section">
          <h3>Tags</h3>
          <div class="tags">
            {#each app.tags as tag}
              <span class="tag">{tag}</span>
            {/each}
          </div>
        </div>
      {/if}

      {#if app.args && app.args.length > 0}
        <div class="details-section">
          <h3>Arguments</h3>
          <div class="args-list">
            {#each app.args as arg}
              <div class="arg-item">{arg}</div>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <div class="details-actions">
      <button class="launch-btn" on:click={handleLaunch}>
        ▶ Launch {app.name}
      </button>
    </div>
  </div>
{/if}

<style>
  .app-details {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .details-header {
    padding: 16px 24px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .back-btn {
    background: none;
    border: none;
    color: var(--accent-light);
    cursor: pointer;
    font-size: 16px;
    padding: 4px 8px;
    transition: color 0.2s;
  }

  .back-btn:hover {
    color: white;
  }

  .details-header h2 {
    font-size: 24px;
    font-weight: 600;
  }

  .details-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .details-icon {
    font-size: 64px;
    margin-bottom: 24px;
  }

  .details-section {
    margin-bottom: 24px;
  }

  .details-section h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .description {
    font-size: 16px;
    line-height: 1.6;
    color: var(--text-primary);
    margin: 0;
  }

  .details-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
  }

  .details-item {
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 6px;
    border: 1px solid var(--border-color);
  }

  .details-item label {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .details-item value {
    display: block;
    color: var(--text-primary);
    font-size: 14px;
    word-break: break-word;
  }

  .mono {
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
    font-size: 12px;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .tag {
    padding: 4px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 20px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .args-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .arg-item {
    padding: 8px 12px;
    background: var(--bg-tertiary);
    border-left: 2px solid var(--accent-color);
    border-radius: 4px;
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
    font-size: 12px;
  }

  .details-actions {
    padding: 16px 24px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
    display: flex;
    gap: 12px;
  }

  .launch-btn {
    flex: 1;
    padding: 12px 24px;
    background: var(--accent-color);
    border: none;
    border-radius: 6px;
    color: white;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .launch-btn:hover {
    background: var(--accent-light);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(66, 165, 245, 0.3);
  }

  .launch-btn:active {
    transform: translateY(0);
  }
</style>
