<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { addAssistantMessage } from '$lib/stores/chat';
  import { activeModel, orchestratorStatus, refreshStatus, modelSwitchStatus } from '$lib/stores/models';

  const dispatch = createEventDispatcher<{ close: void }>();

  interface Model {
    id:              string;
    name:            string;
    quant:           string;
    ram_required_mb: number;
    valid:           boolean;
    download_url?:   string;
    file_name?:      string;
  }

  let models:     Model[] = [];
  let hwInfo:     Record<string, unknown> = {};
  let loadingOp   = '';
  let errorMsg    = '';
  let switchDetails = '';

  onMount(async () => {
    await refresh();
    await refreshStatus();
    modelSwitchStatus.set('');
    try {
      hwInfo = await invoke<Record<string,unknown>>('get_hardware_info');
    } catch {}
  });

  async function refresh() {
    models = await invoke<Model[]>('list_available_models');
  }

  async function switchModel(modelId: string, name: string) {
    loadingOp = `Switching to ${name}…`;
    const progressText = `Switching to ${name} — requesting backend switch and slot allocation.`;
    switchDetails = progressText;
    modelSwitchStatus.set(progressText);
    errorMsg = '';
    try {
      const msg = await invoke<string>('switch_model', { model_id: modelId });
      await refresh();
      await refreshStatus();
      const successText = `${msg} Orchestrator state refreshed.`;
      switchDetails = successText;
      modelSwitchStatus.set(successText);
      addAssistantMessage(msg);
    } catch (e) {
      errorMsg = String(e);
      const failText = `Model switch failed: ${errorMsg}`;
      switchDetails = failText;
      modelSwitchStatus.set(failText);
    } finally {
      loadingOp = '';
    }
  }

  async function downloadGguf(model: Model) {
    if (!model.download_url || !model.file_name) {
      addAssistantMessage('⚠️ No download URL for this model. Use **Import GGUF** to load a local file.');
      return;
    }
    loadingOp = `Downloading ${model.name}…`;
    try {
      const path = await invoke<string>('download_gguf_model', {
        url: model.download_url,
        fileName: model.file_name,
      });
      addAssistantMessage(`✅ Model saved to \`${path}\``);
      await refresh();
    } catch (e) { errorMsg = String(e); }
    finally { loadingOp = ''; }
  }

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
      if (path) { addAssistantMessage(`📦 Model imported from \`${path}\``); await refresh(); }
    } catch (e) { errorMsg = String(e); }
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
        {#each models as model (model.id)}
          <div class="model-row" class:active-model={model.id === $activeModel?.id}>
            <div class="model-info">
              <div class="model-name">{model.name}</div>
              <div class="model-meta">{model.quant} · ~{Math.round(model.ram_required_mb / 1024 * 10) / 10} GB RAM</div>
            </div>
            <div class="model-actions">
              {#if model.id === $activeModel?.id}
                <span class="badge-active">Active</span>
              {:else}
                <button class="btn-sm" on:click={() => switchModel(model.id, model.name)} disabled={!!loadingOp}>
                  Use
                </button>
              {/if}
              {#if model.download_url}
                <button class="btn-sm" on:click={() => downloadGguf(model)} disabled={!!loadingOp}
                  title="Download {model.name}">
                  ⬇
                </button>
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

  .model-name { font-size: 13px; font-weight: 500; }
  .model-meta { font-size: 11px; color: var(--text-dim); margin-top: 2px; }

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
  @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.7} }
</style>
