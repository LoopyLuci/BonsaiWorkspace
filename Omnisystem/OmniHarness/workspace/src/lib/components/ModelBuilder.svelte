<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';

  interface ModuleInfo {
    id: string;
    name: string;
    domain: string;
    version: string;
    entry_count: number;
    path: string;
  }

  interface ModelEntry {
    name: string;
    path: string;
  }

  let baseModel: ModelEntry | null = null;
  let availableModels: ModelEntry[] = [];
  let registeredModules: ModuleInfo[] = [];
  let loadedModuleNames = new Set<string>();
  let loading = false;
  let statusMessage = '';
  let testChatMessage = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    loading = true;
    try {
      const [models, modules, loaded] = await Promise.all([
        invoke<ModelEntry[]>('list_available_models').catch(() => [] as ModelEntry[]),
        invoke<ModuleInfo[]>('kdb_list_modules').catch(() => [] as ModuleInfo[]),
        invoke<ModuleInfo[]>('kdb_list_loaded_modules').catch(() => [] as ModuleInfo[]),
      ]);
      availableModels = models;
      registeredModules = modules;
      loadedModuleNames = new Set(loaded.map(m => m.name));
    } finally {
      loading = false;
    }
  }

  async function toggleModule(mod: ModuleInfo) {
    try {
      if (loadedModuleNames.has(mod.name)) {
        await invoke('kdb_unload_module', { moduleName: mod.name });
        loadedModuleNames.delete(mod.name);
        loadedModuleNames = new Set(loadedModuleNames);
        statusMessage = `Unloaded: ${mod.name}`;
      } else {
        await invoke('kdb_load_module', { moduleName: mod.name });
        loadedModuleNames.add(mod.name);
        loadedModuleNames = new Set(loadedModuleNames);
        statusMessage = `Loaded: ${mod.name}`;
      }
    } catch (e) {
      statusMessage = `Error: ${e}`;
    }
  }

  async function importPackage() {
    const selected = await openDialog({
      title: 'Import Workspace Package',
      filters: [{ name: 'Workspace Package', extensions: ['bkp'] }],
    });
    if (!selected) return;
    const path = typeof selected === 'string' ? selected : (selected as string[] | null)?.[0];
    if (!path) return;
    try {
      loading = true;
      const summary = await invoke<any>('package_import', { path });
      statusMessage = `Imported "${summary.name}" — ${summary.module_count} modules, base: ${summary.base_model_name}`;
      await refresh();
    } catch (e) {
      statusMessage = `Import failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function exportPackage() {
    if (!baseModel) { statusMessage = 'Select a base model first.'; return; }
    const outPath = await saveDialog({
      title: 'Export Workspace Package',
      defaultPath: `${baseModel.name}.bkp`,
      filters: [{ name: 'Workspace Package', extensions: ['bkp'] }],
    });
    if (!outPath) return;
    try {
      loading = true;
      const result = await invoke<string>('package_export', {
        baseModelPath: baseModel.path,
        baseModelName: baseModel.name,
        baseModelArch: 'gguf',
        packageName: baseModel.name,
        packageVersion: '1.0.0',
        description: `Workspace package for ${baseModel.name}`,
        outputPath: outPath,
      });
      statusMessage = `Exported to: ${result}`;
    } catch (e) {
      statusMessage = `Export failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function testModel() {
    if (!baseModel) { statusMessage = 'Select a base model first.'; return; }
    try {
      const context = await invoke<string>('kdb_format_context');
      testChatMessage = context
        ? `[KDB context injected]\n\n${context.slice(0, 200)}…`
        : 'No knowledge modules loaded — chat will use base model only.';
    } catch (e) {
      testChatMessage = `Error: ${e}`;
    }
  }

  async function deleteModule(mod: ModuleInfo) {
    if (!confirm(`Delete module "${mod.name}"? This removes it from the registry.`)) return;
    try {
      await invoke('kdb_delete_module', { moduleName: mod.name });
      statusMessage = `Deleted: ${mod.name}`;
      await refresh();
    } catch (e) {
      statusMessage = `Delete failed: ${e}`;
    }
  }
</script>

<div class="model-builder" data-workspace-action="ModelBuilder:Panel">
  <div class="header">
    <h2>🧠 Model Builder</h2>
    <p class="subtitle">Combine a base model with knowledge modules to create a custom AI configuration.</p>
  </div>

  {#if loading}
    <div class="loading-bar" aria-label="Loading…"></div>
  {/if}

  {#if statusMessage}
    <div class="status-banner" role="status">{statusMessage}</div>
  {/if}

  <div class="builder-grid">
    <!-- Column 1: Base Model -->
    <section class="panel">
      <h3>1. Base Model</h3>
      {#if availableModels.length === 0}
        <p class="empty">No local models found. Download a model first.</p>
      {:else}
        <ul class="model-list">
          {#each availableModels as m}
            <li
              class="model-item"
              class:selected={baseModel?.name === m.name}
              on:click={() => { baseModel = m; statusMessage = `Selected: ${m.name}`; }}
              role="option"
              aria-selected={baseModel?.name === m.name}
              tabindex="0"
              on:keydown={(e) => e.key === 'Enter' && (baseModel = m)}
            >
              <span class="model-icon">🤖</span>
              <span class="model-name">{m.name}</span>
              {#if baseModel?.name === m.name}<span class="check">✓</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- Column 2: Knowledge Modules -->
    <section class="panel">
      <h3>2. Knowledge Modules</h3>
      <p class="hint">Toggle modules on/off. Active modules inject relevant context before each response.</p>
      {#if registeredModules.length === 0}
        <p class="empty">No modules registered. Import a .bkp package or build a module.</p>
      {:else}
        <ul class="module-list">
          {#each registeredModules as mod}
            <li class="module-item">
              <div class="module-info">
                <strong>{mod.name}</strong>
                <span class="module-meta">{mod.domain} · {mod.entry_count} entries · v{mod.version}</span>
              </div>
              <div class="module-actions">
                <button
                  class="toggle-btn"
                  class:active={loadedModuleNames.has(mod.name)}
                  on:click={() => toggleModule(mod)}
                  title={loadedModuleNames.has(mod.name) ? 'Unload' : 'Load'}
                >
                  {loadedModuleNames.has(mod.name) ? '● On' : '○ Off'}
                </button>
                <button class="icon-btn delete-btn" on:click={() => deleteModule(mod)} title="Delete">🗑</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- Column 3: Actions & Preview -->
    <section class="panel">
      <h3>3. Actions</h3>
      <div class="action-list">
        <button class="action-btn primary" on:click={testModel} disabled={!baseModel}>
          🧪 Test Configuration
        </button>
        <button class="action-btn" on:click={exportPackage} disabled={!baseModel}>
          📦 Export as .bkp Package
        </button>
        <button class="action-btn" on:click={importPackage}>
          📥 Import .bkp Package
        </button>
        <button class="action-btn secondary" on:click={refresh}>
          🔄 Refresh
        </button>
      </div>

      {#if testChatMessage}
        <div class="test-preview">
          <h4>Context Preview</h4>
          <pre>{testChatMessage}</pre>
        </div>
      {/if}

      {#if loadedModuleNames.size > 0}
        <div class="active-summary">
          <strong>{loadedModuleNames.size} module{loadedModuleNames.size !== 1 ? 's' : ''} active</strong>
          <ul>
            {#each [...loadedModuleNames] as name}
              <li>● {name}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .model-builder {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.25rem;
    height: 100%;
    overflow: auto;
  }

  .header h2 { margin: 0 0 0.25rem; font-size: 1.25rem; }
  .subtitle { margin: 0; color: var(--text-secondary, #888); font-size: 0.875rem; }

  .loading-bar {
    height: 3px;
    background: linear-gradient(90deg, transparent, var(--accent, #3b82f6), transparent);
    background-size: 200% 100%;
    animation: shimmer 1.2s infinite;
    border-radius: 2px;
  }

  @keyframes shimmer { 0% { background-position: -200% 0; } 100% { background-position: 200% 0; } }

  .status-banner {
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    background: var(--bg-secondary, #1e1e2e);
    border-left: 3px solid var(--accent, #3b82f6);
    font-size: 0.875rem;
  }

  .builder-grid {
    display: grid;
    grid-template-columns: 1fr 1.4fr 1fr;
    gap: 1rem;
    flex: 1;
    min-height: 0;
  }

  .panel {
    border: 1px solid var(--border, #333);
    border-radius: 10px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    overflow: hidden;
  }

  .panel h3 { margin: 0; font-size: 0.9rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-secondary, #888); }
  .empty, .hint { color: var(--text-secondary, #888); font-size: 0.82rem; font-style: italic; margin: 0; }

  .model-list, .module-list { list-style: none; margin: 0; padding: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.4rem; }

  .model-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background 0.12s;
    font-size: 0.875rem;
  }
  .model-item:hover { background: var(--bg-hover, #2a2a3a); }
  .model-item.selected { border-color: var(--accent, #3b82f6); background: var(--bg-selected, #1e2a3a); }
  .model-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .check { color: var(--accent, #3b82f6); font-weight: bold; }

  .module-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    border: 1px solid var(--border, #333);
    gap: 0.5rem;
    font-size: 0.875rem;
  }

  .module-info { display: flex; flex-direction: column; gap: 0.1rem; overflow: hidden; }
  .module-info strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .module-meta { font-size: 0.75rem; color: var(--text-secondary, #888); }
  .module-actions { display: flex; gap: 0.35rem; flex-shrink: 0; }

  .toggle-btn {
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    border: 1px solid var(--border, #333);
    background: transparent;
    cursor: pointer;
    font-size: 0.78rem;
    color: var(--text-secondary, #888);
    transition: all 0.12s;
  }
  .toggle-btn.active { border-color: var(--accent, #3b82f6); color: var(--accent, #3b82f6); background: var(--bg-selected, #1e2a3a); }

  .icon-btn { background: transparent; border: none; cursor: pointer; padding: 0.2rem; font-size: 0.9rem; opacity: 0.6; }
  .icon-btn:hover { opacity: 1; }

  .action-list { display: flex; flex-direction: column; gap: 0.5rem; }

  .action-btn {
    padding: 0.6rem 1rem;
    border-radius: 8px;
    border: 1px solid var(--border, #333);
    background: var(--bg-secondary, #1e1e2e);
    cursor: pointer;
    text-align: left;
    font-size: 0.875rem;
    transition: all 0.12s;
  }
  .action-btn:hover:not(:disabled) { background: var(--bg-hover, #2a2a3a); border-color: var(--accent, #3b82f6); }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .action-btn.primary { background: var(--accent, #3b82f6); border-color: var(--accent, #3b82f6); color: white; }
  .action-btn.primary:hover:not(:disabled) { filter: brightness(1.15); }

  .test-preview {
    background: var(--bg-secondary, #1e1e2e);
    border-radius: 6px;
    padding: 0.75rem;
    margin-top: 0.5rem;
  }
  .test-preview h4 { margin: 0 0 0.5rem; font-size: 0.8rem; color: var(--text-secondary, #888); }
  .test-preview pre { margin: 0; font-size: 0.78rem; white-space: pre-wrap; word-break: break-word; max-height: 120px; overflow-y: auto; }

  .active-summary {
    margin-top: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    background: var(--bg-selected, #1e2a3a);
    font-size: 0.82rem;
  }
  .active-summary ul { margin: 0.35rem 0 0; padding: 0 0 0 0.5rem; list-style: none; color: var(--accent, #3b82f6); }

  @media (max-width: 900px) {
    .builder-grid { grid-template-columns: 1fr; }
  }
</style>
