<script lang="ts">
  import FileTree        from '$lib/components/FileTree.svelte';
  import MonacoEditor    from '$lib/components/MonacoEditor.svelte';
  import ChatPanel       from '$lib/components/ChatPanel.svelte';
  import StatusBar       from '$lib/components/StatusBar.svelte';
  import CommandPalette  from '$lib/components/CommandPalette.svelte';
  import SettingsPanel   from '$lib/components/SettingsPanel.svelte';
  import TerminalPanel   from '$lib/components/TerminalPanel.svelte';
  import DownloadProgress from '$lib/components/DownloadProgress.svelte';
  import BootstrapScreen from '$lib/components/BootstrapScreen.svelte';

  import { showTerminal, toggleTerminal } from '$lib/stores/terminal';
  import { isBootstrapping, initModelStores } from '$lib/stores/models';

  // ── Layout toggles ────────────────────────────────────────────────────────
  let showFileTree  = true;
  let showChat      = true;
  let showSettings  = false;
  let sidebarWidth  = 280;
  let chatWidth     = 360;
  let resizingPane: 'sidebar' | 'chat' | null = null;
  let pointerStartX = 0;
  let pointerStartWidth = 0;

  function startResizingSidebar(event: PointerEvent) {
    event.preventDefault();
    resizingPane = 'sidebar';
    pointerStartX = event.clientX;
    pointerStartWidth = sidebarWidth;
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', stopResizing);
  }

  function startResizingChat(event: PointerEvent) {
    event.preventDefault();
    resizingPane = 'chat';
    pointerStartX = event.clientX;
    pointerStartWidth = chatWidth;
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', stopResizing);
  }

  function onPointerMove(event: PointerEvent) {
    if (!resizingPane) return;
    const delta = event.clientX - pointerStartX;
    if (resizingPane === 'sidebar') {
      sidebarWidth = Math.min(420, Math.max(180, pointerStartWidth + delta));
    } else if (resizingPane === 'chat') {
      chatWidth = Math.min(520, Math.max(220, pointerStartWidth - delta));
    }
  }

  function stopResizing() {
    resizingPane = null;
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', stopResizing);
  }

  // ── Theme ─────────────────────────────────────────────────────────────────
  type Theme = 'dark' | 'light' | 'high-contrast';
  let theme: Theme = 'dark';

  function cycleTheme() {
    const order: Theme[] = ['dark', 'light', 'high-contrast'];
    theme = order[(order.indexOf(theme) + 1) % order.length];
    document.documentElement.dataset.theme = theme;
  }

  // Sync theme on mount; initialise model/bootstrap listeners
  import { onMount } from 'svelte';
  onMount(() => {
    document.documentElement.dataset.theme = theme;
    initModelStores();
  });
</script>

