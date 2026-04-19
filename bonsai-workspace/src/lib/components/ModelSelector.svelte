<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
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
  import { swarmEnabled } from '$lib/stores/agents';
  import {
    availableModels,
    activeModel,
    activeModelId,
    orchestratorStatus,
    modelSwitchStatus,
    CUSTOM_SWARM_MODEL_ID,
  } from '$lib/stores/models';
  import type { ModelInfo } from '$lib/stores/models';

  export let inline = false;

  let open = false;
  let selectorEl: HTMLDivElement;
  let dropdownEl: HTMLDivElement | null = null;
  let dropdownStyle = '';

  function toggle() { open = !open; }

  function close(e: MouseEvent) {
    if (selectorEl && !selectorEl.contains(e.target as Node)) open = false;
  }

  function clearDropdownStyle() {
    dropdownStyle = '';
  }

  function recomputeDropdownLayout() {
    if (!open || !selectorEl) return;

    const rect = selectorEl.getBoundingClientRect();
    const viewportW = window.innerWidth;
    const viewportH = window.innerHeight;
    const margin = 8;
    const gap = 6;

    // Scale width with viewport while keeping a comfortable desktop default.
    const maxWidth = Math.max(280, Math.min(560, viewportW - margin * 2));
    const preferredWidth = inline ? 420 : 380;
    const minWidth = Math.min(300, maxWidth);
    const width = Math.max(minWidth, Math.min(preferredWidth, maxWidth));

    let left = rect.right - width;
    if (left < margin) left = margin;
    if (left + width > viewportW - margin) left = viewportW - margin - width;

    const availableAbove = Math.max(0, rect.top - margin - gap);
    const availableBelow = Math.max(0, viewportH - rect.bottom - margin - gap);
    const placeAbove = availableAbove >= availableBelow;
    const preferredMaxHeight = inline ? 560 : 520;
    const chosenSpace = placeAbove ? availableAbove : availableBelow;
    const maxHeight = Math.max(180, Math.min(preferredMaxHeight, chosenSpace));

    if (placeAbove) {
      const bottom = Math.max(margin, viewportH - rect.top + gap);
      dropdownStyle = `position: fixed; left: ${left}px; width: ${width}px; max-height: ${maxHeight}px; bottom: ${bottom}px;`;
      return;
    }

    const top = Math.min(viewportH - margin - 180, rect.bottom + gap);
    dropdownStyle = `position: fixed; left: ${left}px; width: ${width}px; max-height: ${maxHeight}px; top: ${top}px;`;
  }

  async function openWithLayout() {
    open = !open;
    if (!open) {
      clearDropdownStyle();
      return;
    }

    await tick();
    recomputeDropdownLayout();
  }

  function openAgentsPanel() {
    window.dispatchEvent(new CustomEvent('open-agents'));
    open = false;
    clearDropdownStyle();
  }

  function handleViewportChange() {
    recomputeDropdownLayout();
  }

  onMount(() => {
    window.addEventListener('click', close, true);
    window.addEventListener('resize', handleViewportChange);
    // Capture scroll events from nested containers as well.
    window.addEventListener('scroll', handleViewportChange, true);
  });

  onDestroy(() => {
    window.removeEventListener('click', close, true);
    window.removeEventListener('resize', handleViewportChange);
    window.removeEventListener('scroll', handleViewportChange, true);
  });

  async function pickModel(entry: typeof BONSAI_CATALOG[number]) {
    open = false;
    clearDropdownStyle();
    await switchToCatalogModel(entry);
  }

  async function pickInstalledModel(model: ModelInfo) {
    open = false;
    clearDropdownStyle();
    autoMode.set(false);
    activeModelId.set(model.id);
    modelSwitchStatus.set(`Switching to ${model.name}...`);
    try {
      const msg = await invoke<string>('switch_model', { modelId: model.id });
      modelSwitchStatus.set(msg);
    } catch (e) {
      modelSwitchStatus.set(`Switch failed: ${e}`);
    }
  }

  async function pickAuto() {
    open = false;
    clearDropdownStyle();
    autoMode.set(true);
    await triggerAutoSelect();
  }

  function pickCustomSwarm() {
    open = false;
    clearDropdownStyle();
    autoMode.set(false);
    activeModelId.set(CUSTOM_SWARM_MODEL_ID);
  }

  async function download(e: MouseEvent, entry: typeof BONSAI_CATALOG[number]) {
    e.stopPropagation();
    await downloadCatalogModel(entry);
  }

  $: isCustomSwarmActive = $swarmEnabled || $activeModelId === CUSTOM_SWARM_MODEL_ID;

  $: label = isCustomSwarmActive
    ? 'Custom Swarm'
    : $autoMode
      ? `Auto · ${$activeModel?.name ?? 'selecting…'}`
      : ($activeModel?.name ?? 'No Model Selected');

  $: freeGb = $orchestratorStatus
    ? Math.round($orchestratorStatus.free_ram_mb / 1024 * 10) / 10
    : null;

  $: catalogRegistryIds = new Set(
    BONSAI_CATALOG
      .map((entry) => findRegistryModel(entry, $availableModels)?.id)
      .filter((id): id is string => Boolean(id)),
  );

  $: extraInstalledModels = $availableModels.filter((model) => !catalogRegistryIds.has(model.id));
