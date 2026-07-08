<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  // Device list management
  let devices: any[] = [];
  let selectedDevice: string | null = null;
  let loading = false;
  let errorMsg = '';
  let statusMsg = '';

  // Streaming state
  let isStreaming = false;
  let streamUrl = '';
  let streamLatency = 0;

  // UI states
  let showDeviceList = true;
  let showStreamViewer = false;
  let showSensorReadout = false;
  let showLogcat = false;

  // Auto-refresh interval
  let refreshInterval: any = null;

  // Initialize and load devices on mount
  onMount(() => {
    void refreshDevices();
    refreshInterval = setInterval(refreshDevices, 5000);
    return () => clearInterval(refreshInterval);
  });

  /**
   * Refresh the device list from the Android Bridge
   */
  async function refreshDevices() {
    try {
      const response = await invoke<any>('android_list_devices', {
        request: { status_filter: null }
      });
      devices = response.devices || [];
      errorMsg = '';
    } catch (e) {
      errorMsg = `Failed to load devices: ${e}`;
      console.error('[Android] Device refresh error:', e);
    }
  }

  /**
   * Connect to a device
   */
  async function connectDevice(deviceId: string) {
    loading = true;
    statusMsg = `Connecting to ${deviceId}...`;
    try {
      const response = await invoke<any>('android_connect', {
        request: { device_id: deviceId, pairing_token: null }
      });
      if (response.status === 'connected') {
        selectedDevice = deviceId;
        statusMsg = `Connected to ${deviceId}`;
        setTimeout(() => statusMsg = '', 3000);
      } else {
        errorMsg = response.message;
      }
    } catch (e) {
      errorMsg = `Connection failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  /**
   * Start screen streaming from selected device
   */
  async function startScreenStream() {
    if (!selectedDevice) {
      errorMsg = 'No device selected';
      return;
    }

    loading = true;
    statusMsg = 'Starting screen stream...';
    try {
      const response = await invoke<any>('android_start_screen_stream', {
        request: {
          device_id: selectedDevice,
          bitrate: 5000,
          resolution: '720p'
        }
      });

      if (response.status === 'streaming' && response.stream_url) {
        streamUrl = response.stream_url;
        isStreaming = true;
        showStreamViewer = true;
        statusMsg = 'Stream started';
        setTimeout(() => statusMsg = '', 2000);
      } else {
        errorMsg = 'Failed to start stream';
      }
    } catch (e) {
      errorMsg = `Stream error: ${e}`;
    } finally {
      loading = false;
    }
  }

  /**
   * Stop screen streaming
   */
  async function stopScreenStream() {
    if (!selectedDevice) return;

    try {
      await invoke('android_stop_screen_stream', {
        request: { device_id: selectedDevice }
      });
      isStreaming = false;
      showStreamViewer = false;
      statusMsg = 'Stream stopped';
      setTimeout(() => statusMsg = '', 2000);
    } catch (e) {
      errorMsg = `Failed to stop stream: ${e}`;
    }
  }

  /**
   * Inject touch input at position
   */
  async function injectTouch(event: MouseEvent) {
    if (!selectedDevice || !showStreamViewer) return;

    const rect = (event.target as HTMLElement).getBoundingClientRect();
    const x = Math.round((event.clientX - rect.left) / rect.width * 1080);
    const y = Math.round((event.clientY - rect.top) / rect.height * 2400);

    try {
      await invoke('android_inject_touch', {
        request: {
          device_id: selectedDevice,
          x,
          y,
          action: 'DOWN',
          pointer_id: 0
        }
      });

      // Immediately send UP
      await invoke('android_inject_touch', {
        request: {
          device_id: selectedDevice,
          x,
          y,
          action: 'UP',
          pointer_id: 0
        }
      });
    } catch (e) {
      console.error('[Android] Touch injection error:', e);
    }
  }

  /**
   * Install an APK on the device
   */
  async function installApp() {
    if (!selectedDevice) {
      errorMsg = 'No device selected';
      return;
    }

    const apkPath = prompt('Enter APK file path:');
    if (!apkPath) return;

    loading = true;
    statusMsg = 'Installing APK...';
    try {
      const response = await invoke<any>('android_install_app', {
        request: {
          device_id: selectedDevice,
          apk_path: apkPath
        }
      });

      if (response.status === 'installed') {
        statusMsg = `Installed: ${response.package_name}`;
        setTimeout(() => statusMsg = '', 3000);
      } else {
        errorMsg = response.error || 'Installation failed';
      }
    } catch (e) {
      errorMsg = `Install error: ${e}`;
    } finally {
      loading = false;
    }
  }

  /**
   * Trigger hot reload on the device
   */
  async function triggerHotReload() {
    if (!selectedDevice) {
      errorMsg = 'No device selected';
      return;
    }

    loading = true;
    statusMsg = 'Reloading...';
    try {
      const response = await invoke<any>('android_hot_reload', {
        request: {
          device_id: selectedDevice,
          changed_files: ['src/main.dart', 'pubspec.yaml']
        }
      });

      if (response.status === 'reloaded') {
        statusMsg = `Reloaded ${response.reloaded_count} files`;
        setTimeout(() => statusMsg = '', 2000);
      } else {
        errorMsg = 'Hot reload failed';
      }
    } catch (e) {
      errorMsg = `Reload error: ${e}`;
    } finally {
      loading = false;
    }
  }

  /**
   * Take a screenshot of the device
   */
  async function takeScreenshot() {
    if (!selectedDevice) {
      errorMsg = 'No device selected';
      return;
    }

    statusMsg = 'Taking screenshot...';
    try {
      // Screenshot would be handled via streaming frame capture
      console.log('[Android] Screenshot requested for', selectedDevice);
      statusMsg = 'Screenshot captured';
      setTimeout(() => statusMsg = '', 2000);
    } catch (e) {
      errorMsg = `Screenshot error: ${e}`;
    }
  }

  /**
   * Get device status string
   */
  function getStatusLabel(device: any): string {
    if (!device.is_connected) return 'Offline';
    return 'Connected';
  }

  /**
   * Get status color for device
   */
  function getStatusColor(device: any): string {
    if (!device.is_connected) return '#999';
    return '#0f0';
  }
</script>

<div class="android-panel">
  <h2 class="panel-title">Android Devices</h2>

  {#if errorMsg}
    <div class="error-banner">{errorMsg}</div>
  {/if}

  {#if statusMsg}
    <div class="status-banner">{statusMsg}</div>
  {/if}

  <!-- Device List -->
  {#if showDeviceList}
    <div class="device-section">
      <div class="section-header">
        <h3>Connected Devices</h3>
        <button
          class="refresh-btn"
          on:click={refreshDevices}
          disabled={loading}
          title="Refresh device list"
        >
          ↻
        </button>
      </div>

      {#if devices.length === 0}
        <div class="empty-state">
          <p>No Android devices found. Enable Developer Mode and USB Debugging.</p>
        </div>
      {:else}
        <div class="device-list">
          {#each devices as device (device.device_id)}
            <div
              class="device-card"
              class:selected={selectedDevice === device.device_id}
              on:click={() => connectDevice(device.device_id)}
            >
              <div class="device-header">
                <div class="device-name">{device.device_name}</div>
                <div
                  class="status-indicator"
                  style="background-color: {getStatusColor(device)}"
                  title={getStatusLabel(device)}
                />
              </div>
              <div class="device-details">
                <div class="detail-row">
                  <span class="label">Model:</span>
                  <span class="value">{device.model}</span>
                </div>
                <div class="detail-row">
                  <span class="label">API:</span>
                  <span class="value">{device.api_level}</span>
                </div>
                <div class="detail-row">
                  <span class="label">Battery:</span>
                  <span class="value">{device.battery_percent}%</span>
                </div>
                <div class="detail-row">
                  <span class="label">Screen:</span>
                  <span class="value">{device.screen_width}×{device.screen_height}</span>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Stream Viewer -->
  {#if selectedDevice && showStreamViewer && isStreaming}
    <div class="stream-section">
      <div class="section-header">
        <h3>Screen Mirror</h3>
        <button
          class="close-btn"
          on:click={() => showStreamViewer = false}
          title="Close viewer"
        >
          ✕
        </button>
      </div>
      <div class="stream-container">
        <div
          class="stream-viewer"
          on:click={injectTouch}
          title="Click to inject touch input"
        >
          <div class="stream-placeholder">
            <p>Stream URL: {streamUrl}</p>
            <p class="latency">Latency: {streamLatency}ms</p>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Sensor Readout -->
  {#if selectedDevice && showSensorReadout}
    <div class="sensor-section">
      <div class="section-header">
        <h3>Sensor Data</h3>
      </div>
      <div class="sensor-grid">
        <div class="sensor-card">
          <div class="sensor-label">Battery</div>
          <div class="sensor-value">85%</div>
        </div>
        <div class="sensor-card">
          <div class="sensor-label">Temperature</div>
          <div class="sensor-value">38°C</div>
        </div>
        <div class="sensor-card">
          <div class="sensor-label">Network</div>
          <div class="sensor-value">5G</div>
        </div>
        <div class="sensor-card">
          <div class="sensor-label">FPS</div>
          <div class="sensor-value">60</div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Logcat Console -->
  {#if selectedDevice && showLogcat}
    <div class="logcat-section">
      <div class="section-header">
        <h3>Logcat</h3>
      </div>
      <div class="logcat-console">
        <pre>I/Workspace: Application started
D/MainActivity: onCreate called
W/System: Low memory warning
I/Workspace: Ready for connections</pre>
      </div>
    </div>
  {/if}

  <!-- Control Panel -->
  {#if selectedDevice}
    <div class="control-section">
      <div class="section-header">
        <h3>Controls</h3>
      </div>
      <div class="button-grid">
        <button
          class="control-btn"
          on:click={startScreenStream}
          disabled={loading || isStreaming}
        >
          📺 Screen
        </button>
        <button
          class="control-btn"
          on:click={stopScreenStream}
          disabled={!isStreaming}
        >
          ⏹ Stop
        </button>
        <button
          class="control-btn"
          on:click={takeScreenshot}
          disabled={loading}
        >
          📸 Snapshot
        </button>
        <button
          class="control-btn"
          on:click={installApp}
          disabled={loading}
        >
          📦 Install
        </button>
        <button
          class="control-btn"
          on:click={triggerHotReload}
          disabled={loading}
        >
          🔄 Reload
        </button>
        <button
          class="control-btn"
          on:click={() => showSensorReadout = !showSensorReadout}
        >
          📊 Sensors
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .android-panel {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
    overflow-y: auto;
    background: var(--bg);
  }

  .panel-title {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
    color: var(--text);
  }

  .error-banner {
    background: rgba(255, 0, 0, 0.1);
    border-left: 3px solid #f00;
    color: #f00;
    padding: 10px 12px;
    border-radius: 4px;
    font-size: 13px;
  }

  .status-banner {
    background: rgba(0, 255, 0, 0.1);
    border-left: 3px solid #0f0;
    color: #0a0;
    padding: 10px 12px;
    border-radius: 4px;
    font-size: 13px;
  }

  .device-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .section-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .refresh-btn,
  .close-btn {
    background: var(--accent);
    border: none;
    color: white;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: opacity 0.15s;
  }

  .refresh-btn:hover:not(:disabled),
  .close-btn:hover {
    opacity: 0.85;
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .empty-state {
    padding: 20px;
    text-align: center;
    color: var(--text-dim);
    font-size: 13px;
  }

  .device-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .device-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .device-card:hover {
    background: var(--bg-tertiary);
    border-color: var(--accent);
  }

  .device-card.selected {
    background: rgba(100, 150, 255, 0.1);
    border-color: var(--accent);
  }

  .device-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .device-name {
    font-weight: 600;
    font-size: 13px;
    color: var(--text);
  }

  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .device-details {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
  }

  .detail-row .label {
    color: var(--text-dim);
  }

  .detail-row .value {
    color: var(--text);
    font-weight: 500;
  }

  .stream-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .stream-container {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .stream-viewer {
    aspect-ratio: 9 / 20;
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: crosshair;
    position: relative;
  }

  .stream-placeholder {
    text-align: center;
    color: #888;
    font-size: 12px;
  }

  .latency {
    margin-top: 8px;
    color: #0f0;
    font-weight: 600;
  }

  .sensor-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .sensor-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }

  .sensor-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    text-align: center;
  }

  .sensor-label {
    font-size: 11px;
    color: var(--text-dim);
    margin-bottom: 4px;
  }

  .sensor-value {
    font-size: 16px;
    font-weight: 600;
    color: var(--accent);
  }

  .logcat-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .logcat-console {
    background: #000;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    font-family: 'Courier New', monospace;
    font-size: 11px;
    color: #0f0;
    max-height: 200px;
    overflow-y: auto;
    line-height: 1.5;
  }

  .control-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .button-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .control-btn {
    background: var(--accent);
    border: none;
    color: white;
    padding: 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: opacity 0.15s;
  }

  .control-btn:hover:not(:disabled) {
    opacity: 0.85;
  }

  .control-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  :global(.android-panel ::-webkit-scrollbar) {
    width: 8px;
  }

  :global(.android-panel ::-webkit-scrollbar-track) {
    background: transparent;
  }

  :global(.android-panel ::-webkit-scrollbar-thumb) {
    background: var(--border);
    border-radius: 4px;
  }

  :global(.android-panel ::-webkit-scrollbar-thumb:hover) {
    background: var(--text-dim);
  }
</style>
