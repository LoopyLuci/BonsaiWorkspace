<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { derived } from 'svelte/store';
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
    setModelSwitchStatus,
    modelDataList,
    CUSTOM_SWARM_MODEL_ID,
  } from '$lib/stores/models';
  import type { ModelInfo } from '$lib/stores/models';
  import type { ModelDataSummary, ModelTier, ModelStrength, ToolCallingSupport } from '$lib/types/model_data';

  export let inline = false;

  let open = false;
  let selectorEl: HTMLDivElement;
  let dropdownEl: HTMLDivElement | null = null;
  let dropdownStyle = '';

  // ── Model Data lookup ─────────────────────────────────────────────────────

  // Map from registry_id → ModelDataSummary for fast O(1) lookup.
  $: dataByRegistryId = (() => {
    const map = new Map<string, ModelDataSummary>();
    for (const d of $modelDataList) {
      // source_type is available on the summary; we need the registry_id.
      // The summary doesn't include registry_id directly, but the full
      // list_model_data returns summaries. We match by name as fallback,
      // but ideally we use the store's enrichment once data loads.
    }
    return map;
  })();

  // Simpler: build name → summary map for display enrichment.
  $: dataByName = (() => {
    const map = new Map<string, ModelDataSummary>();
    for (const d of $modelDataList) {
      map.set(d.name.toLowerCase(), d);
    }
    return map;
  })();

  function bestDataForModel(model: ModelInfo): ModelDataSummary | null {
    // Try exact name match first.
    const byName = dataByName.get(model.name.toLowerCase());
    if (byName) return byName;
    // Fuzzy: check if any ModelData name is contained in the model name or vice versa.
    for (const [key, d] of dataByName.entries()) {
      if (model.name.toLowerCase().includes(key) || key.includes(model.name.toLowerCase())) {
        return d;
      }
    }
    return null;
  }

  // ── Display helpers ───────────────────────────────────────────────────────

  function tierIcon(tier: ModelTier | undefined): string {
    switch (tier) {
      case 'frontier':    return '🔮';
      case 'capable':     return '💎';
      case 'fast':        return '⚡';
      case 'specialized': return '🔬';
      case 'embedded':    return '🌱';
      default:            return '🧠';
    }
  }

  function tierColor(tier: ModelTier | undefined): string {
    switch (tier) {
      case 'frontier':    return '#a78bfa';   /* violet */
      case 'capable':     return '#60a5fa';   /* blue */
      case 'fast':        return '#fbbf24';   /* amber */
      case 'specialized': return '#34d399';   /* teal */
      case 'embedded':    return '#86efac';   /* green */
      default:            return '#a1a1aa';
    }
  }

  function tierLabel(tier: ModelTier | undefined): string {
    switch (tier) {
      case 'frontier':    return 'Frontier';
      case 'capable':     return 'Capable';
      case 'fast':        return 'Fast';
      case 'specialized': return 'Specialized';
      case 'embedded':    return 'Embedded';
      default:            return '';
    }
  }

  function strengthLabel(s: ModelStrength): string {
    const map: Record<ModelStrength, string> = {
      coding: 'Code', math: 'Math', reasoning: 'Reasoning', writing: 'Writing',
      instruction: 'Instruction', multilingual: 'Multilingual', long_context: '128K+',
      speed: 'Fast', vision: 'Vision', research: 'Research', data_analysis: 'Data',
    };
    return map[s] ?? s;
  }

  function toolCallingIcon(tc: ToolCallingSupport | undefined): string {
    switch (tc) {
      case 'native':   return '🔧';
      case 'parallel': return '🔧';
      case 'basic':    return '🔩';
      default:         return '';
    }
  }

  function ctxLabel(tokens: number): string {
    if (tokens >= 1_000_000) return `${Math.round(tokens / 1_000_000)}M ctx`;
    if (tokens >= 1_000)     return `${Math.round(tokens / 1_000)}K ctx`;
    return `${tokens} ctx`;
  }

  function ramLabel(mb: number): string {
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
    return `${mb} MB`;
  }

  // Pick a clean display name: ModelData name first, then file stem.
  function displayName(model: ModelInfo, data: ModelDataSummary | null): string {
    return data?.name ?? model.name;
  }

  function displayDesc(model: ModelInfo, data: ModelDataSummary | null): string {
    if (data?.description) return data.description;
    return `Local · ${model.architecture} · ${ramLabel(model.ram_required_mb)}`;
  }

  // ── Layout ────────────────────────────────────────────────────────────────

  function clearDropdownStyle() { dropdownStyle = ''; }

  function recomputeDropdownLayout() {
    if (!open || !selectorEl) return;
    const rect = selectorEl.getBoundingClientRect();
    const viewportW = window.innerWidth;
    const viewportH = window.innerHeight;
    const margin = 8;
    const gap = 6;

    const maxWidth = Math.max(300, Math.min(600, viewportW - margin * 2));
    const preferredWidth = inline ? 460 : 420;
    const width = Math.max(300, Math.min(preferredWidth, maxWidth));

    let left = rect.right - width;
    if (left < margin) left = margin;
    if (left + width > viewportW - margin) left = viewportW - margin - width;

    const availableAbove = Math.max(0, rect.top - margin - gap);
    const availableBelow = Math.max(0, viewportH - rect.bottom - margin - gap);
    const placeAbove = availableAbove >= availableBelow;
    const preferredMaxHeight = inline ? 620 : 580;
    const chosenSpace = placeAbove ? availableAbove : availableBelow;
    const maxHeight = Math.max(200, Math.min(preferredMaxHeight, chosenSpace));

    if (placeAbove) {
      const bottom = Math.max(margin, viewportH - rect.top + gap);
      dropdownStyle = `position:fixed;left:${left}px;width:${width}px;max-height:${maxHeight}px;bottom:${bottom}px;`;
      return;
    }
    const top = Math.min(viewportH - margin - 200, rect.bottom + gap);
    dropdownStyle = `position:fixed;left:${left}px;width:${width}px;max-height:${maxHeight}px;top:${top}px;`;
  }

  async function openWithLayout() {
    open = !open;
    if (!open) { clearDropdownStyle(); return; }
    await tick();
    recomputeDropdownLayout();
  }

  function openAgentsPanel() {
    window.dispatchEvent(new CustomEvent('open-agents'));
    open = false;
    clearDropdownStyle();
  }

  onMount(() => {
    window.addEventListener('click', close, true);
    window.addEventListener('resize', recomputeDropdownLayout);
    window.addEventListener('scroll', recomputeDropdownLayout, true);
  });
  onDestroy(() => {
    window.removeEventListener('click', close, true);
    window.removeEventListener('resize', recomputeDropdownLayout);
    window.removeEventListener('scroll', recomputeDropdownLayout, true);
  });

  function close(e: MouseEvent) {
    if (selectorEl && !selectorEl.contains(e.target as Node)) open = false;
  }

  // ── Actions ───────────────────────────────────────────────────────────────

  async function pickModel(entry: typeof BONSAI_CATALOG[number]) {
    open = false; clearDropdownStyle();
    await switchToCatalogModel(entry);
  }

  async function pickInstalledModel(model: ModelInfo) {
    open = false; clearDropdownStyle();
    autoMode.set(false);
    activeModelId.set(model.id);
    modelSwitchStatus.set(`Loading ${model.name}…`);
    try {
      const msg = await invoke<string>('switch_model', { modelId: model.id });
      setModelSwitchStatus(msg, 5000);
    } catch (e) {
      modelSwitchStatus.set(`Switch failed: ${e}`);
    }
  }

  async function pickAuto() {
    open = false; clearDropdownStyle();
    autoMode.set(true);
    await triggerAutoSelect();
  }

  function pickCustomSwarm() {
    open = false; clearDropdownStyle();
    autoMode.set(false);
    activeModelId.set(CUSTOM_SWARM_MODEL_ID);
  }

  async function download(e: MouseEvent, entry: typeof BONSAI_CATALOG[number]) {
    e.stopPropagation();
    await downloadCatalogModel(entry);
  }

  // ── Derived state ─────────────────────────────────────────────────────────

  $: isCustomSwarmActive = $swarmEnabled || $activeModelId === CUSTOM_SWARM_MODEL_ID;

  $: label = isCustomSwarmActive
    ? 'Custom Swarm'
    : $autoMode
      ? `Auto · ${$activeModel?.name ?? 'selecting…'}`
      : ($activeModel ? displayName($activeModel, bestDataForModel($activeModel)) : 'Select Model');

  $: freeGb = $orchestratorStatus
    ? Math.round($orchestratorStatus.free_ram_mb / 1024 * 10) / 10
    : null;

  $: catalogRegistryIds = new Set(
    BONSAI_CATALOG
      .map(e => findRegistryModel(e, $availableModels)?.id)
      .filter((id): id is string => Boolean(id)),
  );

  $: extraInstalledModels = $availableModels.filter(m => !catalogRegistryIds.has(m.id));
