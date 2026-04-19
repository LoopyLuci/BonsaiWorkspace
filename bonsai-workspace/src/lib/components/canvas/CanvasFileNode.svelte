<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onDestroy, onMount } from 'svelte';
  import { createEditor, setLanguageFromPath } from '$lib/utils/monaco';
  import { clearCanvasFileConflict, markCanvasFileConflict } from '$lib/stores/canvas';

  export let filePath = '';
  export let zoom = 1;

  let containerRef: HTMLDivElement | null = null;
  let editor: any = null;
  let currentContent = '';
  let lastDiskContent = '';
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let loading = true;
  let loadError = '';
  let isConflict = false;
  let conflictMessage = '';

  async function loadFromDisk() {
    if (!filePath) return;
    loading = true;
    loadError = '';
    try {
      const content = await invoke<string>('read_file', { path: filePath });
      currentContent = content ?? '';
      lastDiskContent = currentContent;
      if (editor) {
        editor.setValue(currentContent);
      }
      isConflict = false;
      conflictMessage = '';
      clearCanvasFileConflict(filePath);
    } catch (error) {
      loadError = String(error);
    } finally {
      loading = false;
    }
  }

  async function overwriteDisk() {
    if (!filePath) return;
    try {
      await invoke('write_file', { path: filePath, content: currentContent });
      lastDiskContent = currentContent;
      isConflict = false;
      conflictMessage = '';
      clearCanvasFileConflict(filePath);
    } catch (error) {
      loadError = String(error);
    }
  }

  async function saveIfSafe() {
    if (!filePath || isConflict) return;
    try {
      const disk = await invoke<string>('read_file', { path: filePath });
      if (disk !== lastDiskContent) {
        isConflict = true;
        conflictMessage = 'File changed on disk since this node loaded.';
        markCanvasFileConflict(filePath, 'conflict', conflictMessage);
        return;
      }
      await invoke('write_file', { path: filePath, content: currentContent });
      lastDiskContent = currentContent;
      clearCanvasFileConflict(filePath);
    } catch (error) {
      loadError = String(error);
    }
  }

  function queueAutosave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void saveIfSafe();
    }, 1000);
  }

  function destroyEditor() {
    if (editor) {
      editor.dispose();
      editor = null;
    }
  }

  function ensureEditor() {
    if (!containerRef || editor || zoom <= 0.6) return;
    editor = createEditor(containerRef, currentContent, 'vs-dark', { scope: 'canvas' });
    setLanguageFromPath(editor, filePath);
    editor.onDidChangeModelContent(() => {
      currentContent = editor.getValue();
      queueAutosave();
    });
  }

  $: if (zoom > 0.6) {
    ensureEditor();
  } else {
    destroyEditor();
  }

  onMount(() => {
    void loadFromDisk().then(() => {
      ensureEditor();
    });
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    destroyEditor();
  });
</script>

{#if loading}
  <div class="state">Loading file...</div>
{:else if loadError}
  <div class="state error">{loadError}</div>
{:else if zoom <= 0.6}
  <pre>{currentContent.slice(0, 1200)}</pre>
{:else}
  <div class="editor" bind:this={containerRef}></div>
{/if}

{#if isConflict}
  <div class="conflict">
    <p>{conflictMessage}</p>
    <button on:click={() => void loadFromDisk()} type="button">Reload Disk</button>
    <button on:click={() => void overwriteDisk()} type="button">Overwrite Disk</button>
  </div>
{/if}

<style>
  .editor {
    width: 100%;
    height: 100%;
  }

  pre {
    width: 100%;
    height: 100%;
    overflow: auto;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    line-height: 1.5;
    margin: 0;
    padding: 10px;
    color: var(--text-dim);
    white-space: pre-wrap;
  }

  .state {
    padding: 10px;
    color: var(--text-dim);
    font-size: 12px;
  }

  .state.error {
    color: #fca5a5;
  }

  .conflict {
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 10px;
    border: 1px solid #be123c;
    border-radius: 10px;
    background: rgba(127, 29, 29, 0.92);
    padding: 10px;
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    z-index: 2;
  }

  .conflict p {
    margin: 0;
    color: #fee2e2;
    font-size: 11px;
    flex: 1;
  }

  .conflict button {
    border: 1px solid rgba(255, 255, 255, 0.45);
    border-radius: 7px;
    background: transparent;
    color: #fff;
    font-size: 11px;
    padding: 6px 8px;
    cursor: pointer;
  }
</style>
