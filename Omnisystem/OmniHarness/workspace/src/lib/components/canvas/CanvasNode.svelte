<script lang="ts">
  import type { CanvasNode } from '$lib/stores/canvas';

  export let node: CanvasNode;
  export let selected = false;
  export let canConnect = false;

  export let onSelect: (event: PointerEvent) => void;
  export let onBringFront: () => void;
  export let onDragStart: (event: PointerEvent) => void;
  export let onResizeStart: (event: PointerEvent) => void;
  export let onDelete: () => void;
  export let onStartConnect: (event: PointerEvent) => void;
  export let onCompleteConnect: () => void;
</script>

<div
  class="node"
  class:selected={selected}
  style="left: {node.x}px; top: {node.y}px; width: {node.width}px; height: {node.height}px; z-index: {node.zIndex ?? 1}; --node-color: {node.color ?? '#4a9eff'};"
  on:pointerdown={onBringFront}
>
  <header class="node-header" on:pointerdown={onDragStart}>
    <button class="connect-port" class:visible={canConnect} on:pointerdown|stopPropagation={onStartConnect} on:pointerup|stopPropagation={onCompleteConnect} type="button" title="Start/complete connection" />
    <button class="title-btn" on:pointerdown={onSelect} type="button">{node.title ?? node.type}</button>
    <button class="delete-btn" on:click|stopPropagation={onDelete} type="button" aria-label="Delete node">x</button>
  </header>

  <div class="node-body">
    <slot />
  </div>

  <button class="resize-handle" on:pointerdown={onResizeStart} type="button" aria-label="Resize node" />
</div>

<style>
  .node {
    position: absolute;
    border: 1px solid color-mix(in srgb, var(--node-color) 35%, var(--border));
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg2) 88%, black 12%);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    overflow: hidden;
    min-width: 240px;
    min-height: 140px;
    display: flex;
    flex-direction: column;
  }

  .node.selected {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--node-color) 55%, transparent), 0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(90deg, color-mix(in srgb, var(--node-color) 20%, transparent), transparent);
    cursor: grab;
  }

  .title-btn {
    text-align: left;
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .connect-port {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1px solid var(--node-color);
    background: color-mix(in srgb, var(--node-color) 45%, transparent);
    opacity: 0;
    cursor: crosshair;
  }

  .connect-port.visible {
    opacity: 1;
  }

  .delete-btn {
    border: 1px solid var(--border);
    border-radius: 6px;
    width: 20px;
    height: 20px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
  }

  .delete-btn:hover {
    color: #fda4af;
    border-color: #be123c;
  }

  .node-body {
    flex: 1;
    min-height: 0;
  }

  .resize-handle {
    position: absolute;
    right: 2px;
    bottom: 2px;
    width: 14px;
    height: 14px;
    border: none;
    background: linear-gradient(135deg, transparent 45%, rgba(255, 255, 255, 0.35) 46%);
    cursor: nwse-resize;
  }
</style>
