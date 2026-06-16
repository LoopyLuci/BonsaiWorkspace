<script lang="ts">
  export let devMode = false;

  const capabilities = [
    { id: 'network.local', name: 'Local Network', granted: true, app: 'API Server' },
    { id: 'storage.user', name: 'User Storage', granted: true, app: 'Workspace' },
    { id: 'gpu.optional', name: 'GPU Access', granted: true, app: 'Inference' },
    { id: 'microphone', name: 'Microphone', granted: false, app: 'None' },
    { id: 'camera', name: 'Camera', granted: false, app: 'None' }
  ];
</script>

<div class="capabilities-tab">
  <div class="capabilities-list">
    <div class="section-title">Granted Capabilities</div>
    {#each capabilities.filter(c => c.granted) as cap (cap.id)}
      <div class="capability-item granted">
        <div class="cap-info">
          <div class="cap-name">{cap.name}</div>
          {#if devMode}
            <div class="cap-details">
              <span class="cap-id">{cap.id}</span>
              <span class="cap-app">{cap.app}</span>
            </div>
          {/if}
        </div>
        <button class="revoke-btn">Revoke</button>
      </div>
    {/each}

    <div class="section-title">Available Capabilities</div>
    {#each capabilities.filter(c => !c.granted) as cap (cap.id)}
      <div class="capability-item available">
        <div class="cap-info">
          <div class="cap-name">{cap.name}</div>
          {#if devMode}
            <div class="cap-details">
              <span class="cap-id">{cap.id}</span>
            </div>
          {/if}
        </div>
        <button class="grant-btn">Grant</button>
      </div>
    {/each}
  </div>
</div>

<style>
  .capabilities-tab {
    width: 100%;
  }

  .capabilities-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section-title {
    margin: 16px 0 8px 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    color: #8b949e;
    letter-spacing: 0.5px;
  }

  .capability-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
  }

  .cap-info {
    flex: 1;
  }

  .cap-name {
    font-weight: 500;
    font-size: 13px;
    margin-bottom: 4px;
  }

  .cap-details {
    display: flex;
    gap: 12px;
    font-size: 11px;
    color: #8b949e;
  }

  .cap-id {
    font-family: 'Courier New', monospace;
    color: #58a6ff;
  }

  .cap-app {
    color: #8b949e;
  }

  .revoke-btn,
  .grant-btn {
    padding: 4px 10px;
    background: transparent;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .revoke-btn {
    color: #f85149;
    border-color: #f85149;
  }

  .revoke-btn:hover {
    background: rgba(248, 81, 73, 0.1);
  }

  .grant-btn {
    color: #3fb950;
    border-color: #3fb950;
  }

  .grant-btn:hover {
    background: rgba(63, 185, 80, 0.1);
  }
</style>
