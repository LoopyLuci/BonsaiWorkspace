<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    BONSAI_CATALOG,
    autoMode,
    downloadingId,
    downloadPct,
    downloadError,
    findRegistryModel,
    downloadCatalogModel,
    switchToCatalogModel,
    triggerAutoSelect,
  } from '$lib/stores/catalog';
  import { availableModels, activeModel, orchestratorStatus } from '$lib/stores/models';

  export let inline = false;

  let open = false;
  let selectorEl: HTMLDivElement;

  function toggle() { open = !open; }

  function close(e: MouseEvent) {
    if (selectorEl && !selectorEl.contains(e.target as Node)) open = false;
  }

  onMount(() => window.addEventListener('click', close, true));
  onDestroy(() => window.removeEventListener('click', close, true));

  async function pickModel(entry: typeof BONSAI_CATALOG[number]) {
    open = false;
    await switchToCatalogModel(entry);
  }

  async function pickAuto() {
    open = false;
    autoMode.set(true);
    await triggerAutoSelect();
  }

  async function download(e: MouseEvent, entry: typeof BONSAI_CATALOG[number]) {
    e.stopPropagation();
    await downloadCatalogModel(entry);
  }

  $: label = $autoMode
    ? `Auto · ${$activeModel?.name ?? 'selecting…'}`
    : ($activeModel?.name ?? 'No Model Selected');

  $: freeGb = $orchestratorStatus
    ? Math.round($orchestratorStatus.free_ram_mb / 1024 * 10) / 10
    : null;
</script>

