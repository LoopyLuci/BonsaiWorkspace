<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, createEventDispatcher } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  const dispatch = createEventDispatcher<{ close: void }>();

  interface SylvaScript {
    name: string;
    path: string;
    source: string;
    loaded_at: number;
    error: string | null;
  }

  interface SylvaCallRecord {
    script_name: string;
    args: unknown;
    result: unknown;
    elapsed_ms: number;
    timestamp: number;
  }

  let scripts: SylvaScript[] = [];
  let selectedName: string | null = null;
  let editorContent = '';
  let output = '';
  let outputError = false;
  let history: SylvaCallRecord[] = [];
  let showHistory = false;
  let loading = false;
  let saving = false;
  let newScriptName = '';
  let showNewForm = false;

  const STARTER = `-- Sylva script
-- Use bonsai.tool(name, args) to call any registered tool.
-- Define a run(args) function to expose this as a UCR tool.

function run(args)
  bonsai.log("Hello from Sylva!")
  return { message = "ok", input = args }
end

-- Runs immediately when you press Run:
local result = run({ test = true })
bonsai.log(bonsai.json_encode(result))
`;

  onMount(async () => {
    await loadScripts();
    await loadHistory();
    const unlisten = await listen('sylva-reload', async () => { await loadScripts(); });
    return unlisten;
  });

  async function loadScripts() {
    try {
      scripts = await invoke<SylvaScript[]>('sylva_list_scripts');
    } catch (e) {
      console.error('sylva_list_scripts:', e);
    }
  }

  async function loadHistory() {
    try {
      history = await invoke<SylvaCallRecord[]>('get_sylva_history');
    } catch (e) { history = []; }
  }

  async function selectScript(name: string) {
    selectedName = name;
    try {
      editorContent = await invoke<string>('sylva_get_script_content', { name });
      output = ''; outputError = false;
    } catch (e) {
      editorContent = `-- could not load ${name}: ${e}`;
    }
  }

  async function runScript() {
    if (!editorContent.trim()) return;
    loading = true; output = ''; outputError = false;
    try {
      const result = await invoke<unknown>('sylva_exec', { src: editorContent });
      output = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
    } catch (e: unknown) {
      output = String(e); outputError = true;
    } finally {
      loading = false;
      await loadHistory();
    }
  }

  async function saveScript() {
    if (!selectedName) return;
    saving = true;
    try {
      await invoke<string>('sylva_save_script', { name: selectedName, source: editorContent });
      await loadScripts();
      output = `✓ Saved '${selectedName}'`; outputError = false;
    } catch (e) {
      output = `Save failed: ${e}`; outputError = true;
    } finally { saving = false; }
  }

  async function createScript() {
    const name = newScriptName.trim().replace(/\.lua$/, '');
    if (!name) return;
    try {
      await invoke<string>('sylva_save_script', { name, source: STARTER });
      newScriptName = ''; showNewForm = false;
      await loadScripts();
      await selectScript(name);
    } catch (e) { output = `Create failed: ${e}`; outputError = true; }
  }

  async function clearHistory() {
    await invoke('sylva_clear_history');
    history = [];
  }

  function formatTime(ts: number): string {
    return new Date(ts).toLocaleTimeString();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); runScript(); }
    if ((e.ctrlKey || e.metaKey) && e.key === 's') { e.preventDefault(); if (selectedName) saveScript(); }
  }
</script>

