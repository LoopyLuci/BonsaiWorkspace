<script lang="ts">
  import type { CanvasInteractionMode } from '$lib/stores/canvas';

  export let mode: CanvasInteractionMode = 'select';

  export let onMode: (mode: CanvasInteractionMode) => void;
  export let onAddNote: () => void;
  export let onAddChat: () => void;
  export let onOpenSearch: () => void;
  export let onFitView: () => void;
  export let onSave: () => void;
</script>

<div class="toolbar">
  <button class:active={mode === 'select'} on:click={() => onMode('select')} type="button">V Select</button>
  <button class:active={mode === 'pan'} on:click={() => onMode('pan')} type="button">H Pan</button>
  <button class:active={mode === 'connect'} on:click={() => onMode('connect')} type="button">C Connect</button>
  <button on:click={onAddNote} type="button">N Note</button>
  <button on:click={onAddChat} type="button">Chat Node</button>
  <button on:click={onOpenSearch} type="button">Cmd+K File</button>
  <button on:click={onFitView} type="button">Fit View</button>
  <button on:click={onSave} type="button">Save</button>
</div>

<style>
  .toolbar {
    position: absolute;
    top: 16px;
    left: 16px;
    z-index: 20;
    display: flex;
    gap: 6px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg2) 86%, black 14%);
    backdrop-filter: blur(4px);
    flex-wrap: wrap;
    max-width: min(90vw, 780px);
  }

  button {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 10px;
    background: transparent;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
  }

  button:hover {
    color: var(--text);
    border-color: var(--accent);
    background: rgba(255, 255, 255, 0.04);
  }

  button.active {
    color: var(--text);
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 28%, transparent);
  }
</style>
