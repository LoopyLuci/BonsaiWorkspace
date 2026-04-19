<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { get } from 'svelte/store';
  import {
    activeModel,
    activeModelId,
    availableModels,
    modelSwitchStatus,
    orchestratorStatus,
    refreshModels,
    refreshStatus,
  } from '$lib/stores/models';
  import { apiBaseUrl, apiHost, apiPort, saveApiSettings } from '$lib/stores/settings';
  import {
    applyAutoDetectedMobileDisplaySettings,
    applyMobileDisplayPreview,
    confirmMobileDisplaySettings,
    mobileDisplayPending,
    mobileDisplaySettings,
    resetMobileDisplaySettings,
    revertUnconfirmedMobileDisplaySettings,
  } from '$lib/stores/mobileDisplay';

  let loading = false;
  let errorMsg = '';
  let infoMsg = '';

  async function switchModelWithRetry(modelId: string, modelName: string) {
    loading = true;
    errorMsg = '';
    infoMsg = '';
    activeModelId.set(modelId);
    modelSwitchStatus.set(`Switching to ${modelName}...`);

    try {
      const msg = await invoke<string>('switch_model', { modelId });
      modelSwitchStatus.set(msg);
      await refreshModels();
      await refreshStatus();
      return;
    } catch (error) {
      const msg = String(error);
      if (!/model load timeout/i.test(msg)) {
        modelSwitchStatus.set(`Switch failed: ${msg}`);
        return;
      }

      // Model loading can be slow on mobile; allow a short grace polling window
      // before declaring failure.
      modelSwitchStatus.set(`Switch timed out; waiting for ${modelName} to become ready...`);
      const deadline = Date.now() + 180000;
      while (Date.now() < deadline) {
        await refreshStatus();
        const status = get(orchestratorStatus);
        const isReady = status?.slots.some((slot) =>
          slot.state.state === 'ready' && slot.state.model_id === modelId,
        );
        if (isReady) {
          modelSwitchStatus.set(`Model ${modelName} became ready after timeout grace window.`);
          await refreshModels();
          infoMsg = `Model switched to ${modelName}.`;
          return;
        }
        await new Promise((resolve) => setTimeout(resolve, 1500));
      }

      modelSwitchStatus.set(`Switch failed: ${msg}`);
    } finally {
      loading = false;
    }
  }

  async function saveApi() {
    loading = true;
    errorMsg = '';
    infoMsg = '';
    try {
      await saveApiSettings($apiHost, Number($apiPort));
      infoMsg = 'API settings saved.';
    } catch (error) {
      errorMsg = String(error);
    } finally {
      loading = false;
    }
  }

  async function testApi() {
    loading = true;
    errorMsg = '';
    infoMsg = '';
    try {
      const resp = await fetch(`${$apiBaseUrl}/v1/models`);
      const json = await resp.json().catch(() => ({}));
      if (resp.ok) {
        infoMsg = `API reachable: ${json.data?.length ?? 'unknown'} model(s) available.`;
      } else {
        errorMsg = `API error: ${json.error?.message ?? resp.statusText}`;
      }
    } catch (error) {
      errorMsg = `API test failed: ${String(error)}`;
    } finally {
      loading = false;
    }
  }

  function sliderValue(event: Event): number {
    const target = event.currentTarget as HTMLInputElement | null;
    return Number(target?.value ?? 0);
  }

  function previewDisplayPatch(patch: {
    topOffsetPx?: number;
    bottomOffsetPx?: number;
    leftOffsetPx?: number;
    rightOffsetPx?: number;
  }) {
    applyMobileDisplayPreview(patch, { source: 'manual' });
    infoMsg = 'Screen adjustment preview started. Confirm within 30 seconds to keep it.';
    errorMsg = '';
  }

  function autoDetectDisplay() {
    const detected = applyAutoDetectedMobileDisplaySettings();
    infoMsg = `Auto-detected insets: top ${detected.topOffsetPx}px, bottom ${detected.bottomOffsetPx}px, left ${detected.leftOffsetPx}px, right ${detected.rightOffsetPx}px.`;
    errorMsg = '';
  }

  function keepDisplayChanges() {
    confirmMobileDisplaySettings();
    infoMsg = 'Screen adjustment confirmed and saved.';
    errorMsg = '';
  }

  function revertDisplayChanges() {
    revertUnconfirmedMobileDisplaySettings();
    infoMsg = 'Unconfirmed screen changes reverted to the last confirmed layout.';
    errorMsg = '';
  }

  function resetDisplayDefaults() {
    resetMobileDisplaySettings();
    infoMsg = 'Screen adjustments reset to defaults.';
    errorMsg = '';
  }
</script>

