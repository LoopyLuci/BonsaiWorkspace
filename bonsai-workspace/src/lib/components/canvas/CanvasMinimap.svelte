<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { CanvasNode, CanvasViewport } from '$lib/stores/canvas';

  export let nodes: CanvasNode[] = [];
  export let viewport: CanvasViewport = { x: 0, y: 0, zoom: 1 };
  export let viewportWidth = 800;
  export let viewportHeight = 600;
  export let onCenter: (worldX: number, worldY: number) => void;
  export let onPanTo: (worldX: number, worldY: number) => void;

  let dragging = false;
  let minimapEl: HTMLDivElement | null = null;

  const miniWidth = 220;
  const miniHeight = 140;

  $: worldBounds = (() => {
    if (!nodes.length) return { minX: -600, minY: -400, maxX: 600, maxY: 400 };
    const minX = Math.min(...nodes.map((n) => n.x));
    const minY = Math.min(...nodes.map((n) => n.y));
    const maxX = Math.max(...nodes.map((n) => n.x + n.width));
    const maxY = Math.max(...nodes.map((n) => n.y + n.height));
    const pad = 120;
    return { minX: minX - pad, minY: minY - pad, maxX: maxX + pad, maxY: maxY + pad };
  })();

  $: worldWidth = Math.max(1, worldBounds.maxX - worldBounds.minX);
  $: worldHeight = Math.max(1, worldBounds.maxY - worldBounds.minY);

  function toMiniX(worldX: number) {
    return ((worldX - worldBounds.minX) / worldWidth) * miniWidth;
  }

  function toMiniY(worldY: number) {
    return ((worldY - worldBounds.minY) / worldHeight) * miniHeight;
  }

  $: viewWorldLeft = (-viewport.x) / viewport.zoom;
  $: viewWorldTop = (-viewport.y) / viewport.zoom;
  $: viewWorldWidth = viewportWidth / viewport.zoom;
  $: viewWorldHeight = viewportHeight / viewport.zoom;

  function handleClick(event: MouseEvent) {
    const target = event.currentTarget as HTMLDivElement;
    const rect = target.getBoundingClientRect();
    const mx = event.clientX - rect.left;
    const my = event.clientY - rect.top;
    const worldX = worldBounds.minX + (mx / miniWidth) * worldWidth;
    const worldY = worldBounds.minY + (my / miniHeight) * worldHeight;
    onCenter(worldX, worldY);
  }

  function panFromPointer(clientX: number, clientY: number) {
    if (!minimapEl) return;
    const rect = minimapEl.getBoundingClientRect();
    const mx = Math.min(rect.width, Math.max(0, clientX - rect.left));
    const my = Math.min(rect.height, Math.max(0, clientY - rect.top));
    const worldX = worldBounds.minX + (mx / miniWidth) * worldWidth;
    const worldY = worldBounds.minY + (my / miniHeight) * worldHeight;
    onPanTo(worldX, worldY);
  }

  function onPointerDown(event: PointerEvent) {
    dragging = true;
    panFromPointer(event.clientX, event.clientY);
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging) return;
    panFromPointer(event.clientX, event.clientY);
  }

  function onPointerUp() {
    dragging = false;
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);
  }

  onDestroy(() => {
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);
  });

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const target = event.currentTarget as HTMLDivElement;
      const rect = target.getBoundingClientRect();
      const mx = rect.width / 2;
      const my = rect.height / 2;
      const worldX = worldBounds.minX + (mx / miniWidth) * worldWidth;
      const worldY = worldBounds.minY + (my / miniHeight) * worldHeight;
      onCenter(worldX, worldY);
    }
  }
</script>

<div bind:this={minimapEl} class="minimap" role="button" tabindex="0" on:click={handleClick} on:keydown={onKeyDown} on:pointerdown={onPointerDown}>
  <svg viewBox={`0 0 ${miniWidth} ${miniHeight}`}>
    {#each nodes as node (node.id)}
      <rect
        x={toMiniX(node.x)}
        y={toMiniY(node.y)}
        width={Math.max(3, (node.width / worldWidth) * miniWidth)}
        height={Math.max(2, (node.height / worldHeight) * miniHeight)}
      />
    {/each}
    <rect
      class="viewport"
      x={toMiniX(viewWorldLeft)}
      y={toMiniY(viewWorldTop)}
      width={Math.max(8, (viewWorldWidth / worldWidth) * miniWidth)}
      height={Math.max(8, (viewWorldHeight / worldHeight) * miniHeight)}
    />
  </svg>
</div>

<style>
  .minimap {
    position: absolute;
    right: 16px;
    bottom: 16px;
    z-index: 20;
    width: 220px;
    height: 140px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg2) 85%, black 15%);
    overflow: hidden;
    cursor: pointer;
  }

  svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  rect {
    fill: rgba(74, 158, 255, 0.5);
    stroke: rgba(74, 158, 255, 0.8);
    stroke-width: 0.5;
  }

  rect.viewport {
    fill: transparent;
    stroke: rgba(255, 255, 255, 0.85);
    stroke-dasharray: 4 3;
    stroke-width: 1;
  }
</style>