<div class="model-selector-bar" class:inline>
  <div class="selector-anchor" bind:this={selectorEl}>
    <!-- Trigger button -->
    <button
      class="selector-trigger"
      class:has-model={!!$activeModel}
      class:auto-active={$autoMode}
      on:click={toggle}
      aria-haspopup="listbox"
      aria-expanded={open}
    >
      <span class="trigger-dot" class:active={!!$activeModel} class:auto={$autoMode}></span>
      <span class="trigger-label">{label}</span>
      <span class="trigger-chevron" class:open>{open ? '▲' : '▼'}</span>
    </button>

    <!-- Dropdown -->
    {#if open}
      <div class="dropdown" role="listbox" aria-label="Select model">

    {#if $downloadError && !inline}
    <span class="dl-error" title={$downloadError}>⚠ {$downloadError}</span>
    <button class="dl-dismiss" on:click={() => downloadError.set(null)}>✕</button>
  {/if}

    {#if $downloadingId && !inline}
    <span class="dl-progress">⬇ {$downloadPct}%</span>
  {/if}

        <div
          class="option option-auto"
          class:selected={$autoMode}
          role="option"
          aria-selected={$autoMode}
          tabindex="0"
          on:click={pickAuto}
          on:keydown={(e) => e.key === 'Enter' && pickAuto()}
        >
          <span class="opt-icon">⚡</span>
          <div class="opt-body">
            <span class="opt-name">Auto</span>
            <span class="opt-desc">Orchestrator picks best model for the task</span>
          </div>
          {#if $autoMode}<span class="opt-badge active">Active</span>{/if}
        </div>

        <div class="divider"></div>

        <!-- Catalog models -->
        {#each BONSAI_CATALOG as entry (entry.catalogId)}
          {@const reg = findRegistryModel(entry, $availableModels)}
          {@const isActive = reg && $activeModel?.id === reg.id}
          {@const isDownloading = $downloadingId === entry.catalogId}
          <div
            class="option"
            class:selected={isActive}
            class:unavailable={!reg}
            role="option"
            aria-selected={isActive ?? false}
            tabindex="0"
            on:click={() => reg && !isDownloading && pickModel(entry)}
            on:keydown={(e) => e.key === 'Enter' && reg && !isDownloading && pickModel(entry)}
          >
            <span class="opt-icon">{entry.params === '1.7B' ? '🌱' : entry.params === '4B' ? '🌿' : '🌳'}</span>
            <div class="opt-body">
              <div class="opt-name-row">
                <span class="opt-name">{entry.name}</span>
                <span class="opt-quant">{entry.quant}</span>
              </div>
              <span class="opt-desc">
                {entry.description}
                {#if freeGb !== null && !reg}
                  &nbsp;·&nbsp;
                  <span class:too-big={entry.ramGb > (freeGb ?? 999)}>
                    {entry.ramGb} GB needed
                  </span>
                {/if}
              </span>
            </div>
            <div class="opt-actions">
              {#if isActive}
                <span class="opt-badge active">Active</span>
              {:else if isDownloading}
                <span class="opt-badge downloading">{$downloadPct}%</span>
              {:else if reg}
                <span class="opt-badge local">Use</span>
              {:else}
                <button class="btn-dl" on:click={(e) => download(e, entry)} title="Download {entry.name}">
                  ⬇ Download
                </button>
              {/if}
            </div>
          </div>
        {/each}

      </div>
    {/if}
  </div>
</div>

<style>
  .model-selector-bar {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 0 10px;
    height: 28px;
    background: #92400e;         /* amber-900 */
    border-top: 1px solid #b45309;
    flex-shrink: 0;
    user-select: none;
    z-index: 100;
  }

  .model-selector-bar.inline {
    height: auto;
    padding: 0;
    background: transparent;
    border-top: none;
    z-index: 1200;
  }

  .selector-anchor {
    position: relative;
    margin-right: 16px;
    z-index: 1201;
  }

  .model-selector-bar.inline .selector-anchor {
    margin-right: 0;
  }

  .selector-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    color: #fef3c7;              /* amber-100 */
    font-size: 11.5px;
    font-weight: 500;
    transition: opacity 0.15s;
    min-width: 0;
  }
  .selector-trigger:hover { opacity: 0.85; }

  .model-selector-bar.inline .selector-trigger {
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    background: var(--bg);
    color: var(--text);
    max-width: 220px;
  }

  .model-selector-bar.inline .trigger-label {
    color: var(--text-dim);
  }

  .model-selector-bar.inline .selector-trigger:not(.has-model) .trigger-label {
    color: var(--text);
  }

  .trigger-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #78350f;         /* off / no model */
    border: 1.5px solid #fde68a;
    flex-shrink: 0;
    transition: background 0.2s;
  }
  .trigger-dot.active { background: #4ade80; }  /* green when loaded */
  .trigger-dot.auto   { background: #fbbf24; }  /* yellow when auto */

  .trigger-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #fef3c7;
    letter-spacing: 0.01em;
  }
  .selector-trigger:not(.has-model) .trigger-label {
    color: #fde68a;
    font-weight: 600;
  }

  .trigger-chevron {
    font-size: 8px;
    color: #fde68a;
    flex-shrink: 0;
  }

  .dl-progress {
    font-size: 11px;
    color: #fde68a;
    margin-left: 4px;
  }

  .dl-error {
    font-size: 11px;
    color: #fca5a5;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 280px;
  }
  .dl-dismiss {
    background: transparent;
    border: none;
    color: #fca5a5;
    cursor: pointer;
    font-size: 11px;
    padding: 0 2px;
  }

  /* ── Dropdown ── */
  .dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    right: 0;
    width: 360px;
    background: #1c1c1f;
    border: 1px solid #b45309;
    border-radius: 10px;
    box-shadow: 0 -8px 32px rgba(0,0,0,0.5);
    overflow: hidden;
    z-index: 1300;
  }

  .model-selector-bar.inline .dropdown {
    right: calc(100% + 8px);
    left: auto;
    bottom: calc(100% + 6px);
    width: min(360px, calc(100vw - 40px));
    max-height: min(420px, calc(100vh - 140px));
    overflow-y: auto;
  }

  .divider {
    height: 1px;
    background: #3f3f46;
    margin: 2px 0;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    cursor: pointer;
    transition: background 0.1s;
  }
  .option:hover:not(.unavailable) { background: #27272a; }
  .option.selected                { background: rgba(251,191,36,0.08); }
  .option.unavailable             { cursor: default; opacity: 0.75; }

  .option-auto { padding-top: 10px; }

  .opt-icon {
    font-size: 16px;
    flex-shrink: 0;
    width: 22px;
    text-align: center;
  }

  .opt-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .opt-name-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .opt-name {
    font-size: 13px;
    font-weight: 500;
    color: #e4e4e7;
    white-space: nowrap;
  }

  .opt-quant {
    font-size: 10px;
    background: #27272a;
    border: 1px solid #3f3f46;
    color: #a1a1aa;
    padding: 1px 5px;
    border-radius: 4px;
  }

  .opt-desc {
    font-size: 11px;
    color: #71717a;
  }

  .too-big { color: #f87171; }

  .opt-actions {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .opt-badge {
    font-size: 10px;
    padding: 2px 8px;
    border-radius: 10px;
    font-weight: 600;
    white-space: nowrap;
  }
  .opt-badge.active      { background: #fbbf24; color: #1c1c1f; }
  .opt-badge.local       { background: #16a34a; color: #fff; }
  .opt-badge.downloading { background: #78350f; color: #fef3c7; }

  .btn-dl {
    font-size: 11px;
    background: #1c1c1f;
    border: 1px solid #b45309;
    color: #fde68a;
    padding: 3px 9px;
    border-radius: 5px;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.12s;
  }
  .btn-dl:hover { background: #92400e; }
</style>