<!-- Root shell -->
<div class="root">

  <!-- Top toolbar -->
  <header class="toolbar">
    <span class="logo">🌿 Bonsai</span>
    <div class="toolbar-actions">
      <button class="btn-icon" title="Toggle File Tree (Ctrl+B)"
        on:click={() => (showFileTree = !showFileTree)}>
        {showFileTree ? '◀ Tree' : '▶ Tree'}
      </button>
      <button class="btn-icon" title="Toggle Terminal (Ctrl+`)"
        on:click={toggleTerminal}>Terminal</button>
      <button class="btn-icon" title="Toggle Chat"
        on:click={() => (showChat = !showChat)}>Chat</button>
      <button class="btn-icon" title="Settings"
        on:click={() => (showSettings = !showSettings)}>⚙</button>
      <button class="btn-icon" title="Cycle Theme"
        on:click={cycleTheme}>
        {theme === 'dark' ? '☀' : theme === 'light' ? '⬛' : '🌑'}
      </button>
    </div>
  </header>

  <!-- Main work area -->
  <main class="work-area">
    <div class="split-layout">
      {#if showFileTree}
        <div class="pane sidebar-pane" style="width: {sidebarWidth}px">
          <FileTree />
        </div>
        <div
          class="split-resizer left-resizer"
          role="separator"
          aria-orientation="vertical"
          on:pointerdown={startResizingSidebar}
          title="Resize file tree"
        >
          <span>⋮</span>
        </div>
      {/if}

      <div class="pane editor-pane">
        <MonacoEditor {theme} />
      </div>

      {#if showChat}
        <div
          class="split-resizer right-resizer"
          role="separator"
          aria-orientation="vertical"
          on:pointerdown={startResizingChat}
          title="Resize chat pane"
        >
          <span>⋮</span>
        </div>
        <div class="pane chat-pane" style="width: {chatWidth}px">
          <ChatPanel />
        </div>
      {/if}
    </div>
  </main>

  <!-- Terminal drawer -->
  {#if $showTerminal}
    <div class="terminal-drawer">
      <TerminalPanel />
    </div>
  {/if}

  <!-- Status bar -->
  <StatusBar />

  <!-- Overlays -->
  <CommandPalette />
  {#if showSettings}<SettingsPanel on:close={() => (showSettings = false)} />{/if}
  <DownloadProgress />
  {#if $isBootstrapping}<BootstrapScreen />{/if}

</div>

<style>
  /* ── CSS custom properties ── */
  :global([data-theme='dark']) {
    --bg:        #18181b;
    --bg2:       #1c1c1f;
    --bg-hover:  #27272a;
    --text:      #e4e4e7;
    --text-dim:  #71717a;
    --border:    #3f3f46;
    --accent:    #3b82f6;
    --accent-hl: #60a5fa;
    --green:     #22c55e;
    --red:       #ef4444;
    --amber:     #f59e0b;
  }
  :global([data-theme='light']) {
    --bg:        #ffffff;
    --bg2:       #f4f4f5;
    --bg-hover:  #e4e4e7;
    --text:      #18181b;
    --text-dim:  #71717a;
    --border:    #d4d4d8;
    --accent:    #2563eb;
    --accent-hl: #3b82f6;
    --green:     #16a34a;
    --red:       #dc2626;
    --amber:     #d97706;
  }
  :global([data-theme='high-contrast']) {
    --bg:        #000000;
    --bg2:       #0a0a0a;
    --bg-hover:  #1a1a1a;
    --text:      #ffffff;
    --text-dim:  #a1a1aa;
    --border:    #ffffff;
    --accent:    #60a5fa;
    --accent-hl: #93c5fd;
    --green:     #4ade80;
    --red:       #f87171;
    --amber:     #fbbf24;
  }

  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }
  :global(body) {
    font-family: system-ui, -apple-system, sans-serif;
    background: var(--bg);
    color: var(--text);
    overflow: hidden;
    height: 100vh;
    width: 100vw;
  }

  /* Diff line decorations for Monaco */
  :global(.diff-insert-line) { background: rgba(34, 197, 94, 0.12); }
  :global(.diff-delete-line) { background: rgba(239, 68, 68, 0.12); }
  :global(.diff-glyph-insert)::before { content: '+'; color: var(--green); font-weight: bold; margin-left: 4px; }
  :global(.diff-glyph-delete)::before { content: '−'; color: var(--red);   font-weight: bold; margin-left: 4px; }

  /* Splitpanes reset */
  :global(.splitpanes) { height: 100%; }
  :global(.splitpanes__splitter) {
    background: var(--border) !important;
    width: 2px !important;
    transition: background 0.15s;
  }
  :global(.splitpanes__splitter:hover) { background: var(--accent) !important; }

  /* ── Layout ── */
  .root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
  }

  .toolbar {
    display: flex;
    align-items: center;
    height: 44px;
    padding: 0 12px;
    background: var(--bg2);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 8px;
    user-select: none;
    /* Make the window draggable while keeping buttons clickable */
    -webkit-app-region: drag;
  }
  .toolbar button,
  .toolbar-actions { -webkit-app-region: no-drag; }

  .logo {
    font-weight: 700;
    font-size: 15px;
    letter-spacing: -0.3px;
    color: var(--accent-hl);
    margin-right: 8px;
  }

  .toolbar-actions {
    display: flex;
    gap: 4px;
    margin-left: auto;
    align-items: center;
  }

  .btn-icon {
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-dim);
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
    white-space: nowrap;
  }
  .btn-icon:hover {
    background: var(--bg-hover);
    color: var(--text);
    border-color: var(--border);
  }

  .work-area {
    flex: 1;
    overflow: hidden;
  }

  .split-layout {
    display: flex;
    width: 100%;
    height: 100%;
    min-height: 0;
  }

  .split-resizer {
    width: 10px;
    cursor: col-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    transition: background 0.2s ease;
    color: rgba(255,255,255,0.45);
    user-select: none;
  }

  .split-resizer:hover {
    background: rgba(255,255,255,0.06);
    color: rgba(255,255,255,0.8);
  }

  .split-resizer span {
    transform: rotate(90deg);
    font-size: 18px;
    line-height: 1;
  }

  .pane {
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .sidebar-pane {
    width: 280px;
    min-width: 180px;
    max-width: 420px;
    border-right: 1px solid var(--border);
  }

  .editor-pane {
    flex: 1;
    min-width: 0;
  }

  .chat-pane {
    width: 22%;
    min-width: 240px;
    max-width: 420px;
    border-left: 1px solid var(--border);
  }

  .terminal-drawer {
    height: 240px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg);
  }
</style>