</script>

<div class="model-selector-bar" class:inline>
  <div class="selector-anchor" bind:this={selectorEl}>

    <!-- Trigger -->
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
        <button type="button" class="trigger-chip" on:click|stopPropagation={openAgentsPanel}
          title="Open Agents panel" aria-label="Open Agents panel">Swarm Active</button>
      {/if}
      <span class="trigger-chevron" class:open>{open ? '▲' : '▼'}</span>
    </button>

    <!-- Dropdown -->
    {#if open}
      <div class="dropdown" bind:this={dropdownEl} style={dropdownStyle}
           role="listbox" aria-label="Select model">

        {#if $downloadError && !inline}
          <div class="status-bar error">
            <span>⚠ {$downloadError}</span>
            <button class="dismiss-btn" on:click={() => downloadError.set(null)}>✕</button>
          </div>
        {/if}
        {#if $downloadingId && !inline}
          <div class="status-bar info">⬇ Downloading… {$downloadPct}%</div>
        {/if}

        <!-- Custom Swarm -->
        <div class="option option-special"
          class:selected={isCustomSwarmActive} role="option" aria-selected={isCustomSwarmActive}
          tabindex="0" on:click={pickCustomSwarm}
          on:keydown={(e) => e.key === 'Enter' && pickCustomSwarm()}>
          <span class="opt-icon">🧩</span>
          <div class="opt-body">
            <span class="opt-name">Custom Swarm</span>
            <span class="opt-desc">Multi-agent orchestration with configured leader and workers</span>
          </div>
          {#if isCustomSwarmActive}<span class="opt-badge active">Active</span>{/if}
        </div>

        <div class="divider"></div>

        <!-- Auto -->
        <div class="option option-special"
          class:selected={$autoMode} role="option" aria-selected={$autoMode}
          tabindex="0" on:click={pickAuto}
          on:keydown={(e) => e.key === 'Enter' && pickAuto()}>
          <span class="opt-icon">⚡</span>
          <div class="opt-body">
            <span class="opt-name">Auto</span>
            <span class="opt-desc">Orchestrator picks the best loaded model for your task</span>
          </div>
          {#if $autoMode}<span class="opt-badge active">Active</span>{/if}
        </div>

        <div class="divider"></div>
        <div class="section-label">Bonsai Models</div>

        <!-- Catalog models -->
        {#each BONSAI_CATALOG as entry (entry.catalogId)}
          {@const reg = findRegistryModel(entry, $availableModels)}
          {@const isActive = reg && $activeModel?.id === reg.id}
          {@const isDownloading = $downloadingId === entry.catalogId}
          <div class="option" class:selected={isActive} class:unavailable={!reg}
            role="option" aria-selected={isActive ?? false} tabindex="0"
            on:click={() => reg && !isDownloading && pickModel(entry)}
            on:keydown={(e) => e.key === 'Enter' && reg && !isDownloading && pickModel(entry)}>
            <span class="opt-icon">{entry.params === '1.7B' ? '🌱' : entry.params === '4B' ? '🌿' : '🌳'}</span>
            <div class="opt-body">
              <div class="opt-name-row">
                <span class="opt-name">{entry.name}</span>
                <span class="opt-quant">{entry.quant}</span>
                <span class="tier-badge" style="color: #86efac">Embedded</span>
              </div>
              <span class="opt-desc">{entry.description}</span>
              <div class="capability-row">
                <span class="cap-tag">Instruction</span>
                <span class="cap-tag">Speed</span>
                <span class="cap-tag">Coding</span>
                {#if freeGb !== null && !reg}
                  <span class="cap-tag" class:too-big={entry.ramGb > (freeGb ?? 999)}>
                    {entry.ramGb} GB needed
                  </span>
                {/if}
              </div>
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

        <!-- Installed / extra models -->
        {#if extraInstalledModels.length > 0}
          <div class="divider"></div>
          <div class="section-label">Local Models</div>
          {#each extraInstalledModels as model (model.id)}
            {@const isActive = $activeModel?.id === model.id}
            {@const data = bestDataForModel(model)}
            {@const icon = tierIcon(data?.tier)}
            {@const tColor = tierColor(data?.tier)}
            {@const tLabel = tierLabel(data?.tier)}
            {@const name = displayName(model, data)}
            {@const desc = displayDesc(model, data)}
            {@const topStrengths = (data?.strengths ?? []).slice(0, 3)}
            <div class="option model-rich" class:selected={isActive}
              role="option" aria-selected={isActive} tabindex="0"
              on:click={() => pickInstalledModel(model)}
              on:keydown={(e) => e.key === 'Enter' && pickInstalledModel(model)}>

              <span class="opt-icon" title={tLabel}>{icon}</span>

              <div class="opt-body">
                <div class="opt-name-row">
                  <span class="opt-name">{name}</span>
                  {#if model.quant !== '?'}
                    <span class="opt-quant">{model.quant}</span>
                  {/if}
                  {#if tLabel}
                    <span class="tier-badge" style="color: {tColor}">{tLabel}</span>
                  {/if}
                </div>

                <span class="opt-desc">{desc}</span>

                <div class="capability-row">
                  {#each topStrengths as s}
                    <span class="cap-tag">{strengthLabel(s)}</span>
                  {/each}
                  {#if data?.tool_calling && data.tool_calling !== 'none'}
                    <span class="cap-tag tool-tag">
                      {toolCallingIcon(data.tool_calling)} Tools
                    </span>
                  {/if}
                  {#if data?.context_window && data.context_window > 4096}
                    <span class="cap-tag ctx-tag">{ctxLabel(data.context_window)}</span>
                  {/if}
                  {#if data?.extended_thinking}
                    <span class="cap-tag reason-tag">Reasoning</span>
                  {/if}
                  <span class="cap-tag ram-tag">{ramLabel(model.ram_required_mb)}</span>
                </div>
              </div>

              <div class="opt-actions">
                {#if isActive}
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
    background: #92400e;
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
  .model-selector-bar.inline .selector-anchor { margin-right: 0; }

  /* ── Trigger ── */
  .selector-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    color: #fef3c7;
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
    max-width: var(--model-inline-trigger-max, 240px);
  }
  .model-selector-bar.inline .trigger-label { color: var(--text-dim); }
  .model-selector-bar.inline .selector-trigger:not(.has-model) .trigger-label { color: var(--text); }

  .trigger-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: #78350f;
    border: 1.5px solid #fde68a;
    flex-shrink: 0;
    transition: background 0.2s;
  }
  .trigger-dot.active { background: #4ade80; }
  .trigger-dot.auto   { background: #fbbf24; }

  .trigger-label {
    flex: 1; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: #fef3c7;
    letter-spacing: 0.01em;
  }
  .selector-trigger:not(.has-model) .trigger-label { color: #fde68a; font-weight: 600; }

  .trigger-chevron { font-size: 8px; color: #fde68a; flex-shrink: 0; }

  .trigger-chip {
    display: inline-flex; align-items: center;
    border: 1px solid rgba(34,197,94,0.35);
    background: rgba(22,163,74,0.2);
    color: #dcfce7;
    border-radius: 999px;
    font-size: 10px; line-height: 1;
    padding: 3px 7px;
    white-space: nowrap; flex-shrink: 0; cursor: pointer;
  }

  .trigger-chip:hover { filter: brightness(1.08); }

  /* ── Dropdown ── */
  .dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    right: 0;
    width: min(420px, calc(100vw - 16px));
    background: #1c1c1f;
    border: 1px solid #b45309;
    border-radius: 10px;
    box-shadow: 0 -8px 32px rgba(0,0,0,0.55);
    max-height: min(580px, calc(100vh - 16px));
    overflow: auto;
    overscroll-behavior: contain;
    z-index: 1300;
  }
  .model-selector-bar.inline .dropdown {
    right: 0; left: auto;
    bottom: calc(100% + 6px);
    width: min(460px, calc(100vw - 16px));
    max-height: min(620px, calc(100vh - 16px));
  }
  @media (max-width: 900px) { .dropdown { width: min(96vw, 460px); } }

  /* ── Status bars ── */
  .status-bar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 6px 12px;
    font-size: 11px;
    border-bottom: 1px solid #3f3f46;
  }
  .status-bar.error { color: #fca5a5; background: rgba(239,68,68,0.08); }
  .status-bar.info  { color: #fde68a; }
  .dismiss-btn {
    background: transparent; border: none; color: #fca5a5;
    cursor: pointer; font-size: 11px; padding: 0 2px;
  }

  /* ── Sections ── */
  .section-label {
    padding: 8px 12px 3px;
    font-size: 10px; letter-spacing: 0.08em;
    text-transform: uppercase; color: #71717a;
  }
  .divider { height: 1px; background: #3f3f46; margin: 4px 0; }

  /* ── Options ── */
  .option {
    display: flex; align-items: flex-start;
    gap: 10px; padding: 9px 12px;
    cursor: pointer;
    transition: background 0.1s;
  }
  .option:hover:not(.unavailable) { background: #27272a; }
  .option.selected { background: rgba(251,191,36,0.08); }
  .option.unavailable { cursor: default; opacity: 0.72; }
  .option-special { align-items: center; }

  .opt-icon {
    font-size: 17px; flex-shrink: 0;
    width: 22px; text-align: center;
    margin-top: 1px;
  }

  .opt-body {
    flex: 1; min-width: 0;
    display: flex; flex-direction: column; gap: 3px;
  }

  .opt-name-row {
    display: flex; align-items: center;
    gap: 6px; flex-wrap: wrap;
  }

  .opt-name {
    font-size: 13px; font-weight: 500; color: #e4e4e7;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    max-width: 200px;
  }

  .opt-quant {
    font-size: 10px;
    background: #27272a; border: 1px solid #3f3f46;
    color: #a1a1aa; padding: 1px 5px; border-radius: 4px;
    white-space: nowrap;
  }

  .tier-badge {
    font-size: 10px; font-weight: 600;
    letter-spacing: 0.03em; white-space: nowrap;
  }

  .opt-desc {
    font-size: 11px; color: #71717a;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  /* ── Capability chips ── */
  .capability-row {
    display: flex; align-items: center;
    flex-wrap: wrap; gap: 4px;
    margin-top: 2px;
  }

  .cap-tag {
    font-size: 10px; font-weight: 500;
    padding: 1px 6px; border-radius: 4px;
    background: #27272a; border: 1px solid #3f3f46;
    color: #a1a1aa; white-space: nowrap;
  }
  .cap-tag.tool-tag   { border-color: #854d0e; color: #fde68a; background: rgba(133,77,14,0.18); }
  .cap-tag.ctx-tag    { border-color: #1e3a5f; color: #93c5fd; background: rgba(30,58,95,0.25); }
  .cap-tag.reason-tag { border-color: #4c1d95; color: #c4b5fd; background: rgba(76,29,149,0.2); }
  .cap-tag.ram-tag    { color: #6b7280; }
  .too-big { color: #f87171 !important; }

  /* ── Actions ── */
  .opt-actions {
    flex-shrink: 0; display: flex;
    align-items: flex-start; padding-top: 1px;
  }

  .opt-badge {
    font-size: 10px; padding: 2px 8px;
    border-radius: 10px; font-weight: 600; white-space: nowrap;
  }
  .opt-badge.active      { background: #fbbf24; color: #1c1c1f; }
  .opt-badge.local       { background: #16a34a; color: #fff; }
  .opt-badge.downloading { background: #78350f; color: #fef3c7; }

  .btn-dl {
    font-size: 11px;
    background: #1c1c1f; border: 1px solid #b45309;
    color: #fde68a; padding: 3px 9px; border-radius: 5px;
    cursor: pointer; white-space: nowrap;
    transition: background 0.12s;
  }
  .btn-dl:hover { background: #92400e; }

  /* Rich installed model rows get a bit more padding */
  .model-rich { padding: 10px 12px; }
</style>