</script>

<div class="model-selector-bar" class:inline>
  <div class="selector-anchor" bind:this={selectorEl}>
    <!-- Trigger button -->
    <button
      class="selector-trigger"
      class:has-model={!!$activeModel}
      class:auto-active={$autoMode}
      on:click={openWithLayout}
      aria-haspopup="listbox"
      aria-expanded={open}
    >
      <span class="trigger-dot" class:active={!!$activeModel} class:auto={$autoMode}></span>
      <span class="trigger-label">{label}</span>
      {#if isCustomSwarmActive}
        <button
          type="button"
          class="trigger-chip"
          on:click|stopPropagation={openAgentsPanel}
          title="Open Agents panel"
          aria-label="Open Agents panel"
        >
          Swarm Active
        </button>
      {/if}
      <span class="trigger-chevron" class:open>{open ? '▲' : '▼'}</span>
    </button>

    <!-- Dropdown -->
    {#if open}
      <div class="dropdown" bind:this={dropdownEl} style={dropdownStyle} role="listbox" aria-label="Select model">

    {#if $downloadError && !inline}
    <span class="dl-error" title={$downloadError}>⚠ {$downloadError}</span>
    <button class="dl-dismiss" on:click={() => downloadError.set(null)}>✕</button>
  {/if}

    {#if $downloadingId && !inline}
    <span class="dl-progress">⬇ {$downloadPct}%</span>
  {/if}

        <div
          class="option option-custom"
          class:selected={isCustomSwarmActive}
          role="option"
          aria-selected={isCustomSwarmActive}
          tabindex="0"
          on:click={pickCustomSwarm}
          on:keydown={(e) => e.key === 'Enter' && pickCustomSwarm()}
        >
          <span class="opt-icon">🧩</span>
          <div class="opt-body">
            <span class="opt-name">Custom Swarm</span>
            <span class="opt-desc">Use multi-agent orchestration with configured leader and workers</span>
          </div>
          {#if isCustomSwarmActive}<span class="opt-badge active">Active</span>{/if}
        </div>

        <div class="divider"></div>

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

        {#if extraInstalledModels.length > 0}
          <div class="divider"></div>
          <div class="section-label">Installed models</div>
          {#each extraInstalledModels as model (model.id)}
            {@const isInstalledActive = $activeModel?.id === model.id}
            <div
              class="option"
              class:selected={isInstalledActive}
              role="option"
              aria-selected={isInstalledActive}
              tabindex="0"
              on:click={() => pickInstalledModel(model)}
              on:keydown={(e) => e.key === 'Enter' && pickInstalledModel(model)}
            >
              <span class="opt-icon">🧠</span>
              <div class="opt-body">
                <div class="opt-name-row">
                  <span class="opt-name">{model.name}</span>
                  <span class="opt-quant">{model.quant}</span>
                </div>
                <span class="opt-desc">Local model · {Math.round((model.ram_required_mb / 1024) * 10) / 10} GB</span>
              </div>
              <div class="opt-actions">
                {#if isInstalledActive}
                  <span class="opt-badge active">Active</span>
                {:else}
                  <span class="opt-badge local">Use</span>
                {/if}
              </div>
            </div>
          {/each}
        {/if}

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
    max-width: var(--model-inline-trigger-max, 220px);
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

  .trigger-chip {
    display: inline-flex;
    align-items: center;
    border: 1px solid rgba(34, 197, 94, 0.35);
    background: rgba(22, 163, 74, 0.2);
    color: #dcfce7;
    border-radius: 999px;
    font-size: 10px;
    line-height: 1;
    padding: 3px 7px;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
  }

  .trigger-chip:hover {
    filter: brightness(1.08);
  }

  .model-selector-bar.inline .trigger-chip {
    border-color: rgba(34, 197, 94, 0.3);
    background: rgba(22, 163, 74, 0.15);
    color: #86efac;
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
    width: min(380px, calc(100vw - 16px));
    background: #1c1c1f;
    border: 1px solid #b45309;
    border-radius: 10px;
    box-shadow: 0 -8px 32px rgba(0,0,0,0.5);
    max-height: min(520px, calc(100vh - 16px));
    overflow: auto;
    overscroll-behavior: contain;
    z-index: 1300;
  }

  .model-selector-bar.inline .dropdown {
    right: 0;
    left: auto;
    bottom: calc(100% + 6px);
    width: min(420px, calc(100vw - 16px));
    max-height: min(560px, calc(100vh - 16px));
  }

  @media (max-width: 900px) {
    .dropdown {
      width: min(96vw, 440px);
    }
  }

  .section-label {
    padding: 6px 12px 4px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #a1a1aa;
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
