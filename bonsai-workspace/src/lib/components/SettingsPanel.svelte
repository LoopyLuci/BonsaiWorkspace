<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { addAssistantMessage } from '$lib/stores/chat';
  import { availableModels, activeModel, activeModelId, orchestratorStatus, refreshStatus, refreshModels, modelSwitchStatus } from '$lib/stores/models';
  import { apiHost, apiPort, apiBaseUrl, loadApiSettings, saveApiSettings } from '$lib/stores/settings';
  import {
    BONSAI_CATALOG, findRegistryModel,
    downloadCatalogModel, downloadingId, downloadPct, downloadError,
  } from '$lib/stores/catalog';

  const dispatch = createEventDispatcher<{ close: void }>();

  let hwInfo:          Record<string, unknown> = {};
  let loadingOp        = '';
  let errorMsg         = '';
  let switchDetails    = '';
  let apiTestResult    = '';
  let apiTestLoading   = false;
  let saveApiLoading   = false;

  let remoteSessionId  = '';
  let remoteState      = 'inactive';
  let remoteStatus     = '';
  let remoteStreamUrl  = '';
  let remoteFrameUrl   = '';
  let remoteInputUrl   = '';
  let remoteLoading    = false;
  let remotePreviewSrc = '';
  let remotePreviewErr = '';
  let remoteInputResult = '';
  let remoteEventSource: EventSource | null = null;

  onMount(async () => {
    await refreshModels();
    await refreshStatus();
    modelSwitchStatus.set('');
    try { hwInfo = await invoke<Record<string,unknown>>('get_hardware_info'); } catch {}
    try { await loadApiSettings(); } catch (e) { console.warn('Failed to load API settings', e); }
  });

  async function switchModel(modelId: string, name: string) {
    activeModelId.set(modelId);
    loadingOp = `Switching to ${name}…`;
    switchDetails = `Switching to ${name}…`;
    modelSwitchStatus.set(switchDetails);
    errorMsg = '';
    try {
      const msg = await invoke<string>('switch_model', { modelId });
      await refreshModels();
      await refreshStatus();
      switchDetails = `${msg} Orchestrator refreshed.`;
      modelSwitchStatus.set(switchDetails);
      addAssistantMessage(msg);
    } catch (e) {
      errorMsg = String(e);
      switchDetails = `Model switch failed: ${errorMsg}`;
      modelSwitchStatus.set(switchDetails);
    } finally { loadingOp = ''; }
  }

  async function copyApiEndpoint() {
    try {
      await navigator.clipboard.writeText($apiBaseUrl);
      apiTestResult = `Copied API endpoint: ${$apiBaseUrl}`;
    } catch {
      apiTestResult = 'Unable to copy API endpoint. Please copy manually.';
    }
  }

  async function testApiEndpoint() {
    apiTestLoading = true;
    apiTestResult = '';
    try {
      const resp = await fetch(`${$apiBaseUrl}/v1/models`);
      const json = await resp.json();
      if (resp.ok) {
        apiTestResult = `API reachable: ${json.data?.length ?? 'unknown'} model(s) available.`;
      } else {
        apiTestResult = `API error: ${json.error?.message ?? resp.statusText}`;
      }
    } catch (e) {
      apiTestResult = `API test failed: ${String(e)}`;
    } finally {
      apiTestLoading = false;
    }
  }

  async function applyApiSettings() {
    saveApiLoading = true;
    apiTestResult = '';
    try {
      const config = await saveApiSettings($apiHost, $apiPort);
      apiTestResult = `API settings saved: ${config.api_host}:${config.api_port}`;
    } catch (e) {
      apiTestResult = `Save failed: ${String(e)}`;
    } finally {
      saveApiLoading = false;
    }
  }

  function disconnectRemotePreview() {
    remotePreviewSrc = '';
    remotePreviewErr = '';
    if (remoteEventSource) {
      remoteEventSource.close();
      remoteEventSource = null;
    }
  }

  function connectRemotePreview(url: string) {
    disconnectRemotePreview();
    try {
      remoteEventSource = new EventSource(url);
      remoteEventSource.onmessage = event => {
        try {
          const data = JSON.parse(event.data);
          if (data.frame) {
            remotePreviewSrc = `data:image/png;base64,${data.frame}`;
            remotePreviewErr = '';
          }
        } catch (err) {
          remotePreviewErr = `Preview parse error: ${String(err)}`;
        }
      };
      remoteEventSource.onerror = () => {
        remotePreviewErr = 'Remote preview connection lost.';
      };
    } catch (err) {
      remotePreviewErr = `Failed to connect preview: ${String(err)}`;
    }
  }

  async function startRemoteSession() {
    remoteLoading = true;
    remoteStatus = 'Starting remote session…';
    try {
      const result = await invoke<{
        session_id: string;
        state: string;
        stream_url: string;
        frame_url: string;
        input_url: string;
      }>('start_remote_session');
      remoteSessionId = result.session_id;
      remoteState = result.state;
      remoteStreamUrl = result.stream_url;
      remoteFrameUrl = result.frame_url;
      remoteInputUrl = result.input_url;
      remoteStatus = `Remote session started. Stream URL is available.`;
      connectRemotePreview(remoteStreamUrl);
    } catch (e) {
      remoteStatus = `Failed to start remote session: ${String(e)}`;
    } finally {
      remoteLoading = false;
    }
  }

  async function stopRemoteSession() {
    remoteLoading = true;
    remoteStatus = 'Stopping remote session…';
    try {
      await invoke('stop_remote_session');
      remoteSessionId = '';
      remoteState = 'inactive';
      remoteStreamUrl = '';
      remoteFrameUrl = '';
      remoteInputUrl = '';
      remoteStatus = 'Remote session stopped.';
      disconnectRemotePreview();
    } catch (e) {
      remoteStatus = `Failed to stop remote session: ${String(e)}`;
    } finally {
      remoteLoading = false;
    }
  }

  async function copyRemoteStreamUrl() {
    if (!remoteStreamUrl) return;
    try {
      await navigator.clipboard.writeText(remoteStreamUrl);
      remoteStatus = 'Remote stream URL copied to clipboard.';
    } catch {
      remoteStatus = 'Unable to copy remote stream URL. Copy manually.';
    }
  }

  async function sendRemoteInputTest(eventType: string) {
    if (!remoteSessionId) {
      remoteInputResult = 'Start a session before sending remote input.';
      return;
    }
    remoteInputResult = 'Sending test input…';
    const payload = {
      event_type: eventType,
      x: 100,
      y: 100,
      button: 'left',
      key: eventType === 'key' ? 'Enter' : undefined,
      modifiers: eventType === 'key' ? ['control'] : undefined,
    };

    try {
      const result = await invoke<{ status: string }>('send_remote_input', { event: payload });
      remoteInputResult = `Remote input accepted: ${result.status}`;
    } catch (e) {
      remoteInputResult = `Remote input failed: ${String(e)}`;
    }
  }

  onDestroy(() => {
    disconnectRemotePreview();
  });

  async function downloadWhisper() {
    loadingOp = 'Downloading Whisper model…';
    try {
      const path = await invoke<string>('download_whisper_model');
      addAssistantMessage(`✅ Whisper model saved to \`${path}\``);
    } catch (e) { errorMsg = String(e); }
    finally { loadingOp = ''; }
  }

  async function importGguf() {
    try {
      const path = await invoke<string>('prompt_gguf_import');
      if (path) { addAssistantMessage(`📦 Model imported from \`${path}\``); await refreshModels(); }
    } catch (e) { errorMsg = String(e); }
  }

  async function handleDownload(entry: typeof BONSAI_CATALOG[number]) {
    await downloadCatalogModel(entry);
    if ($downloadError) errorMsg = $downloadError;
  }

  // ── Connection / pairing ──────────────────────────────────────────────────
  let pairToken    = '';
  let pairQrSvg   = '';
  let localIp     = '';
  let wsClientCount = 0;
  let pairLoading = false;
  let pairError   = '';

  async function loadPairInfo() {
    pairLoading = true;
    pairError   = '';
    try {
      [pairToken, localIp, pairQrSvg] = await Promise.all([
        invoke<string>('get_pair_token'),
        invoke<string>('get_local_ip'),
        invoke<string>('generate_pair_qr'),
      ]);
      wsClientCount = await invoke<number>('ws_client_count');
    } catch (e) {
      pairError = String(e);
    } finally {
      pairLoading = false;
    }
  }

  async function refreshWsCount() {
    try { wsClientCount = await invoke<number>('ws_client_count'); } catch {}
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="settings-overlay" on:click|self={() => dispatch('close')} role="presentation">
  <div class="settings-panel" role="dialog" aria-modal="true" aria-label="Settings">
    <header class="settings-header">
      <h2>Settings</h2>
      <button class="close-btn" on:click={() => dispatch('close')} aria-label="Close settings">✕</button>
    </header>

    {#if errorMsg}
      <div class="error-bar" role="alert">
        {errorMsg}
        <button on:click={() => (errorMsg = '')}>✕</button>
      </div>
    {/if}

    {#if loadingOp}
      <div class="loading-bar">{loadingOp}</div>
    {/if}

    <!-- Hardware info -->
    {#if Object.keys(hwInfo).length}
      <section class="section">
        <h3 class="section-title">Hardware</h3>
        <div class="hw-grid">
          <div class="hw-item"><span class="hw-label">RAM</span><span class="hw-val">{hwInfo.ram_total_gb} GB</span></div>
          <div class="hw-item"><span class="hw-label">Available</span><span class="hw-val">{hwInfo.ram_available_gb} GB</span></div>
          <div class="hw-item"><span class="hw-label">CPUs</span><span class="hw-val">{hwInfo.cpu_count}</span></div>
          <div class="hw-item"><span class="hw-label">Backend</span><span class="hw-val">{hwInfo.backend}</span></div>
          {#if Array.isArray(hwInfo.gpu_names) && hwInfo.gpu_names.length > 0}
            <div class="hw-item hw-item--wide"><span class="hw-label">GPU(s)</span><span class="hw-val hw-val--sm">{hwInfo.gpu_names.join(', ')}</span></div>
          {/if}
        </div>
      </section>
    {/if}

    {#if $orchestratorStatus}
      <section class="section status-summary">
        <h3 class="section-title">Orchestrator Status</h3>
        <div class="status-grid">
          <div class="status-item"><span class="status-label">Slots</span><span class="status-value">{$orchestratorStatus.slots.length}</span></div>
          <div class="status-item"><span class="status-label">Queue</span><span class="status-value">{$orchestratorStatus.queue_depth}</span></div>
          <div class="status-item"><span class="status-label">RAM Free</span><span class="status-value">{Math.round($orchestratorStatus.free_ram_mb / 1024)} GB</span></div>
          <div class="status-item"><span class="status-label">Total RAM</span><span class="status-value">{Math.round($orchestratorStatus.total_ram_mb / 1024)} GB</span></div>
        </div>
        <div class="slot-list">
          {#each $orchestratorStatus.slots as slot}
            <div class="slot-row">
              <span class="slot-label">Slot {slot.index}</span>
              <span class="slot-state">{slot.state.state}{slot.state.model_id ? ` (${slot.state.model_id})` : ''}</span>
              <span class="slot-info">{slot.requests} req · {slot.idle_secs}s idle</span>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    <section class="section api-settings">
      <h3 class="section-title">API Settings</h3>
      <div class="form-group">
        <label for="api-host">API Host</label>
        <input id="api-host" type="text" bind:value={$apiHost} />
      </div>
      <div class="form-group">
        <label for="api-port">API Port</label>
        <input id="api-port" type="number" min="1" max="65535" bind:value={$apiPort} />
      </div>
      <div class="form-note">
        External agents can connect to the OpenAI-compatible endpoint shown here.
      </div>
      <div class="action-grid">
        <button class="action-btn blue" type="button" on:click={copyApiEndpoint} disabled={apiTestLoading || saveApiLoading || remoteLoading}>
          📋 Copy endpoint
        </button>
        <button class="action-btn green" type="button" on:click={testApiEndpoint} disabled={apiTestLoading || saveApiLoading || remoteLoading}>
          {apiTestLoading ? 'Testing…' : 'Test API'}
        </button>
        <button class="action-btn blue" type="button" on:click={applyApiSettings} disabled={apiTestLoading || saveApiLoading || remoteLoading}>
          {saveApiLoading ? 'Saving…' : 'Save API settings'}
        </button>
      </div>
      {#if apiTestResult}
        <div class="api-test-result">{apiTestResult}</div>
      {/if}
      <div class="api-endpoint">Current endpoint: <code>{$apiBaseUrl}</code></div>
    </section>

    <section class="section remote-control">
      <h3 class="section-title">Remote Control</h3>
      <div class="form-note">
        Start a local remote session to stream screen frames and negotiate input.
      </div>
      <div class="action-grid">
        <button class="action-btn green" type="button" on:click={startRemoteSession} disabled={remoteLoading || apiTestLoading || saveApiLoading}>
          {remoteLoading ? 'Starting…' : 'Start Remote Session'}
        </button>
        <button class="action-btn red" type="button" on:click={stopRemoteSession} disabled={remoteLoading || !remoteSessionId}>
          {remoteLoading ? 'Stopping…' : 'Stop Remote Session'}
        </button>
      </div>
      {#if remoteStatus}
        <div class="api-test-result">{remoteStatus}</div>
      {/if}
      {#if remoteSessionId}
        <div class="remote-info">
          <div><strong>Session ID:</strong> <code>{remoteSessionId}</code></div>
          <div><strong>Stream URL:</strong> <code>{remoteStreamUrl}</code> <button class="copy-link" type="button" on:click={copyRemoteStreamUrl}>Copy</button></div>
          <div><strong>Frame URL:</strong> <code>{remoteFrameUrl}</code></div>
          <div><strong>Input URL:</strong> <code>{remoteInputUrl}</code></div>
        </div>
        <div class="remote-action-grid">
          <button class="action-btn blue" type="button" on:click={() => sendRemoteInputTest('click')} disabled={!remoteSessionId || remoteLoading}>
            Send Test Click
          </button>
          <button class="action-btn blue" type="button" on:click={() => sendRemoteInputTest('key')} disabled={!remoteSessionId || remoteLoading}>
            Send Test Key
          </button>
        </div>
        {#if remoteInputResult}
          <div class="api-test-result">{remoteInputResult}</div>
        {/if}
        <div class="remote-preview">
          {#if remotePreviewErr}
            <div class="remote-error">{remotePreviewErr}</div>
          {/if}
          {#if remotePreviewSrc}
            <img class="remote-preview-img" src={remotePreviewSrc} alt="Remote preview" />
          {:else}
            <div class="remote-preview-placeholder">Waiting for first frame…</div>
          {/if}
        </div>
      {/if}
    </section>

    {#if switchDetails}
      <section class="section switch-details">
        <h3 class="section-title">Switch details</h3>
        <div class="switch-log">{switchDetails}</div>
      </section>
    {/if}

    <!-- Models -->
    <section class="section">
      <h3 class="section-title">Language Models</h3>
      <div class="model-list">
        {#each BONSAI_CATALOG as entry (entry.catalogId)}
          {@const reg = findRegistryModel(entry, $availableModels)}
          {@const isActive = !!reg && $activeModel?.id === reg.id}
          {@const isDling = $downloadingId === entry.catalogId}
          <div class="model-row" class:active-model={isActive}>
            <div class="model-info">
              <div class="model-name">
                {entry.name}
                {#if entry.isDefault}<span class="badge-default">default</span>{/if}
              </div>
              <div class="model-meta">
                {entry.quant} · ~{entry.ramGb} GB RAM
                {#if !reg}<span class="badge-notlocal">not downloaded</span>{/if}
              </div>
            </div>
            <div class="model-actions">
              {#if isActive}
                <span class="badge-active">Active</span>
              {:else if reg && !isDling}
                <button class="btn-sm" on:click={() => switchModel(reg.id, entry.name)} disabled={!!loadingOp}>
                  Use
                </button>
              {:else if isDling}
                <span class="badge-active" style="background: var(--accent)">{$downloadPct}%</span>
              {:else}
                <button class="btn-sm btn-dl" on:click={() => handleDownload(entry)} disabled={!!loadingOp}>
                  ⬇ Download
                </button>
              {/if}
            </div>
          </div>
        {/each}

        <!-- Any extra local models not in the catalog -->
        {#each $availableModels.filter(m => !BONSAI_CATALOG.some(e => findRegistryModel(e, $availableModels)?.id === m.id)) as model (model.id)}
          <div class="model-row" class:active-model={model.id === $activeModel?.id}>
            <div class="model-info">
              <div class="model-name">{model.name}</div>
              <div class="model-meta">{model.quant} · ~{Math.round(model.ram_required_mb / 1024 * 10) / 10} GB RAM</div>
            </div>
            <div class="model-actions">
              {#if model.id === $activeModel?.id}
                <span class="badge-active">Active</span>
              {:else}
                <button class="btn-sm" on:click={() => switchModel(model.id, model.name)} disabled={!!loadingOp}>Use</button>
              {/if}
            </div>
          </div>
        {/each}
      </div>

      <div class="action-grid">
        <button class="action-btn blue" on:click={importGguf} disabled={!!loadingOp}>
          📂 Import Local GGUF
        </button>
        <button class="action-btn green" on:click={downloadWhisper} disabled={!!loadingOp}>
          ⬇ Download Whisper
        </button>
      </div>
    </section>

    <!-- ── Connection / Pairing ──────────────────────────────────────────── -->
    <section class="section connection-section">
      <h3 class="section-title">Mobile & VSCode Connection</h3>
      <p class="section-desc">
        Scan the QR code with the Bonsai Android app, or paste the token into
        the <strong>Bonsai Workspace Runner</strong> VSCode extension settings.
      </p>
      <div class="pair-row">
        <div class="qr-area">
          {#if pairQrSvg}
            {@html pairQrSvg}
          {:else}
            <button class="action-btn" on:click={loadPairInfo} disabled={pairLoading}>
              {pairLoading ? 'Loading…' : 'Show QR Code'}
            </button>
          {/if}
        </div>
        <div class="pair-info">
          {#if pairToken}
            <div class="pair-field">
              <span class="pair-label">Pair token</span>
              <code class="pair-token">{pairToken}</code>
            </div>
            <div class="pair-field">
              <span class="pair-label">LAN IP</span>
              <code class="pair-token">{localIp || '…'}</code>
            </div>
            <div class="pair-field">
              <span class="pair-label">WebSocket</span>
              <code class="pair-token">ws://{localIp || '…'}:11369/ws</code>
            </div>
            <div class="pair-field">
              <span class="pair-label">WS clients</span>
              <code class="pair-token">{wsClientCount}</code>
            </div>
            <button class="action-btn" on:click={refreshWsCount}>↺ Refresh</button>
          {/if}
          {#if pairError}
            <div class="pair-error">{pairError}</div>
          {/if}
        </div>
      </div>
    </section>

  </div>
</div>

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    z-index: 400;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .settings-panel {
    width: 520px;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0,0,0,0.5);
    display: flex;
    flex-direction: column;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .settings-header h2 { font-size: 15px; font-weight: 600; }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 16px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg-hover); color: var(--text); }

  .section { padding: 16px 20px; border-bottom: 1px solid var(--border); }
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
    margin-bottom: 10px;
  }

  .hw-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .hw-item--wide { grid-column: 1 / -1; }
  .hw-val--sm { font-size: 11px; font-weight: 400; color: var(--text-dim); }
  .hw-item {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .status-summary .status-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .status-item {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    font-size: 12px;
  }
  .status-label { color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.05em; font-size: 10px; }
  .status-value { font-weight: 600; }
  .slot-list {
    margin-top: 12px;
    display: grid;
    gap: 6px;
  }
  .slot-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    font-size: 12px;
  }
  .slot-state { font-weight: 600; color: var(--text); }
  .slot-info { color: var(--text-dim); }
  .switch-details .switch-log {
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    font-size: 12px;
    white-space: pre-wrap;
  }
  .hw-label { font-size: 10px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.05em; }
  .hw-val   { font-size: 14px; font-weight: 600; }

  .model-list { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
  .model-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    transition: border-color 0.15s;
  }
  .model-row.active-model { border-color: var(--accent); }

  .remote-info {
    display: grid;
    gap: 8px;
    margin-top: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    font-size: 12px;
  }

  .remote-preview {
    margin-top: 12px;
    background: var(--bg);
    border: 1px dashed var(--border);
    border-radius: 10px;
    padding: 12px;
    display: grid;
    gap: 10px;
  }

  .remote-action-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 12px;
  }

  .remote-preview-placeholder,
  .remote-error {
    color: var(--text-dim);
    font-size: 12px;
  }

  .remote-preview-img {
    width: 100%;
    border-radius: 10px;
    max-height: 320px;
    object-fit: contain;
    background: var(--bg2);
  }

  .copy-link {
    margin-left: 8px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
  }

  .copy-link:hover { background: var(--bg-hover); }

  .model-name { font-size: 13px; font-weight: 500; }
  .model-meta { font-size: 11px; color: var(--text-dim); margin-top: 2px; }

  .badge-default {
    font-size: 9px;
    background: rgba(251,191,36,0.15);
    color: #fbbf24;
    border: 1px solid rgba(251,191,36,0.3);
    border-radius: 4px;
    padding: 1px 5px;
    margin-left: 5px;
    vertical-align: middle;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge-notlocal {
    font-size: 9px;
    background: rgba(239,68,68,0.12);
    color: var(--red);
    border: 1px solid rgba(239,68,68,0.25);
    border-radius: 4px;
    padding: 1px 5px;
    margin-left: 5px;
    vertical-align: middle;
  }

  .btn-dl {
    background: rgba(251,191,36,0.1) !important;
    border-color: rgba(251,191,36,0.4) !important;
    color: #fbbf24 !important;
  }
  .btn-dl:hover { background: rgba(251,191,36,0.2) !important; }

  .badge-active {
    font-size: 11px;
    background: var(--accent);
    color: #fff;
    padding: 2px 8px;
    border-radius: 10px;
  }

  .btn-sm {
    font-size: 12px;
    background: var(--bg2);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 10px;
    border-radius: 5px;
    cursor: pointer;
  }
  .btn-sm:hover { background: var(--bg-hover); }
  .btn-sm:disabled { opacity: 0.5; cursor: not-allowed; }

  .action-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
  .action-btn {
    padding: 8px 12px;
    border: none;
    border-radius: 7px;
    font-size: 12px;
    cursor: pointer;
    transition: opacity 0.15s;
    color: #fff;
  }
  .action-btn.green { background: #16a34a; }
  .action-btn.blue  { background: var(--accent); }
  .action-btn:hover { opacity: 0.85; }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .error-bar {
    background: var(--red);
    color: #fff;
    font-size: 12px;
    padding: 6px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .error-bar button { background: transparent; border: none; color: #fff; cursor: pointer; }

  .loading-bar {
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    padding: 6px 16px;
    animation: pulse 1.2s infinite;
  }

  .api-settings .form-group {
    display: grid;
    gap: 6px;
    margin-bottom: 12px;
  }
  .api-settings label {
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .api-settings input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    background: var(--bg);
    color: var(--text);
  }
  .api-settings .form-note {
    color: var(--text-dim);
    font-size: 11px;
    margin-top: -6px;
    margin-bottom: 12px;
  }
  .api-test-result {
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: rgba(59,130,246,0.08);
    border: 1px solid rgba(59,130,246,0.2);
    color: #fff;
    font-size: 12px;
  }
  .api-endpoint {
    margin-top: 12px;
    color: var(--text-dim);
    font-size: 11px;
    word-break: break-all;
  }

  @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.7} }

  /* ── Connection / pairing ── */
  .connection-section .section-desc {
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 14px;
    line-height: 1.5;
  }

  .pair-row {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .qr-area {
    flex-shrink: 0;
    width: 160px;
    height: 160px;
    background: #fff;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .qr-area :global(svg) {
    width: 100%;
    height: 100%;
  }

  .pair-info {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex: 1;
    min-width: 180px;
  }

  .pair-field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pair-label {
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .pair-token {
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 13px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    word-break: break-all;
  }

  .pair-error {
    color: var(--red);
    font-size: 12px;
  }
</style>
