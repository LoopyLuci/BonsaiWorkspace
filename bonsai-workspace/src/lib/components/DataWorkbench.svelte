<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();

  type Tab = 'sql' | 'apl';
  let activeTab: Tab = 'sql';

  let sqlQuery = 'SELECT 1 + 1 AS answer;';
  let aplExpr  = '1 + 1 2 3';

  let running = false;
  let result: unknown = null;
  let error = '';

  async function runSql() {
    running = true; error = ''; result = null;
    try {
      result = await invoke('rpc', { method: 'data.execute_sql', params: { query: sqlQuery } });
    } catch (e) { error = String(e); }
    finally { running = false; }
  }

  async function runApl() {
    running = true; error = ''; result = null;
    try {
      result = await invoke('rpc', { method: 'data.eval_apl', params: { expr: aplExpr } });
    } catch (e) { error = String(e); }
    finally { running = false; }
  }

  function run() { activeTab === 'sql' ? runSql() : runApl(); }

  function formatResult(r: unknown): string {
    if (r == null) return '';
    if (typeof r === 'string') return r;
    return JSON.stringify(r, null, 2);
  }
</script>

<div class="panel">
  <div class="panel-header">
    <h2>Data Workbench</h2>
    <button class="close-btn" on:click={() => dispatch('close')}>✕</button>
  </div>

  <div class="tabs">
    <button class="tab" class:active={activeTab === 'sql'} on:click={() => activeTab = 'sql'}>SQL</button>
    <button class="tab" class:active={activeTab === 'apl'} on:click={() => activeTab = 'apl'}>APL / Array</button>
  </div>

  <div class="body">
    {#if activeTab === 'sql'}
      <textarea class="code-input" bind:value={sqlQuery} rows="6" spellcheck="false" placeholder="SELECT …" />
    {:else}
      <textarea class="code-input" bind:value={aplExpr} rows="6" spellcheck="false" placeholder="APL expression…" />
    {/if}

    <button class="run-btn" on:click={run} disabled={running}>
      {running ? 'Running…' : 'Run (Shift+Enter)'}
    </button>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    {#if result != null}
      <pre class="result">{formatResult(result)}</pre>
    {/if}
  </div>
</div>

<style>
  .panel {
    position: fixed; right: 0; top: 44px; bottom: 0; width: 520px;
    background: #18181b; border-left: 1px solid #3f3f46;
    display: flex; flex-direction: column; z-index: 500;
    font-size: 13px; color: #e4e4e7;
  }
  .panel-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px; border-bottom: 1px solid #3f3f46; flex-shrink: 0;
  }
  h2 { font-size: 14px; font-weight: 600; color: #fff; margin: 0; }
  .close-btn {
    background: none; border: none; color: #71717a; cursor: pointer;
    font-size: 16px; line-height: 1; padding: 2px 4px;
  }
  .close-btn:hover { color: #e4e4e7; }
  .tabs { display: flex; border-bottom: 1px solid #3f3f46; flex-shrink: 0; }
  .tab {
    padding: 8px 16px; background: none; border: none; cursor: pointer;
    color: #71717a; font-size: 12px; font-weight: 500;
    border-bottom: 2px solid transparent; margin-bottom: -1px;
  }
  .tab:hover { color: #e4e4e7; }
  .tab.active { color: #a5b4fc; border-bottom-color: #6366f1; }
  .body {
    flex: 1; overflow-y: auto; padding: 12px 16px;
    display: flex; flex-direction: column; gap: 10px;
  }
  .code-input {
    width: 100%; padding: 8px; background: #0f0f12; border: 1px solid #3f3f46;
    border-radius: 6px; color: #e4e4e7; font-family: monospace; font-size: 12px;
    resize: vertical; outline: none;
  }
  .code-input:focus { border-color: #6366f1; }
  .run-btn {
    align-self: flex-start; padding: 6px 16px; border-radius: 6px;
    background: #4f46e5; border: none; color: #fff; cursor: pointer; font-size: 12px;
  }
  .run-btn:hover { background: #4338ca; }
  .run-btn:disabled { opacity: 0.5; cursor: default; }
  .error {
    padding: 8px; background: #450a0a; border: 1px solid #b91c1c;
    border-radius: 6px; color: #fca5a5; font-size: 12px;
  }
  .result {
    padding: 10px; background: #0f0f12; border: 1px solid #3f3f46;
    border-radius: 6px; color: #a3e635; font-family: monospace; font-size: 12px;
    white-space: pre-wrap; word-break: break-all; overflow-x: auto;
    max-height: 400px; overflow-y: auto;
  }
</style>