<div class="se-root" role="dialog" aria-modal="true" aria-label="Sylva Script Editor">
  <div class="se-header">
    <span class="se-title">🧪 Sylva Script Editor</span>
    <button class="se-btn se-btn-ghost" on:click={() => dispatch('close')}>✕ Close</button>
  </div>

  <div class="se-body">
    <aside class="se-sidebar">
      <div class="se-sidebar-header">
        <span class="se-sidebar-label">Scripts</span>
        <button class="se-btn se-btn-xs" on:click={() => (showNewForm = !showNewForm)}>+</button>
      </div>

      {#if showNewForm}
        <div class="se-new-form">
          <input class="se-input" bind:value={newScriptName} placeholder="script-name"
            on:keydown={(e) => e.key === 'Enter' && createScript()} />
          <button class="se-btn se-btn-primary se-btn-xs" on:click={createScript}>Create</button>
        </div>
      {/if}

      <ul class="se-script-list">
        {#each scripts as script}
          <li class="se-script-item" class:selected={script.name === selectedName}
            class:has-error={!!script.error}
            role="button" tabindex="0"
            on:click={() => selectScript(script.name)}
            on:keydown={(e) => e.key === 'Enter' && selectScript(script.name)}>
            <span class="se-script-name">{script.name}</span>
            {#if script.error}<span class="se-error-dot" title={script.error}>⚠</span>{/if}
          </li>
        {/each}
        {#if scripts.length === 0}
          <li class="se-empty-hint">No scripts yet — click + to create one.</li>
        {/if}
      </ul>

      <div class="se-sidebar-footer">
        <button class="se-btn se-btn-ghost se-btn-xs" on:click={loadScripts}>↺ Refresh</button>
      </div>
    </aside>

    <div class="se-main">
      <div class="se-editor-bar">
        {#if selectedName}
          <span class="se-filename">{selectedName}.lua</span>
        {:else}
          <span class="se-filename se-placeholder">— no script selected —</span>
        {/if}
        <div class="se-editor-actions">
          <span class="se-hint">Ctrl+Enter run · Ctrl+S save</span>
          {#if selectedName}
            <button class="se-btn se-btn-ghost se-btn-sm" on:click={saveScript} disabled={saving}>
              {saving ? 'Saving…' : '💾 Save'}
            </button>
          {/if}
          <button class="se-btn se-btn-primary se-btn-sm" on:click={runScript} disabled={loading}>
            {loading ? '⏳ Running…' : '▶ Run'}
          </button>
        </div>
      </div>

      <textarea class="se-editor" bind:value={editorContent} placeholder={STARTER}
        spellcheck={false} on:keydown={handleKeyDown}></textarea>

      <div class="se-output-bar">
        <span class="se-output-label">Output</span>
        {#if output}<button class="se-btn se-btn-ghost se-btn-xs" on:click={() => { output = ''; }}>Clear</button>{/if}
      </div>
      <pre class="se-output" class:se-output-error={outputError}>{output || '—'}</pre>
    </div>
  </div>

  <div class="se-history-bar">
    <button class="se-btn se-btn-ghost se-btn-sm"
      on:click={() => { showHistory = !showHistory; loadHistory(); }}>
      📜 Call History ({history.length}) {showHistory ? '▲' : '▼'}
    </button>
    {#if history.length > 0}
      <button class="se-btn se-btn-ghost se-btn-xs" on:click={clearHistory}>Clear</button>
    {/if}
  </div>

  {#if showHistory}
    <div class="se-history">
      {#each [...history].reverse() as h}
        <div class="se-history-row">
          <span class="se-history-time">{formatTime(h.timestamp)}</span>
          <span class="se-history-name">{h.script_name || 'repl'}</span>
          <span class="se-history-ms">{h.elapsed_ms}ms</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .se-root {
    display: flex; flex-direction: column;
    background: #0f1117; border: 1px solid #2a2d3a; border-radius: 10px;
    overflow: hidden; width: 900px; max-width: 95vw; max-height: 85vh;
    font-size: 13px; color: #d4d4d4; box-shadow: 0 8px 40px rgba(0,0,0,.6);
  }
  .se-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 16px; background: #161822; border-bottom: 1px solid #2a2d3a; flex-shrink: 0;
  }
  .se-title { font-weight: 700; font-size: 14px; color: #e0e0e0; }
  .se-body { display: flex; flex: 1; overflow: hidden; min-height: 0; }

  .se-sidebar {
    width: 180px; flex-shrink: 0; display: flex; flex-direction: column;
    border-right: 1px solid #2a2d3a; background: #0d0f18; overflow: hidden;
  }
  .se-sidebar-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 10px; border-bottom: 1px solid #2a2d3a;
  }
  .se-sidebar-label { font-size: 11px; text-transform: uppercase; color: #888; letter-spacing: .06em; }
  .se-new-form { display: flex; gap: 4px; padding: 6px 8px; border-bottom: 1px solid #2a2d3a; }
  .se-script-list { list-style: none; margin: 0; padding: 4px 0; overflow-y: auto; flex: 1; }
  .se-script-item {
    padding: 6px 10px; cursor: pointer; display: flex; align-items: center;
    justify-content: space-between; border-radius: 4px; margin: 1px 4px; transition: background .1s;
  }
  .se-script-item:hover { background: #1e2030; }
  .se-script-item.selected { background: #1e3a5f; color: #7ec8e3; }
  .se-script-item.has-error .se-script-name { color: #e57373; }
  .se-script-name { font-family: monospace; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .se-error-dot { color: #f59e0b; font-size: 10px; margin-left: 4px; }
  .se-empty-hint { padding: 10px; color: #555; font-size: 11px; text-align: center; }
  .se-sidebar-footer { padding: 6px 8px; border-top: 1px solid #2a2d3a; }

  .se-main { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0; }
  .se-editor-bar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 6px 12px; border-bottom: 1px solid #2a2d3a; background: #13151f; flex-shrink: 0;
  }
  .se-filename { font-family: monospace; font-size: 12px; color: #9ca3af; }
  .se-placeholder { color: #444; }
  .se-editor-actions { display: flex; align-items: center; gap: 8px; }
  .se-hint { font-size: 10px; color: #4b5563; }

  .se-editor {
    flex: 2; min-height: 0; background: #0a0c14; color: #d4d4d4;
    font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace;
    font-size: 13px; line-height: 1.6; padding: 12px;
    border: none; resize: none; outline: none; tab-size: 2;
  }
  .se-editor:focus { background: #0b0d15; }

  .se-output-bar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 4px 12px; border-top: 1px solid #2a2d3a; border-bottom: 1px solid #2a2d3a;
    background: #13151f; flex-shrink: 0;
  }
  .se-output-label { font-size: 10px; text-transform: uppercase; color: #888; letter-spacing: .06em; }
  .se-output {
    flex: 1; min-height: 60px; max-height: 140px; overflow-y: auto;
    margin: 0; padding: 10px 12px; background: #080a10;
    color: #a3e635; font-family: monospace; font-size: 12px; white-space: pre-wrap; word-break: break-all;
  }
  .se-output-error { color: #f87171; }

  .se-history-bar {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 12px; border-top: 1px solid #2a2d3a; background: #0d0f18; flex-shrink: 0;
  }
  .se-history { max-height: 120px; overflow-y: auto; background: #080a10; border-top: 1px solid #2a2d3a; flex-shrink: 0; }
  .se-history-row {
    display: flex; gap: 10px; padding: 4px 12px;
    font-size: 11px; border-bottom: 1px solid #1a1c28;
  }
  .se-history-time { color: #6b7280; min-width: 70px; }
  .se-history-name { font-family: monospace; color: #93c5fd; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .se-history-ms { color: #6b7280; min-width: 45px; text-align: right; }

  .se-btn {
    border: none; border-radius: 5px; cursor: pointer;
    font-size: 12px; font-weight: 500; padding: 4px 10px; transition: background .15s;
  }
  .se-btn:disabled { opacity: .5; cursor: not-allowed; }
  .se-btn-primary { background: #2563eb; color: #fff; }
  .se-btn-primary:hover:not(:disabled) { background: #1d4ed8; }
  .se-btn-ghost { background: transparent; color: #9ca3af; border: 1px solid #374151; }
  .se-btn-ghost:hover:not(:disabled) { background: #1f2937; color: #e5e7eb; }
  .se-btn-sm { padding: 4px 10px; font-size: 12px; }
  .se-btn-xs { padding: 2px 7px; font-size: 11px; }
  .se-input {
    flex: 1; background: #1a1c28; border: 1px solid #374151; border-radius: 4px;
    color: #d4d4d4; font-size: 12px; padding: 3px 7px; outline: none;
  }
  .se-input:focus { border-color: #2563eb; }
</style>