<div class="mobile-settings">
  <section class="panel">
    <h3>Model</h3>
    <div class="model-list">
      {#each $availableModels as model (model.id)}
        <button
          class="model-btn"
          class:active={model.id === $activeModel?.id}
          on:click={() => switchModelWithRetry(model.id, model.name)}
          disabled={loading}
          type="button"
        >
          <span>{model.name}</span>
          <small>{model.quant} - {Math.round((model.ram_required_mb / 1024) * 10) / 10} GB</small>
        </button>
      {/each}
    </div>
    <div class="actions">
      <button type="button" on:click={refreshModels} disabled={loading}>Refresh Models</button>
      <button type="button" on:click={refreshStatus} disabled={loading}>Refresh Status</button>
    </div>
    {#if $modelSwitchStatus}
      <p class="hint">{$modelSwitchStatus}</p>
    {/if}
  </section>

  <section class="panel">
    <h3>API Endpoint</h3>
    <label>
      Host
      <input type="text" bind:value={$apiHost} />
    </label>
    <label>
      Port
      <input type="number" min="1" max="65535" bind:value={$apiPort} />
    </label>
    <button type="button" on:click={saveApi} disabled={loading}>Save API</button>
    <button type="button" on:click={testApi} disabled={loading}>Test API</button>
    <p class="hint">{$apiBaseUrl}</p>
  </section>

  <section class="panel">
    <h3>Mobile Screen Adjustment</h3>
    <p class="hint">Auto-detect viewport insets, fine-tune offsets manually, and confirm changes to keep them.</p>

    <div class="actions">
      <button type="button" on:click={autoDetectDisplay}>Auto Detect Screen Size</button>
      <button type="button" on:click={keepDisplayChanges} disabled={!$mobileDisplayPending.isPending}>Keep Screen Changes</button>
    </div>

    <div class="actions">
      <button type="button" on:click={revertDisplayChanges} disabled={!$mobileDisplayPending.isPending}>Revert Now</button>
      <button type="button" on:click={resetDisplayDefaults}>Reset to Defaults</button>
    </div>

    {#if $mobileDisplayPending.isPending}
      <p class="warn">
        Preview active ({$mobileDisplayPending.source}). Reverting in {$mobileDisplayPending.secondsLeft}s unless confirmed.
      </p>
    {/if}

    <label>
      Top offset ({$mobileDisplaySettings.topOffsetPx}px)
      <input
        type="range"
        min="-24"
        max="72"
        step="1"
        value={$mobileDisplaySettings.topOffsetPx}
        on:input={(e) => previewDisplayPatch({ topOffsetPx: sliderValue(e) })}
      />
    </label>

    <label>
      Bottom offset ({$mobileDisplaySettings.bottomOffsetPx}px)
      <input
        type="range"
        min="-24"
        max="96"
        step="1"
        value={$mobileDisplaySettings.bottomOffsetPx}
        on:input={(e) => previewDisplayPatch({ bottomOffsetPx: sliderValue(e) })}
      />
    </label>

    <label>
      Left offset ({$mobileDisplaySettings.leftOffsetPx}px)
      <input
        type="range"
        min="-24"
        max="48"
        step="1"
        value={$mobileDisplaySettings.leftOffsetPx}
        on:input={(e) => previewDisplayPatch({ leftOffsetPx: sliderValue(e) })}
      />
    </label>

    <label>
      Right offset ({$mobileDisplaySettings.rightOffsetPx}px)
      <input
        type="range"
        min="-24"
        max="48"
        step="1"
        value={$mobileDisplaySettings.rightOffsetPx}
        on:input={(e) => previewDisplayPatch({ rightOffsetPx: sliderValue(e) })}
      />
    </label>
  </section>

  {#if errorMsg}
    <p class="error">{errorMsg}</p>
  {/if}
  {#if infoMsg}
    <p class="ok">{infoMsg}</p>
  {/if}
</div>

<style>
  .mobile-settings {
    height: 100%;
    overflow: auto;
    padding: 14px;
    display: grid;
    gap: 12px;
    background: var(--bg);
  }

  .panel {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg2);
    padding: 12px;
    display: grid;
    gap: 10px;
  }

  h3 {
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .model-list {
    display: grid;
    gap: 8px;
  }

  .model-btn {
    text-align: left;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    background: var(--bg);
    color: var(--text);
    display: grid;
    gap: 2px;
  }

  .model-btn.active {
    border-color: var(--accent-hl);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-hl) 40%, transparent);
  }

  .model-btn small {
    color: var(--text-dim);
  }

  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  label {
    display: grid;
    gap: 6px;
    font-size: 12px;
    color: var(--text-dim);
  }

  input[type='text'],
  input[type='number'] {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    padding: 8px 10px;
  }

  input[type='range'] {
    width: 100%;
  }

  button {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    background: var(--bg);
    color: var(--text);
  }

  .hint {
    font-size: 11px;
    color: var(--text-dim);
    line-height: 1.4;
  }

  .warn {
    color: var(--yellow);
    font-size: 11px;
    line-height: 1.4;
  }

  .error {
    color: var(--red);
    font-size: 12px;
  }

  .ok {
    color: var(--green);
    font-size: 12px;
  }
</style>
