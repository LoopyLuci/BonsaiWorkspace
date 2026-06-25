<script lang="ts">
  export let devMode = false;

  let updateChannel = 'stable';
  let autoStart = true;
  let showNotifications = true;
</script>

<div class="settings-tab">
  <div class="settings-sections">
    <div class="settings-section">
      <h3>General</h3>
      <div class="setting-item">
        <div class="setting-label">
          <div class="label-text">Auto-start with system</div>
          <div class="label-desc">Launch Bonsai automatically when your computer starts</div>
        </div>
        <input type="checkbox" bind:checked={autoStart} class="toggle" />
      </div>

      <div class="setting-item">
        <div class="setting-label">
          <div class="label-text">Show notifications</div>
          <div class="label-desc">Receive alerts from services and apps</div>
        </div>
        <input type="checkbox" bind:checked={showNotifications} class="toggle" />
      </div>
    </div>

    <div class="settings-section">
      <h3>Updates</h3>
      <div class="setting-item">
        <div class="setting-label">
          <div class="label-text">Update channel</div>
          <div class="label-desc">Choose how frequently you receive updates</div>
        </div>
        <select bind:value={updateChannel} class="select">
          <option value="stable">Stable (recommended)</option>
          <option value="beta">Beta</option>
          <option value="dev">Development</option>
        </select>
      </div>
    </div>

    {#if devMode}
      <div class="settings-section">
        <h3>Developer Settings</h3>
        <div class="setting-item">
          <div class="setting-label">
            <div class="label-text">API Base URL</div>
            <div class="label-desc">Backend API endpoint</div>
          </div>
          <input type="text" value="http://localhost:11369" class="input" disabled />
        </div>

        <div class="setting-item">
          <div class="setting-label">
            <div class="label-text">Log level</div>
          </div>
          <select class="select">
            <option>Info</option>
            <option>Debug</option>
            <option>Trace</option>
          </select>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-tab {
    width: 100%;
    max-width: 600px;
  }

  .settings-sections {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .settings-section {
    border-bottom: 1px solid #30363d;
    padding-bottom: 24px;
  }

  .settings-section:last-child {
    border-bottom: none;
  }

  .settings-section h3 {
    margin: 0 0 16px 0;
    font-size: 14px;
    font-weight: 600;
    text-transform: uppercase;
    color: #8b949e;
    letter-spacing: 0.5px;
  }

  .setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
    padding: 12px;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
  }

  .setting-item:last-child {
    margin-bottom: 0;
  }

  .setting-label {
    flex: 1;
  }

  .label-text {
    font-weight: 500;
    font-size: 13px;
    margin-bottom: 2px;
  }

  .label-desc {
    font-size: 11px;
    color: #8b949e;
  }

  .toggle {
    width: 40px;
    height: 20px;
    cursor: pointer;
    accent-color: #3fb950;
  }

  .select,
  .input {
    padding: 6px 10px;
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    font-size: 12px;
    font-family: inherit;
  }

  .select:focus,
  .input:focus {
    outline: none;
    border-color: #3fb950;
    box-shadow: 0 0 0 2px rgba(63, 185, 80, 0.1);
  }

  .input:disabled {
    color: #6e7681;
    background: #0d1117;
  }
</style>
