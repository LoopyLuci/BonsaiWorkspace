<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher<{ close: void }>();

  type AndroidUsbDevice = {
    serial: string;
    state: string;
    model?: string;
    device?: string;
  };

  type MobileStatus = {
    adb_executable: string;
    scrcpy_executable: string;
    scrcpy_available: boolean;
    scrcpy_candidates?: string[];
    active_views?: Array<{ serial: string; pid: number }>;
    active_recordings?: Array<{ serial: string; pid: number; remote_path: string }>;
  };

  type DisplayInfo = {
    width?: number;
    height?: number;
    density_dpi?: number;
    surface_orientation?: number;
  };

  type PrepareRuntimeOut = {
    ok?: boolean;
    serial?: string;
    remote_surface_enabled?: boolean;
    remote_surface?: {
      session_id?: string;
      pair_token?: string;
      frame_url?: string;
      input_url?: string;
      stop_url?: string;
    };
  };

  let busy = false;
  let error = '';
  let logText = '';

  let devices: AndroidUsbDevice[] = [];
  let selectedSerial = '';
  let status: MobileStatus | null = null;

  let maxSize = 1600;
  let bitrateMbps = 12;
  let fullscreen = false;
  let stayAwake = true;
  let turnScreenOff = false;

  let recordingBitrate = 10;
  let textInput = '';
  let runtimeApiPort = 11369;
  let runtimeWsPort = 11371;
  let displayInfo: DisplayInfo | null = null;
  let settingsOpen = false;

  let remoteSessionId = '';
  let pairToken = '';
  let frameUrl = '';
  let inputUrl = '';
  let frameObjectUrl = '';
  let framePollMs = 350;
  let framePollTimer: ReturnType<typeof setInterval> | null = null;
  let frameLoading = false;
  let frameConnected = false;
  let lastFrameAt = '';

  type DragPoint = { x: number; y: number };
  let pointerStart: DragPoint | null = null;

  function appendLog(title: string, payload: unknown) {
    const body = typeof payload === 'string' ? payload : JSON.stringify(payload);
    const line = `[${new Date().toISOString()}] ${title}: ${body}`;
    logText = logText ? `${line}\n${logText}` : line;
  }

  function serialOrThrow(): string {
    const serial = selectedSerial.trim();
    if (!serial) {
      throw new Error('Select a connected device first.');
    }
    return serial;
  }

  function activeViewFor(serial: string): { serial: string; pid: number } | null {
    if (!status?.active_views?.length) return null;
    return status.active_views.find((v) => v.serial === serial) ?? null;
  }

  function withAuthHeaders(): Record<string, string> {
    if (!pairToken) return {};
    return { 'x-bonsai-token': pairToken };
  }

  function clearFrameObjectUrl() {
    if (!frameObjectUrl) return;
    URL.revokeObjectURL(frameObjectUrl);
    frameObjectUrl = '';
  }

  async function withBusy<T>(title: string, task: () => Promise<T>) {
    busy = true;
    error = '';
    try {
      const out = await task();
      appendLog(title, out);
      return out;
    } catch (e) {
      error = String(e);
      appendLog(`${title} failed`, error);
      throw e;
    } finally {
      busy = false;
    }
  }

  async function refreshDevices() {
    await withBusy('Refresh devices', async () => {
      const out = await invoke<{ devices?: AndroidUsbDevice[] }>('android_usb_list_devices');
      devices = Array.isArray(out?.devices) ? out.devices : [];
      if (devices.length > 0 && !devices.some((d) => d.serial === selectedSerial)) {
        selectedSerial = devices[0].serial;
      }
      await refreshStatus();
      return { count: devices.length, selected: selectedSerial || null };
    });
  }

  async function refreshStatus() {
    const out = await invoke<MobileStatus>('android_mobile_view_status');
    status = out;
    return out;
  }

  async function refreshDisplayInfo() {
    const serial = serialOrThrow();
    const out = await invoke<DisplayInfo>('android_mobile_get_display_info', { serial });
    displayInfo = out;
    appendLog('Read display info', out);
    return out;
  }

  async function startLiveView() {
    const serial = serialOrThrow();
    await withBusy('Start Mobile Viewer', async () => {
      const out = await invoke('android_mobile_view_start', {
        serial,
        maxSize: Number(maxSize || 1600),
        bitrateMbps: Number(bitrateMbps || 12),
        fullscreen,
        stayAwake,
        turnScreenOff,
      });
      await refreshStatus();
      return out;
    });
  }

  async function stopLiveView() {
    const serial = serialOrThrow();
    await withBusy('Stop Mobile Viewer', async () => {
      const out = await invoke('android_mobile_view_stop', { serial });
      await refreshStatus();
      return out;
    });
  }

  async function prepareViewerRuntime() {
    const serial = serialOrThrow();
    await withBusy('Prepare Mobile Viewer runtime', async () => {
      const out = await invoke<PrepareRuntimeOut>('android_mobile_prepare_uniform_runtime', {
        serial,
        apiPort: Number(runtimeApiPort || 11369),
        wsPort: Number(runtimeWsPort || 11371),
        startRemoteSurface: true,
      });

      remoteSessionId = out?.remote_surface?.session_id || '';
      pairToken = out?.remote_surface?.pair_token || '';
      frameUrl = out?.remote_surface?.frame_url || '';
      inputUrl = out?.remote_surface?.input_url || '';
      frameConnected = false;
      startFramePolling();
      await refreshDisplayInfo();
      return out;
    });
  }

  async function fetchFrame() {
    if (!frameUrl || frameLoading) return;
    frameLoading = true;
    try {
      const response = await fetch(frameUrl, {
        method: 'GET',
        headers: withAuthHeaders(),
        cache: 'no-store',
      });

      if (!response.ok) {
        throw new Error(`Frame fetch failed (${response.status})`);
      }

      const blob = await response.blob();
      clearFrameObjectUrl();
      frameObjectUrl = URL.createObjectURL(blob);
      frameConnected = true;
      lastFrameAt = new Date().toLocaleTimeString();
    } catch (e) {
      frameConnected = false;
      error = String(e);
    } finally {
      frameLoading = false;
    }
  }

  function startFramePolling() {
    stopFramePolling();
    if (!frameUrl) return;
    void fetchFrame();
    framePollTimer = setInterval(() => {
      void fetchFrame();
    }, Math.max(120, framePollMs));
  }

  function stopFramePolling() {
    if (framePollTimer) {
      clearInterval(framePollTimer);
      framePollTimer = null;
    }
  }

  async function startViewerStack() {
    await startLiveView();
    await prepareViewerRuntime();
  }

  async function stopViewerStack() {
    stopFramePolling();
    clearFrameObjectUrl();
    remoteSessionId = '';
    frameUrl = '';
    inputUrl = '';
    pairToken = '';
    frameConnected = false;
    await stopLiveView();
  }

  async function takeScreenshot() {
    const serial = serialOrThrow();
    await withBusy('Take screenshot', () => invoke('android_mobile_take_screenshot', { serial }));
    if (frameUrl) {
      void fetchFrame();
    }
  }

  async function startRecording() {
    const serial = serialOrThrow();
    await withBusy('Start screen recording', async () => {
      const out = await invoke('android_mobile_start_recording', {
        serial,
        bitrateMbps: Number(recordingBitrate || 10),
      });
      await refreshStatus();
      return out;
    });
  }

  async function stopRecording() {
    const serial = serialOrThrow();
    await withBusy('Stop screen recording', async () => {
      const out = await invoke('android_mobile_stop_recording', { serial });
      await refreshStatus();
      return out;
    });
  }

  async function launchCamera(videoMode: boolean) {
    const serial = serialOrThrow();
    await withBusy(videoMode ? 'Launch camera (video)' : 'Launch camera (photo)', () =>
      invoke('android_mobile_launch_camera', { serial, videoMode }),
    );
  }

  async function launchBonsai() {
    const serial = serialOrThrow();
    await withBusy('Launch Bonsai app', () => invoke('android_mobile_launch_bonsai', { serial }));
  }

  async function sendKey(keyCode: number, label: string) {
    const serial = serialOrThrow();
    await withBusy(`Send key ${label}`, () => invoke('android_mobile_send_key', { serial, keyCode }));
  }

  async function sendText() {
    const serial = serialOrThrow();
    const text = textInput.trim();
    if (!text) {
      error = 'Text input cannot be empty.';
      return;
    }
    await withBusy('Send text', () => invoke('android_mobile_send_text', { serial, text }));
  }

  async function openNotificationsShade() {
    const serial = serialOrThrow();
    await withBusy('Open notifications shade', () =>
      invoke('android_mobile_swipe', {
        serial,
        x1: 540,
        y1: 32,
        x2: 540,
        y2: 1100,
        durationMs: 260,
      }),
    );
  }

  function mapToDevicePoint(clientX: number, clientY: number, element: HTMLElement) {
    const rect = element.getBoundingClientRect();
    const rawX = Math.min(Math.max(clientX - rect.left, 0), rect.width);
    const rawY = Math.min(Math.max(clientY - rect.top, 0), rect.height);

    const width = Math.max(displayInfo?.width ?? 1080, 1);
    const height = Math.max(displayInfo?.height ?? 2400, 1);

    return {
      x: Math.round((rawX / rect.width) * width),
      y: Math.round((rawY / rect.height) * height),
    };
  }

  async function handleScreenTap(event: PointerEvent) {
    const serial = serialOrThrow();
    const target = event.currentTarget as HTMLElement;
    const point = mapToDevicePoint(event.clientX, event.clientY, target);
    await withBusy('Tap device screen', () =>
      invoke('android_mobile_tap', {
        serial,
        x: point.x,
        y: point.y,
      }),
    );
  }

  function onScreenPointerDown(event: PointerEvent) {
    pointerStart = { x: event.clientX, y: event.clientY };
  }

  async function onScreenPointerUp(event: PointerEvent) {
    const serial = serialOrThrow();
    const target = event.currentTarget as HTMLElement;
    const start = pointerStart;
    pointerStart = null;

    if (!start) {
      await handleScreenTap(event);
      return;
    }

    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;
    const distance = Math.hypot(dx, dy);

    if (distance < 12) {
      await handleScreenTap(event);
      return;
    }

    const from = mapToDevicePoint(start.x, start.y, target);
    const to = mapToDevicePoint(event.clientX, event.clientY, target);

    await withBusy('Swipe on device screen', () =>
      invoke('android_mobile_swipe', {
        serial,
        x1: from.x,
        y1: from.y,
        x2: to.x,
        y2: to.y,
        durationMs: 220,
      }),
    );
  }

  onMount(async () => {
    try {
      await refreshDevices();
      if (selectedSerial) {
        await refreshDisplayInfo();
      }
    } catch {
      // withBusy records details for UI.
    }
  });

  onDestroy(() => {
    stopFramePolling();
    clearFrameObjectUrl();
  });
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="mobile-viewer-overlay" role="presentation" on:click|self={() => dispatch('close')}>
  <section class="mobile-viewer-window" role="dialog" aria-modal="true" aria-label="Mobile Viewer">
    <header class="viewer-header">
      <div class="viewer-title-wrap">
        <h2>Mobile Viewer</h2>
        <p>Production-grade device mirror and control surface</p>
      </div>
      <div class="viewer-header-actions">
        <button class="ghost" type="button" on:click={refreshDevices} disabled={busy}>{busy ? 'Working...' : 'Refresh'}</button>
        <button class="ghost" type="button" on:click={() => (settingsOpen = true)}>Settings</button>
        <button class="ghost danger" type="button" on:click={() => dispatch('close')} aria-label="Close Mobile Viewer">Close</button>
      </div>
    </header>

    <div class="viewer-layout">
      <div class="screen-column">
        <div class="top-strip">
          <label>
            Device
            <select bind:value={selectedSerial} disabled={busy}>
              <option value="">Select device</option>
              {#each devices as d}
                <option value={d.serial}>{d.serial} ({d.state}{d.model ? ` - ${d.model}` : ''})</option>
              {/each}
            </select>
          </label>
          <div class="runtime-state">
            <span class:ok={status?.scrcpy_available}>{status?.scrcpy_available ? 'scrcpy ready' : 'scrcpy missing'}</span>
            <span class:ok={!!activeViewFor(selectedSerial)}>{activeViewFor(selectedSerial) ? 'live view active' : 'live view idle'}</span>
            <span class:ok={frameConnected}>{frameConnected ? `remote frame ${lastFrameAt}` : 'remote frame offline'}</span>
          </div>
          {#if status && !status.scrcpy_available}
            <div class="scrcpy-help">
              <small>scrcpy not available on this host. Candidate locations checked:</small>
              {#if status.scrcpy_candidates && status.scrcpy_candidates.length}
                <ul class="scrcpy-candidates">
                  {#each status.scrcpy_candidates as c}
                    <li><code>{c}</code></li>
                  {/each}
                </ul>
              {/if}
              <small>See <a href="https://github.com/Genymobile/scrcpy" target="_blank" rel="noopener">scrcpy docs</a> to install.</small>
            </div>
          {/if}
        </div>

        <div class="screen-shell">
          <div
            class="screen-viewport"
            role="button"
            tabindex="0"
            aria-label="Interactive device screen"
            on:pointerdown={onScreenPointerDown}
            on:pointerup={onScreenPointerUp}
          >
            {#if frameObjectUrl}
              <img class="device-frame" src={frameObjectUrl} alt="Live Android device screen" draggable="false" />
            {:else}
              <div class="screen-empty">
                <p>No live frame yet</p>
                <small>Click Start Viewer to launch scrcpy + remote runtime.</small>
              </div>
            {/if}
          </div>

          <div class="viewer-control-bar">
            <button class="control" type="button" on:click={startViewerStack} disabled={busy || !selectedSerial}>Start Viewer</button>
            <button class="control" type="button" on:click={stopViewerStack} disabled={busy || !selectedSerial}>Stop Viewer</button>
            <button class="control" type="button" on:click={takeScreenshot} disabled={busy || !selectedSerial}>Screenshot</button>
            <button class="control" type="button" on:click={startRecording} disabled={busy || !selectedSerial}>Record</button>
            <button class="control" type="button" on:click={stopRecording} disabled={busy || !selectedSerial}>Stop Rec</button>
            <button class="control" type="button" on:click={refreshDisplayInfo} disabled={busy || !selectedSerial}>Display</button>
            <button class="control" type="button" on:click={() => sendKey(3, 'HOME')} disabled={busy || !selectedSerial}>Home</button>
            <button class="control" type="button" on:click={() => sendKey(4, 'BACK')} disabled={busy || !selectedSerial}>Back</button>
            <button class="control" type="button" on:click={() => sendKey(187, 'RECENTS')} disabled={busy || !selectedSerial}>Recents</button>
            <button class="control" type="button" on:click={openNotificationsShade} disabled={busy || !selectedSerial}>Shade</button>
            <button class="control" type="button" on:click={() => (settingsOpen = true)}>Settings</button>
          </div>
        </div>
      </div>

      <aside class="ops-column">
        <div class="ops-card">
          <h3>Quick Ops</h3>
          <div class="ops-buttons">
            <button class="control" type="button" on:click={launchBonsai} disabled={busy || !selectedSerial}>Launch Bonsai</button>
            <button class="control" type="button" on:click={() => launchCamera(false)} disabled={busy || !selectedSerial}>Camera Photo</button>
            <button class="control" type="button" on:click={() => launchCamera(true)} disabled={busy || !selectedSerial}>Camera Video</button>
            <button class="control" type="button" on:click={() => sendKey(26, 'POWER')} disabled={busy || !selectedSerial}>Power</button>
            <button class="control" type="button" on:click={() => sendKey(24, 'VOL_UP')} disabled={busy || !selectedSerial}>Vol +</button>
            <button class="control" type="button" on:click={() => sendKey(25, 'VOL_DOWN')} disabled={busy || !selectedSerial}>Vol -</button>
          </div>
        </div>

        <div class="ops-card">
          <h3>Text Input</h3>
          <div class="text-row">
            <input type="text" bind:value={textInput} placeholder="Type and send to device" />
            <button class="control" type="button" on:click={sendText} disabled={busy || !selectedSerial}>Send</button>
          </div>
          {#if displayInfo}
            <p class="meta">
              {displayInfo.width || '?'}x{displayInfo.height || '?'} px,
              {displayInfo.density_dpi || '?'} dpi,
              rot {displayInfo.surface_orientation ?? '?'}
            </p>
          {/if}
          <p class="meta">ADB: {status?.adb_executable || 'unknown'}</p>
          <p class="meta">scrcpy: {status?.scrcpy_executable || 'unknown'}</p>
          {#if remoteSessionId}
            <p class="meta">Session: {remoteSessionId}</p>
          {/if}
          {#if inputUrl}
            <p class="meta">Input endpoint ready</p>
          {/if}
        </div>

        <div class="ops-card log-card">
          <h3>Activity Log</h3>
          {#if error}
            <div class="error">{error}</div>
          {/if}
          {#if logText}
            <pre class="log-box">{logText}</pre>
          {:else}
            <p class="meta">No actions yet.</p>
          {/if}
        </div>
      </aside>
    </div>

    {#if settingsOpen}
      <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
      <div class="settings-overlay" role="presentation" on:click|self={() => (settingsOpen = false)}>
        <section class="settings-popup" role="dialog" aria-modal="true" aria-label="Mobile Viewer settings">
          <header>
            <h3>Mobile Viewer Settings</h3>
            <button class="ghost" type="button" on:click={() => (settingsOpen = false)}>Done</button>
          </header>

          <div class="settings-grid">
            <label>Max stream size
              <input type="number" min="240" bind:value={maxSize} />
            </label>
            <label>Video bitrate (Mbps)
              <input type="number" min="1" max="80" bind:value={bitrateMbps} />
            </label>
            <label>Recording bitrate (Mbps)
              <input type="number" min="1" max="80" bind:value={recordingBitrate} />
            </label>
            <label>API port
              <input type="number" min="1" max="65535" bind:value={runtimeApiPort} />
            </label>
            <label>WS port
              <input type="number" min="1" max="65535" bind:value={runtimeWsPort} />
            </label>
            <label>Frame polling ms
              <input
                type="number"
                min="120"
                max="2000"
                bind:value={framePollMs}
                on:change={() => {
                  if (frameUrl) startFramePolling();
                }}
              />
            </label>
          </div>

          <div class="switch-row">
            <label class="check"><input type="checkbox" bind:checked={fullscreen} /> Fullscreen scrcpy window</label>
            <label class="check"><input type="checkbox" bind:checked={stayAwake} /> Keep device awake</label>
            <label class="check"><input type="checkbox" bind:checked={turnScreenOff} /> Turn device screen off</label>
          </div>

          <div class="settings-actions">
            <button class="control" type="button" on:click={prepareViewerRuntime} disabled={busy || !selectedSerial}>Prepare Runtime</button>
            <button class="control" type="button" on:click={refreshDisplayInfo} disabled={busy || !selectedSerial}>Refresh Display</button>
          </div>
        </section>
      </div>
    {/if}
  </section>
</div>

<style>
  .mobile-viewer-overlay {
    position: fixed;
    inset: 0;
    z-index: 420;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 18px;
    background:
      radial-gradient(circle at 14% 10%, rgba(13, 94, 116, 0.35) 0, rgba(13, 94, 116, 0) 46%),
      radial-gradient(circle at 80% 85%, rgba(14, 52, 95, 0.33) 0, rgba(14, 52, 95, 0) 55%),
      rgba(6, 12, 20, 0.72);
    backdrop-filter: blur(3px);
  }

  .mobile-viewer-window {
    width: min(1320px, 97vw);
    height: min(820px, 93vh);
    border: 1px solid #224765;
    border-radius: 16px;
    background: linear-gradient(165deg, #081826 0%, #0b1520 56%, #0f1c2d 100%);
    color: #d6ebff;
    box-shadow: 0 28px 80px rgba(0, 0, 0, 0.55);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .viewer-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
  }

  .viewer-title-wrap h2 {
    margin: 0;
    font-size: 1.28rem;
    letter-spacing: 0.02em;
  }

  .viewer-title-wrap p {
    margin: 4px 0 0;
    color: #93bddf;
    font-size: 0.9rem;
  }

  .viewer-header-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .viewer-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 330px;
    gap: 12px;
    min-height: 0;
    flex: 1;
  }

  .screen-column {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 10px;
    min-height: 0;
  }

  .top-strip {
    border: 1px solid #27506f;
    border-radius: 12px;
    background: rgba(12, 33, 47, 0.9);
    padding: 10px;
    display: grid;
    grid-template-columns: minmax(220px, 360px) minmax(0, 1fr);
    gap: 10px;
    align-items: end;
  }

  .runtime-state {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  .runtime-state span {
    border: 1px solid #466680;
    border-radius: 999px;
    padding: 4px 10px;
    background: rgba(11, 25, 38, 0.8);
    color: #aac9df;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .runtime-state span.ok {
    border-color: #2f9162;
    color: #8de0b2;
  }

  .scrcpy-candidates {
    margin: 6px 0 8px;
    max-height: 96px;
    overflow: auto;
    font-size: 0.85rem;
    color: #9db8cb;
  }
  .scrcpy-candidates li { list-style: none; padding: 2px 0; }
  .scrcpy-candidates code { font-family: 'Cascadia Code', monospace; font-size: 0.82rem; background: rgba(0,0,0,0.06); padding: 2px 6px; border-radius: 6px; }

  .screen-shell {
    border: 1px solid #2a4f6b;
    border-radius: 14px;
    background:
      linear-gradient(180deg, rgba(10, 25, 38, 0.95), rgba(8, 18, 29, 0.96)),
      repeating-linear-gradient(
        45deg,
        rgba(255, 255, 255, 0.012) 0,
        rgba(255, 255, 255, 0.012) 8px,
        rgba(255, 255, 255, 0) 8px,
        rgba(255, 255, 255, 0) 16px
      );
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    min-height: 0;
    overflow: hidden;
  }

  .screen-viewport {
    margin: 16px auto 10px;
    width: min(420px, calc(100% - 28px));
    aspect-ratio: 9 / 19.5;
    border-radius: 26px;
    border: 2px solid #5d7f98;
    background: #030910;
    box-shadow:
      inset 0 0 0 5px #0f2d44,
      inset 0 0 40px rgba(33, 89, 124, 0.35),
      0 18px 40px rgba(0, 0, 0, 0.5);
    cursor: crosshair;
    overflow: hidden;
    position: relative;
  }

  .device-frame {
    width: 100%;
    height: 100%;
    object-fit: cover;
    user-select: none;
    pointer-events: none;
  }

  .screen-empty {
    height: 100%;
    display: grid;
    place-items: center;
    text-align: center;
    color: #9db8cb;
    padding: 18px;
  }

  .screen-empty p {
    margin: 0;
    font-weight: 600;
    color: #c2d6e5;
  }

  .screen-empty small {
    margin-top: 6px;
    display: block;
    font-size: 0.8rem;
  }

  .viewer-control-bar {
    border-top: 1px solid #2a4f6b;
    background: rgba(6, 18, 28, 0.86);
    padding: 10px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .ops-column {
    min-height: 0;
    display: grid;
    gap: 10px;
    grid-template-rows: auto auto minmax(0, 1fr);
  }

  .ops-card {
    border: 1px solid #2a4f6b;
    border-radius: 12px;
    background: rgba(11, 27, 40, 0.86);
    padding: 10px;
    display: grid;
    gap: 8px;
  }

  .ops-card h3 {
    margin: 0;
    font-size: 0.9rem;
    letter-spacing: 0.02em;
    color: #b7d8ee;
  }

  .ops-buttons {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .text-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }

  .meta {
    margin: 0;
    font-size: 0.78rem;
    color: #91b2cb;
  }

  .log-card {
    min-height: 0;
  }

  .log-box {
    margin: 0;
    border: 1px solid #2e4a61;
    border-radius: 8px;
    background: #07121d;
    color: #95b3cc;
    padding: 8px;
    min-height: 110px;
    max-height: 100%;
    overflow: auto;
    font-size: 0.77rem;
    line-height: 1.35;
    white-space: pre-wrap;
  }

  .settings-overlay {
    position: fixed;
    inset: 0;
    z-index: 430;
    background: rgba(4, 10, 16, 0.58);
    display: grid;
    place-items: center;
    padding: 20px;
  }

  .settings-popup {
    width: min(760px, 94vw);
    border: 1px solid #2c5976;
    border-radius: 14px;
    background: linear-gradient(180deg, #0b1927 0%, #0f2133 100%);
    padding: 14px;
    color: #d4e9fb;
    box-shadow: 0 26px 68px rgba(0, 0, 0, 0.52);
    display: grid;
    gap: 12px;
  }

  .settings-popup header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .settings-popup h3 {
    margin: 0;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .switch-row {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .settings-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 0.84rem;
    color: #b3d2e9;
  }

  input,
  select {
    border: 1px solid #3a6280;
    background: #102436;
    color: #d5e8f9;
    border-radius: 8px;
    min-height: 36px;
    padding: 7px 10px;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: #b9d7ec;
  }

  .control,
  .ghost {
    border: 1px solid #3a617d;
    border-radius: 8px;
    min-height: 34px;
    padding: 7px 10px;
    color: #d5e8f9;
    background: linear-gradient(180deg, #16324a, #12283c);
    cursor: pointer;
    transition: background 120ms ease, transform 120ms ease;
    font-size: 0.82rem;
  }

  .ghost {
    background: #12293d;
  }

  .ghost.danger {
    border-color: #86525b;
    background: #3b2028;
    color: #ffd8de;
  }

  .control:hover:not(:disabled),
  .ghost:hover:not(:disabled) {
    background: linear-gradient(180deg, #1a3c59, #153248);
    transform: translateY(-1px);
  }

  .ghost.danger:hover:not(:disabled) {
    background: #4b2730;
  }

  .control:disabled,
  .ghost:disabled {
    opacity: 0.58;
    cursor: default;
    transform: none;
  }

  .error {
    border: 1px solid #814656;
    border-radius: 8px;
    padding: 8px;
    background: rgba(73, 24, 36, 0.75);
    color: #ffd5de;
    font-size: 0.8rem;
  }

  @media (max-width: 1180px) {
    .viewer-layout {
      grid-template-columns: 1fr;
    }

    .ops-column {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      grid-template-rows: auto;
    }

    .log-card {
      grid-column: span 3;
      min-height: 170px;
    }
  }

  @media (max-width: 780px) {
    .mobile-viewer-window {
      height: 95vh;
      padding: 12px;
    }

    .top-strip {
      grid-template-columns: 1fr;
    }

    .settings-grid {
      grid-template-columns: 1fr;
    }

    .ops-column {
      grid-template-columns: 1fr;
    }

    .log-card {
      grid-column: span 1;
    }
  }
</style>
