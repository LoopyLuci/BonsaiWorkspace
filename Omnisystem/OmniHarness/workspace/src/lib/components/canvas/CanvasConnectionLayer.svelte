<script lang="ts">
  import type { CanvasConnection, CanvasNode, PendingConnection } from '$lib/stores/canvas';

  export let nodes: CanvasNode[] = [];
  export let connections: CanvasConnection[] = [];
  export let pending: PendingConnection | null = null;
  export let selectedConnectionId: string | null = null;
  export let onSelectConnection: (connectionId: string | null) => void;
  export let onStartEditLabel: (connectionId: string, x: number, y: number, currentLabel: string) => void;

  function centerRight(node: CanvasNode) {
    return { x: node.x + node.width, y: node.y + node.height / 2 };
  }

  function centerLeft(node: CanvasNode) {
    return { x: node.x, y: node.y + node.height / 2 };
  }

  function pathFor(from: { x: number; y: number }, to: { x: number; y: number }) {
    const dx = Math.max(80, Math.abs(to.x - from.x) * 0.35);
    return `M ${from.x} ${from.y} C ${from.x + dx} ${from.y}, ${to.x - dx} ${to.y}, ${to.x} ${to.y}`;
  }

  function byId(nodeId: string) {
    return nodes.find((node) => node.id === nodeId);
  }

  function midpoint(from: { x: number; y: number }, to: { x: number; y: number }) {
    return { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 };
  }
</script>

<svg class="links" viewBox="-8000 -8000 16000 16000" preserveAspectRatio="none">
  {#each connections as conn (conn.id)}
    {@const fromNode = byId(conn.fromNodeId)}
    {@const toNode = byId(conn.toNodeId)}
    {#if fromNode && toNode}
      {@const from = centerRight(fromNode)}
      {@const to = centerLeft(toNode)}
      <path
        class:selected={selectedConnectionId === conn.id}
        d={pathFor(from, to)}
        on:pointerdown|stopPropagation={() => onSelectConnection(conn.id)}
      />
      {@const mid = midpoint(from, to)}
      <g
        class="label-group"
        role="button"
        tabindex="0"
        on:pointerdown|stopPropagation={() => onSelectConnection(conn.id)}
        on:dblclick|stopPropagation={() => onStartEditLabel(conn.id, mid.x, mid.y, conn.label ?? '')}
        on:keydown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onStartEditLabel(conn.id, mid.x, mid.y, conn.label ?? '');
          }
        }}
      >
        <rect
          x={mid.x - 74}
          y={mid.y - 12}
          width="148"
          height="22"
          rx="6"
        />
        <text x={mid.x} y={mid.y + 4} dominant-baseline="middle">
          {(conn.label ?? '').trim() || 'Double-click to label'}
        </text>
      </g>
    {/if}
  {/each}

  {#if pending}
    {@const source = byId(pending.fromNodeId)}
    {#if source}
      {@const from = centerRight(source)}
      <path class="pending" d={pathFor(from, { x: pending.x, y: pending.y })} />
    {/if}
  {/if}
</svg>

<style>
  .links {
    position: absolute;
    inset: -4000px;
    pointer-events: none;
    overflow: visible;
  }

  path {
    fill: none;
    stroke: rgba(74, 158, 255, 0.72);
    stroke-width: 2;
    filter: drop-shadow(0 0 4px rgba(74, 158, 255, 0.3));
  }

  path.pending {
    stroke-dasharray: 7 6;
    animation: dash 0.75s linear infinite;
  }

  path.selected {
    stroke: rgba(251, 191, 36, 0.9);
    stroke-width: 3;
    filter: drop-shadow(0 0 6px rgba(251, 191, 36, 0.35));
  }

  .label-group {
    pointer-events: auto;
    cursor: text;
  }

  .label-group rect {
    fill: rgba(15, 23, 42, 0.75);
    stroke: rgba(148, 163, 184, 0.5);
    stroke-width: 1;
  }

  .label-group text {
    fill: rgba(226, 232, 240, 0.9);
    font-size: 11px;
    text-anchor: middle;
    /* dominant-baseline applied inline on <text> elements (CSS linter doesn't recognize SVG props) */
    pointer-events: none;
  }

  @keyframes dash {
    to {
      stroke-dashoffset: -26;
    }
  }
</style>
