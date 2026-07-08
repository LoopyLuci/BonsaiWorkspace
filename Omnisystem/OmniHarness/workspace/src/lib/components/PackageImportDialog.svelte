<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';

  export let initialPath: string = '';

  const dispatch = createEventDispatcher<{
    imported: { name: string; moduleCount: number };
    close: void;
  }>();

  interface PackageSummary {
    id: string;
    name: string;
    version: string;
    description: string;
    base_model_name: string;
    base_model_arch: string;
    adapter_count: number;
    module_count: number;
    created_at: string;
  }

  let path = initialPath;
  let summary: PackageSummary | null = null;
  let entries: string[] = [];
  let loading = false;
  let error = '';
  let phase: 'idle' | 'inspecting' | 'ready' | 'importing' | 'done' = 'idle';

  onMount(() => {
    if (path) inspect();
  });

  async function pickFile() {
    const selected = await openDialog({
      title: 'Open Workspace Package',
      filters: [{ name: 'Workspace Package', extensions: ['bkp'] }],
    });
    if (!selected) return;
    path = typeof selected === 'string' ? selected : ((selected as string[] | null)?.[0] ?? '');
    await inspect();
  }

  async function inspect() {
    loading = true;
    error = '';
    phase = 'inspecting';
    try {
      summary = await invoke<PackageSummary>('package_inspect', { path });
      entries = await invoke<string[]>('package_list_entries', { path });
      phase = 'ready';
    } catch (e) {
      error = String(e);
      phase = 'idle';
    } finally {
      loading = false;
    }
  }

  async function doImport() {
    if (!summary) return;
    loading = true;
    phase = 'importing';
    error = '';
    try {
      await invoke('package_import', { path });
      phase = 'done';
      dispatch('imported', { name: summary.name, moduleCount: summary.module_count });
    } catch (e) {
      error = String(e);
      phase = 'ready';
    } finally {
      loading = false;
    }
  }

  function close() { dispatch('close'); }

  function formatDate(iso: string) {
    try { return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }); }
    catch { return iso; }
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-noninteractive-element-interactions -->
<div class="overlay" on:click|self={close} role="dialog" aria-modal="true" aria-label="Import Workspace Package">
  <div class="dialog">
    <button class="close-btn" on:click={close} aria-label="Close">✕</button>

    {#if phase === 'done' && summary}
      <div class="success">
        <span class="big-icon">✅</span>
        <h3>Package Imported!</h3>
        <p><strong>{summary.name}</strong> is now in your library.</p>
        <p class="hint">{summary.module_count} knowledge module{summary.module_count !== 1 ? 's' : ''} added. Open Model Builder to activate them.</p>
        <button class="btn primary" on:click={close}>Done</button>
      </div>

    {:else}
      <h3>📦 Import Workspace Package</h3>

      {#if !summary}
        <div class="file-pick">
          {#if path}
            <p class="path-preview">{path}</p>
          {:else}
            <p class="hint">Choose a .bkp file to inspect before importing.</p>
          {/if}
          <button class="btn" on:click={pickFile} disabled={loading}>
            {path ? 'Choose Different File' : 'Choose .bkp File…'}
          </button>
        </div>
      {:else}
        <div class="summary-card">
          <div class="summary-row">
            <span class="label">Name</span>
            <strong>{summary.name} v{summary.version}</strong>
          </div>
          {#if summary.description}
            <div class="summary-row">
              <span class="label">About</span>
              <span>{summary.description}</span>
            </div>
          {/if}
          <div class="summary-row">
            <span class="label">Base Model</span>
            <span>{summary.base_model_name} ({summary.base_model_arch})</span>
          </div>
          <div class="summary-row">
            <span class="label">Knowledge Modules</span>
            <span>{summary.module_count}</span>
          </div>
          {#if summary.adapter_count > 0}
            <div class="summary-row">
              <span class="label">Adapters</span>
              <span>{summary.adapter_count}</span>
            </div>
          {/if}
          <div class="summary-row">
            <span class="label">Created</span>
            <span>{formatDate(summary.created_at)}</span>
          </div>
        </div>

        {#if entries.length > 0}
          <details class="entries-details">
            <summary>Show {entries.length} files in package</summary>
            <ul class="entries-list">
              {#each entries.slice(0, 40) as entry}
                <li>{entry}</li>
              {/each}
              {#if entries.length > 40}
                <li class="more">…and {entries.length - 40} more</li>
              {/if}
            </ul>
          </details>
        {/if}

        <button class="btn small secondary" on:click={() => { summary = null; path = ''; phase = 'idle'; }}>
          ← Choose Different File
        </button>
      {/if}

      {#if error}
        <p class="error-msg">⚠ {error}</p>
      {/if}

      {#if loading}
        <div class="progress-bar" aria-label="Loading…"></div>
      {/if}

      <div class="dialog-footer">
        <button class="btn" on:click={close}>Cancel</button>
        {#if summary && phase === 'ready'}
          <button class="btn primary" on:click={doImport} disabled={loading}>
            📥 Add to My Library
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 1rem;
  }

  .dialog {
    background: var(--bg-primary, #13131f);
    border: 1px solid var(--border, #333);
    border-radius: 14px;
    padding: 1.75rem;
    width: 100%;
    max-width: 500px;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-height: 90vh;
    overflow-y: auto;
  }

  .close-btn {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    opacity: 0.5;
    line-height: 1;
  }
  .close-btn:hover { opacity: 1; }

  h3 { margin: 0; font-size: 1.1rem; }

  .hint { margin: 0; color: var(--text-secondary, #888); font-size: 0.875rem; }
  .path-preview { margin: 0; font-size: 0.8rem; font-family: monospace; color: var(--accent, #3b82f6); word-break: break-all; }

  .file-pick { display: flex; flex-direction: column; gap: 0.75rem; }

  .summary-card {
    background: var(--bg-secondary, #1e1e2e);
    border-radius: 10px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .summary-row { display: flex; gap: 0.75rem; font-size: 0.875rem; }
  .label { color: var(--text-secondary, #888); min-width: 6rem; flex-shrink: 0; }

  .entries-details { font-size: 0.82rem; }
  .entries-details summary { cursor: pointer; color: var(--text-secondary, #888); padding: 0.25rem 0; }
  .entries-list { margin: 0.5rem 0 0; padding: 0 0 0 1rem; list-style: none; max-height: 120px; overflow-y: auto; display: flex; flex-direction: column; gap: 0.2rem; }
  .entries-list li { font-family: monospace; font-size: 0.78rem; color: var(--text-secondary, #888); }
  .more { font-style: italic; }

  .error-msg { color: #f87171; font-size: 0.875rem; margin: 0; background: rgba(248, 113, 113, 0.1); padding: 0.5rem 0.75rem; border-radius: 6px; }

  .progress-bar {
    height: 3px;
    background: linear-gradient(90deg, transparent, var(--accent, #3b82f6), transparent);
    background-size: 200% 100%;
    animation: shimmer 1.2s infinite;
    border-radius: 2px;
  }
  @keyframes shimmer { 0% { background-position: -200% 0; } 100% { background-position: 200% 0; } }

  .dialog-footer { display: flex; justify-content: flex-end; gap: 0.75rem; padding-top: 0.25rem; }

  .btn {
    padding: 0.55rem 1.1rem;
    border-radius: 8px;
    border: 1px solid var(--border, #333);
    background: var(--bg-secondary, #1e1e2e);
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.12s;
  }
  .btn:hover:not(:disabled) { background: var(--bg-hover, #2a2a3a); }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn.primary { background: var(--accent, #3b82f6); border-color: var(--accent, #3b82f6); color: white; }
  .btn.primary:hover:not(:disabled) { filter: brightness(1.15); }
  .btn.secondary { font-size: 0.8rem; padding: 0.3rem 0.7rem; }

  .success {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    text-align: center;
    padding: 1rem;
  }
  .big-icon { font-size: 2.5rem; }
  .success h3 { margin: 0; }
  .success p { margin: 0; font-size: 0.9rem; }
</style>
