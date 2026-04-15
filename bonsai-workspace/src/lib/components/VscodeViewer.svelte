<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    vscodeFileTree, vscodeEditor, vscodeDiagnostics, vscodeConnected,
  } from '$lib/stores/vscodeState';
  import { wsClientStart, wsClientStop, sendVscodeCmd } from '$lib/utils/wsClient';
  import { requestOpenFile } from '$lib/stores/openFile';
  import VscodeFileEntryComponent from './VscodeFileEntry.svelte';

  // ── Tab state ────────────────────────────────────────────────────────────────
  let activeTab: 'files' | 'editor' | 'diagnostics' = 'files';

  // ── Token loading ─────────────────────────────────────────────────────────────
  onMount(async () => {
    try {
      const token = await invoke<string>('get_pair_token');
      wsClientStart(token);
    } catch (e) {
      console.error('[VscodeViewer] could not get pair token', e);
      wsClientStart('');
    }
  });

  onDestroy(() => {
    wsClientStop();
  });

  // ── File tree helpers ─────────────────────────────────────────────────────────
  let expandedDirs = new Set<string>();

  function toggleDir(path: string) {
    if (expandedDirs.has(path)) expandedDirs.delete(path);
    else expandedDirs.add(path);
    expandedDirs = expandedDirs; // trigger reactivity
  }

  function openFile(path: string) {
    // Tell the VSCode extension to open this file there too.
    sendVscodeCmd('open_file', { path });
    // Also open it in the local Monaco editor if we have it.
    requestOpenFile(path);
  }

  // ── Diagnostics helpers ───────────────────────────────────────────────────────
  function severityClass(s: string) {
    return s === 'error' ? 'diag-error' : s === 'warning' ? 'diag-warn' : 'diag-info';
  }

  function severityIcon(s: string) {
    return s === 'error' ? '✕' : s === 'warning' ? '⚠' : 'ℹ';
  }
</script>

<div class="vscode-viewer">
  <!-- Header -->
  <div class="viewer-header">
    <span class="viewer-title">VSCode</span>
    <span class="conn-badge" class:connected={$vscodeConnected}>
      {$vscodeConnected ? 'live' : 'waiting'}
    </span>
    <div class="tab-bar">
      <button class:active={activeTab==='files'}       on:click={() => activeTab='files'}>Files</button>
      <button class:active={activeTab==='editor'}      on:click={() => activeTab='editor'}>Editor</button>
      <button class:active={activeTab==='diagnostics'} on:click={() => activeTab='diagnostics'}>
        Diagnostics
        {#if $vscodeDiagnostics.filter(d => d.severity==='error').length > 0}
          <span class="badge-err">{$vscodeDiagnostics.filter(d => d.severity==='error').length}</span>
        {/if}
      </button>
    </div>
  </div>

  <!-- Content -->
  <div class="viewer-content">

    <!-- Files tab -->
    {#if activeTab === 'files'}
      {#if $vscodeFileTree.length === 0}
        <div class="empty-state">
          {#if $vscodeConnected}
            No workspace folder open in VSCode.
          {:else}
            Waiting for VSCode extension to connect…<br/>
            1. Install <strong>Bonsai Workspace Runner</strong> in VSCode.<br/>
            2. Set <code>bonsai.pairToken</code> in VSCode settings.<br/>
            3. Enable auto-connect or run the extension connect command.
          {/if}
        </div>
      {:else}
        <div class="file-tree" role="tree">
          {#each $vscodeFileTree as entry (entry.path)}
            <VscodeFileEntryComponent
              {entry}
              depth={0}
              {expandedDirs}
              {toggleDir}
              {openFile}
            />
          {/each}
        </div>
      {/if}
    {/if}

    <!-- Editor tab -->
    {#if activeTab === 'editor'}
      {#if !$vscodeEditor}
        <div class="empty-state">No file open in VSCode.</div>
      {:else}
        <div class="editor-meta">
          <span class="editor-path" title={$vscodeEditor.path}>
            {$vscodeEditor.path.split(/[/\\]/).pop()}
          </span>
          <span class="editor-lang">{$vscodeEditor.language}</span>
          <span class="editor-cursor">Ln {$vscodeEditor.cursor.line + 1}, Col {$vscodeEditor.cursor.col + 1}</span>
        </div>
        <pre class="editor-content"><code>{$vscodeEditor.content}</code></pre>
      {/if}
    {/if}

    <!-- Diagnostics tab -->
    {#if activeTab === 'diagnostics'}
      {#if $vscodeDiagnostics.length === 0}
        <div class="empty-state">No diagnostics.</div>
      {:else}
        <div class="diag-list">
          {#each $vscodeDiagnostics as d}
            <button
              class="diag-item {severityClass(d.severity)}"
              on:click={() => sendVscodeCmd('cursor_set', { path: d.path, line: d.line, col: d.col })}
              title="Click to jump to this location in VSCode"
            >
              <span class="diag-icon">{severityIcon(d.severity)}</span>
              <div class="diag-body">
                <span class="diag-msg">{d.message}</span>
                <span class="diag-loc">{d.path.split(/[/\\]/).pop()}:{d.line + 1}:{d.col + 1}{d.source ? ` (${d.source})` : ''}</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    {/if}

  </div>
</div>

<style>
  .vscode-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    color: var(--text);
    font-size: 13px;
  }

  .viewer-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px 0;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .viewer-title {
    font-weight: 600;
    font-size: 13px;
    color: var(--accent-hl);
  }

  .conn-badge {
    font-size: 10px;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--bg2);
    color: var(--text-dim);
    border: 1px solid var(--border);
  }
  .conn-badge.connected {
    background: rgba(34,197,94,0.15);
    color: var(--green);
    border-color: rgba(34,197,94,0.3);
  }

  .tab-bar {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }

  .tab-bar button {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    padding: 6px 10px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .tab-bar button.active {
    color: var(--text);
    border-bottom-color: var(--accent-hl);
  }

  .tab-bar button:hover:not(.active) {
    color: var(--text);
    background: var(--bg-hover);
  }

  .badge-err {
    background: var(--red);
    color: #fff;
    border-radius: 999px;
    font-size: 10px;
    padding: 0 5px;
  }

  .viewer-content {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .empty-state {
    padding: 24px 16px;
    color: var(--text-dim);
    font-size: 13px;
    line-height: 1.6;
  }

  /* File tree */
  .file-tree {
    display: flex;
    flex-direction: column;
  }

  /* Editor preview */
  .editor-meta {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    background: var(--bg2);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    font-size: 12px;
  }

  .editor-path { font-weight: 500; }
  .editor-lang, .editor-cursor { color: var(--text-dim); }

  .editor-content {
    flex: 1;
    overflow: auto;
    padding: 12px 16px;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
    tab-size: 2;
    color: var(--text);
    margin: 0;
  }

  /* Diagnostics */
  .diag-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 8px;
  }

  .diag-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid transparent;
    cursor: pointer;
    text-align: left;
    width: 100%;
    background: transparent;
    color: var(--text);
  }

  .diag-item:hover {
    background: var(--bg-hover);
    border-color: var(--border);
  }

  .diag-icon { flex-shrink: 0; font-size: 13px; margin-top: 1px; }
  .diag-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .diag-msg  { font-size: 12px; word-break: break-word; }
  .diag-loc  { font-size: 11px; color: var(--text-dim); }

  .diag-error .diag-icon { color: var(--red); }
  .diag-warn  .diag-icon { color: var(--amber); }
  .diag-info  .diag-icon { color: var(--accent-hl); }
</style>
