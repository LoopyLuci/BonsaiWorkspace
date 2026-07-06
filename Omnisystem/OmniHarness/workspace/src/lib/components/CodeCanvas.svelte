<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { get } from 'svelte/store';
  import { messages } from '$lib/stores/chat';
  import { currentWorkspace } from '$lib/stores/workspace';
  import {
    addChatNode,
    addFileNode,
    addStickyNote,
    beginCanvasGesture,
    beginConnectionDrag,
    bringNodeToFront,
    cancelCanvasTransientMode,
    canvasCorruptionNotice,
    canvasLayout,
    canvasMode,
    completePendingConnection,
    clearPendingConnection,
    deleteConnection,
    deleteNode,
    deleteSelectedNodes,
    endCanvasGesture,
    fitCanvasView,
    loadCanvasLayout,
    pendingConnection,
    saveCanvas,
    selectedNodeIds,
    setCanvasMode,
    setSelectedNode,
    setViewport,
    updateNode,
    updateConnection,
    updatePendingConnection,
    type CanvasInteractionMode,
    type CanvasNode,
  } from '$lib/stores/canvas';
  import CanvasGrid from '$lib/components/canvas/CanvasGrid.svelte';
  import CanvasToolbar from '$lib/components/canvas/CanvasToolbar.svelte';
  import CanvasNodeShell from '$lib/components/canvas/CanvasNode.svelte';
  import CanvasStickyNote from '$lib/components/canvas/CanvasStickyNote.svelte';
  import CanvasChatNode from '$lib/components/canvas/CanvasChatNode.svelte';
  import CanvasFileNode from '$lib/components/canvas/CanvasFileNode.svelte';
  import CanvasConnectionLayer from '$lib/components/canvas/CanvasConnectionLayer.svelte';
  import CanvasMinimap from '$lib/components/canvas/CanvasMinimap.svelte';
  import CanvasNodeSearch from '$lib/components/canvas/CanvasNodeSearch.svelte';

  export let onClose: () => void;

  interface SearchFile {
    path: string;
    rel: string;
    name: string;
    is_dir: boolean;
  }

  let viewportRef: HTMLDivElement | null = null;
  let showSearch = false;
  let searchQuery = '';
  let searchableFiles: SearchFile[] = [];
  let selectedConnectionId: string | null = null;
  let labelEditor: { id: string; x: number; y: number; value: string } | null = null;

  let dragState: { nodeId: string; startX: number; startY: number; baseX: number; baseY: number } | null = null;
  let resizeState: { nodeId: string; startX: number; startY: number; baseW: number; baseH: number } | null = null;
  let panState: { startX: number; startY: number; baseX: number; baseY: number } | null = null;

  async function loadFilesIndex() {
    const ws = get(currentWorkspace);
    if (!ws?.path) return;
    try {
      const entries = await invoke<SearchFile[]>('list_project_files', { workspacePath: ws.path });
      searchableFiles = entries ?? [];
    } catch {
      searchableFiles = [];
    }
  }

  async function loadCanvas() {
    const ws = get(currentWorkspace);
    if (!ws?.path) return;
    try {
      await loadCanvasLayout(ws.path);
    } catch {
      // no-op
    }
  }

  async function saveCurrentCanvas() {
    const ws = get(currentWorkspace);
    if (!ws?.path) return;
    try {
      await saveCanvas(ws.path);
    } catch {
      // no-op
    }
  }

  function viewportCenterWorld() {
    const layout = get(canvasLayout);
    const rect = viewportRef?.getBoundingClientRect();
    const width = rect?.width ?? 1000;
    const height = rect?.height ?? 700;
    return {
      x: (width / 2 - layout.viewport.x) / layout.viewport.zoom,
      y: (height / 2 - layout.viewport.y) / layout.viewport.zoom,
    };
  }

  function openSearch() {
    showSearch = true;
    if (!searchableFiles.length) {
      void loadFilesIndex();
    }
  }

  function addNoteAtCenter() {
    const center = viewportCenterWorld();
    addStickyNote(center.x - 140, center.y - 80);
  }

  function addChatAtCenter() {
    const history = get(messages);
    const latestAssistant = [...history].reverse().find((msg) => msg.role === 'assistant');
    if (!latestAssistant?.content) return;
    const center = viewportCenterWorld();
    addChatNode(latestAssistant.content, center.x - 180, center.y - 100);
  }

  function onMode(mode: CanvasInteractionMode) {
    setCanvasMode(mode);
  }

  function worldFromClient(clientX: number, clientY: number) {
    const rect = viewportRef?.getBoundingClientRect();
    const layout = get(canvasLayout);
    const px = clientX - (rect?.left ?? 0);
    const py = clientY - (rect?.top ?? 0);
    return {
      x: (px - layout.viewport.x) / layout.viewport.zoom,
      y: (py - layout.viewport.y) / layout.viewport.zoom,
    };
  }

  function onViewportWheel(event: WheelEvent) {
    event.preventDefault();
    const layout = get(canvasLayout);
    const rect = viewportRef?.getBoundingClientRect();
    const localX = event.clientX - (rect?.left ?? 0);
    const localY = event.clientY - (rect?.top ?? 0);

    const factor = event.deltaY < 0 ? 1.1 : 0.9;
    const nextZoom = Math.max(0.08, Math.min(4, layout.viewport.zoom * factor));

    const worldX = (localX - layout.viewport.x) / layout.viewport.zoom;
    const worldY = (localY - layout.viewport.y) / layout.viewport.zoom;
    const x = localX - worldX * nextZoom;
    const y = localY - worldY * nextZoom;

    setViewport({ x, y, zoom: nextZoom });
  }

  function onBackgroundPointerDown(event: PointerEvent) {
    if ((event.target as HTMLElement).closest('.node')) return;
    selectedConnectionId = null;
    labelEditor = null;
    const mode = get(canvasMode);
    if (mode !== 'pan') {
      setSelectedNode(null);
      return;
    }

    if (!beginCanvasGesture('pan')) return;
    const layout = get(canvasLayout);
    panState = {
      startX: event.clientX,
      startY: event.clientY,
      baseX: layout.viewport.x,
      baseY: layout.viewport.y,
    };
    window.addEventListener('pointermove', onGlobalPointerMove);
    window.addEventListener('pointerup', onGlobalPointerUp);
  }

  function startNodeDrag(node: CanvasNode, event: PointerEvent) {
    event.preventDefault();
    if (!beginCanvasGesture('select')) return;
    dragState = {
      nodeId: node.id,
      startX: event.clientX,
      startY: event.clientY,
      baseX: node.x,
      baseY: node.y,
    };
    window.addEventListener('pointermove', onGlobalPointerMove);
    window.addEventListener('pointerup', onGlobalPointerUp);
  }

  function startNodeResize(node: CanvasNode, event: PointerEvent) {
    event.preventDefault();
    if (!beginCanvasGesture('resizing')) return;
    resizeState = {
      nodeId: node.id,
      startX: event.clientX,
      startY: event.clientY,
      baseW: node.width,
      baseH: node.height,
    };
    window.addEventListener('pointermove', onGlobalPointerMove);
    window.addEventListener('pointerup', onGlobalPointerUp);
  }

  function startConnect(node: CanvasNode, event: PointerEvent) {
    event.preventDefault();
    const mode = get(canvasMode);
    if (mode !== 'connect') return;
    const layout = get(canvasLayout);
    const x = node.x + node.width;
    const y = node.y + node.height / 2;
    beginConnectionDrag(node.id, x, y);
    updatePendingConnection(x, y);
    window.addEventListener('pointermove', onGlobalPointerMove);
    window.addEventListener('pointerup', onGlobalPointerUp);
  }

  function onGlobalPointerMove(event: PointerEvent) {
    const layout = get(canvasLayout);

    if (dragState) {
      const dx = (event.clientX - dragState.startX) / layout.viewport.zoom;
      const dy = (event.clientY - dragState.startY) / layout.viewport.zoom;
      updateNode(dragState.nodeId, {
        x: dragState.baseX + dx,
        y: dragState.baseY + dy,
      });
      return;
    }

    if (resizeState) {
      const dx = (event.clientX - resizeState.startX) / layout.viewport.zoom;
      const dy = (event.clientY - resizeState.startY) / layout.viewport.zoom;
      updateNode(resizeState.nodeId, {
        width: Math.max(240, resizeState.baseW + dx),
        height: Math.max(140, resizeState.baseH + dy),
      });
      return;
    }

    if (panState) {
      const dx = event.clientX - panState.startX;
      const dy = event.clientY - panState.startY;
      setViewport({
        ...layout.viewport,
        x: panState.baseX + dx,
        y: panState.baseY + dy,
      });
      return;
    }

    if (get(pendingConnection)) {
      const world = worldFromClient(event.clientX, event.clientY);
      updatePendingConnection(world.x, world.y);
    }
  }

  function onGlobalPointerUp() {
    if (get(pendingConnection)) {
      clearPendingConnection();
    }
    dragState = null;
    resizeState = null;
    panState = null;
    endCanvasGesture();
    window.removeEventListener('pointermove', onGlobalPointerMove);
    window.removeEventListener('pointerup', onGlobalPointerUp);
  }

  function onNodeSelect(node: CanvasNode, event: PointerEvent) {
    const append = event.shiftKey || event.metaKey || event.ctrlKey;
    selectedConnectionId = null;
    setSelectedNode(node.id, append);
  }

  function onConnectionSelect(connectionId: string | null) {
    setSelectedNode(null);
    selectedConnectionId = connectionId;
  }

  function onStartEditConnectionLabel(connectionId: string, worldX: number, worldY: number, currentLabel: string) {
    selectedConnectionId = connectionId;
    labelEditor = {
      id: connectionId,
      x: worldX,
      y: worldY,
      value: currentLabel,
    };
  }

  function commitConnectionLabel() {
    if (!labelEditor) return;
    updateConnection(labelEditor.id, { label: labelEditor.value.trim() });
    labelEditor = null;
  }

  function cancelConnectionLabelEdit() {
    labelEditor = null;
  }

  function onLabelEditorInput(event: Event) {
    if (!labelEditor) return;
    const nextValue = (event.currentTarget as HTMLInputElement).value;
    labelEditor = { ...labelEditor, value: nextValue };
  }

  function onLabelEditorKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      commitConnectionLabel();
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      cancelConnectionLabelEdit();
    }
  }

  function onMiniMapCenter(worldX: number, worldY: number) {
    const layout = get(canvasLayout);
    const rect = viewportRef?.getBoundingClientRect();
    const w = rect?.width ?? 900;
    const h = rect?.height ?? 700;
    setViewport({
      ...layout.viewport,
      x: w / 2 - worldX * layout.viewport.zoom,
      y: h / 2 - worldY * layout.viewport.zoom,
    });
  }

  function onMiniMapPan(worldX: number, worldY: number) {
    onMiniMapCenter(worldX, worldY);
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (showSearch) {
        showSearch = false;
      }
      if (labelEditor) {
        labelEditor = null;
      }
      cancelCanvasTransientMode();
      setSelectedNode(null);
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      openSearch();
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key === '0') {
      event.preventDefault();
      const rect = viewportRef?.getBoundingClientRect();
      fitCanvasView(rect?.width ?? 900, rect?.height ?? 700);
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
      event.preventDefault();
      void saveCurrentCanvas();
      return;
    }

    if ((event.ctrlKey || event.metaKey) && (event.key === '=' || event.key === '+')) {
      event.preventDefault();
      const layout = get(canvasLayout);
      setViewport({ ...layout.viewport, zoom: Math.min(4, layout.viewport.zoom * 1.1) });
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key === '-') {
      event.preventDefault();
      const layout = get(canvasLayout);
      setViewport({ ...layout.viewport, zoom: Math.max(0.08, layout.viewport.zoom * 0.9) });
      return;
    }

    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      if (selectedConnectionId) {
        deleteConnection(selectedConnectionId);
        selectedConnectionId = null;
        return;
      }
      deleteSelectedNodes();
      return;
    }

    const key = event.key.toLowerCase();
    if (key === 'v') setCanvasMode('select');
    if (key === 'h') setCanvasMode('pan');
    if (key === 'c') setCanvasMode('connect');
    if (key === 'n') addNoteAtCenter();
  }

  function pickFile(file: SearchFile) {
    const center = viewportCenterWorld();
    addFileNode(file.path, file.rel, center.x - 240, center.y - 120);
    showSearch = false;
    searchQuery = '';
  }

  $: nodes = $canvasLayout.nodes;
  $: viewport = $canvasLayout.viewport;
  $: zoom = viewport.zoom;

  $: orderedNodes = [...nodes].sort((a, b) => (a.zIndex ?? 1) - (b.zIndex ?? 1));

  const CULL_MARGIN_WORLD = 280;
  $: viewportWidth = viewportRef?.clientWidth ?? 900;
  $: viewportHeight = viewportRef?.clientHeight ?? 700;
  $: visibleWorldLeft = (-viewport.x) / viewport.zoom - CULL_MARGIN_WORLD;
  $: visibleWorldTop = (-viewport.y) / viewport.zoom - CULL_MARGIN_WORLD;
  $: visibleWorldRight = visibleWorldLeft + viewportWidth / viewport.zoom + CULL_MARGIN_WORLD * 2;
  $: visibleWorldBottom = visibleWorldTop + viewportHeight / viewport.zoom + CULL_MARGIN_WORLD * 2;

  $: visibleNodes = orderedNodes.filter((node) => {
    const nodeRight = node.x + node.width;
    const nodeBottom = node.y + node.height;
    return (
      nodeRight >= visibleWorldLeft
      && node.x <= visibleWorldRight
      && nodeBottom >= visibleWorldTop
      && node.y <= visibleWorldBottom
    );
  });

  $: visibleNodeIdSet = new Set(visibleNodes.map((node) => node.id));
  $: visibleConnections = $canvasLayout.connections.filter(
    (connection) =>
      visibleNodeIdSet.has(connection.fromNodeId) && visibleNodeIdSet.has(connection.toNodeId),
  );

  $: canvasTransform = `translate(${viewport.x}px, ${viewport.y}px) scale(${viewport.zoom})`;

  $: if (showSearch && searchableFiles.length === 0) {
    void loadFilesIndex();
  }

  import { onDestroy, onMount } from 'svelte';
  onMount(() => {
    void loadCanvas();
    void loadFilesIndex();
    window.addEventListener('keydown', onKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', onKeyDown);
    window.removeEventListener('pointermove', onGlobalPointerMove);
    window.removeEventListener('pointerup', onGlobalPointerUp);
  });
</script>

<div class="overlay">
  <div class="top-row">
    <h2>Spatial Code Canvas</h2>
    <button class="close-btn" on:click={onClose} type="button">Close Canvas</button>
  </div>

  {#if $canvasCorruptionNotice}
    <div class="notice">{$canvasCorruptionNotice}</div>
  {/if}

  <CanvasToolbar
    mode={$canvasMode}
    onMode={onMode}
    onAddNote={addNoteAtCenter}
    onAddChat={addChatAtCenter}
    onOpenSearch={openSearch}
    onFitView={() => {
      const rect = viewportRef?.getBoundingClientRect();
      fitCanvasView(rect?.width ?? 900, rect?.height ?? 700);
    }}
    onSave={() => void saveCurrentCanvas()}
  />

  <div class="viewport" bind:this={viewportRef} on:pointerdown={onBackgroundPointerDown} on:wheel={onViewportWheel}>
    <div class="world" style="transform: {canvasTransform};">
      <CanvasGrid spacing={40} />
      <CanvasConnectionLayer
        nodes={visibleNodes}
        connections={visibleConnections}
        pending={$pendingConnection}
        {selectedConnectionId}
        onSelectConnection={onConnectionSelect}
        onStartEditLabel={onStartEditConnectionLabel}
      />

      {#each visibleNodes as node (node.id)}
        <CanvasNodeShell
          {node}
          selected={$selectedNodeIds.includes(node.id)}
          canConnect={$canvasMode === 'connect'}
          onSelect={(event) => onNodeSelect(node, event)}
          onBringFront={() => bringNodeToFront(node.id)}
          onDragStart={(event) => startNodeDrag(node, event)}
          onResizeStart={(event) => startNodeResize(node, event)}
          onDelete={() => deleteNode(node.id)}
          onStartConnect={(event) => startConnect(node, event)}
          onCompleteConnect={() => completePendingConnection(node.id)}
        >
          {#if zoom < 0.25}
            <div class="collapsed">{node.title ?? node.type}</div>
          {:else if node.type === 'file'}
            <CanvasFileNode filePath={node.filePath ?? ''} {zoom} />
          {:else if node.type === 'note'}
            <CanvasStickyNote
              text={node.noteText ?? ''}
              onChange={(value) => updateNode(node.id, { noteText: value })}
            />
          {:else}
            <CanvasChatNode content={node.chatContent ?? ''} />
          {/if}
        </CanvasNodeShell>
      {/each}
    </div>

    <CanvasMinimap
      nodes={nodes}
      {viewport}
      {viewportWidth}
      {viewportHeight}
      onCenter={onMiniMapCenter}
      onPanTo={onMiniMapPan}
    />

    {#if labelEditor}
      <div
        class="label-editor"
        style={`left: ${labelEditor.x * viewport.zoom + viewport.x}px; top: ${labelEditor.y * viewport.zoom + viewport.y}px;`}
        on:pointerdown|stopPropagation
      >
        <input
          value={labelEditor.value}
          on:input={onLabelEditorInput}
          on:keydown={onLabelEditorKeydown}
        />
        <button type="button" on:click={commitConnectionLabel}>Save</button>
      </div>
    {/if}

    <CanvasNodeSearch
      open={showSearch}
      files={searchableFiles}
      query={searchQuery}
      onClose={() => (showSearch = false)}
      onQuery={(value) => (searchQuery = value)}
      onPick={pickFile}
    />
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 44px 0 0 0;
    z-index: 150;
    background: #0d1117;
    color: var(--text);
    display: flex;
    flex-direction: column;
  }

  .top-row {
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, #0d1117 88%, var(--bg2) 12%);
  }

  h2 {
    font-size: 13px;
    letter-spacing: 0.02em;
    color: var(--text-dim);
    margin: 0;
  }

  .close-btn {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 10px;
    font-size: 11px;
    color: var(--text-dim);
    background: transparent;
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--text);
    border-color: var(--accent);
  }

  .label-editor {
    position: absolute;
    transform: translate(-50%, -50%);
    z-index: 28;
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(10, 16, 25, 0.94);
    border: 1px solid rgba(148, 163, 184, 0.45);
    border-radius: 8px;
    padding: 6px;
  }

  .label-editor input {
    width: 220px;
    border-radius: 6px;
    border: 1px solid rgba(148, 163, 184, 0.5);
    background: rgba(15, 23, 42, 0.88);
    color: var(--text);
    padding: 5px 8px;
    font-size: 12px;
    outline: none;
  }

  .label-editor button {
    border-radius: 6px;
    border: 1px solid rgba(148, 163, 184, 0.55);
    background: rgba(30, 41, 59, 0.9);
    color: var(--text);
    padding: 5px 9px;
    font-size: 11px;
    cursor: pointer;
  }

  .notice {
    margin: 8px 12px 0;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid #92400e;
    color: #fef3c7;
    background: rgba(133, 77, 14, 0.35);
    font-size: 12px;
  }

  .viewport {
    position: relative;
    flex: 1;
    overflow: hidden;
    cursor: crosshair;
  }

  .world {
    position: absolute;
    inset: 0;
    transform-origin: 0 0;
    will-change: transform;
  }

  .collapsed {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: var(--text-dim);
    background: rgba(255, 255, 255, 0.02);
  }
</style>
