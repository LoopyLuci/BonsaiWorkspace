<script lang="ts">
  import FileTree        from '$lib/components/FileTree.svelte';
  import MonacoEditor    from '$lib/components/MonacoEditor.svelte';
  import ChatPanel       from '$lib/components/ChatPanel.svelte';
  import StatusBar       from '$lib/components/StatusBar.svelte';
  import CommandPalette  from '$lib/components/CommandPalette.svelte';
  import SettingsPanel   from '$lib/components/SettingsPanel.svelte';
  import SessionPanel    from '$lib/components/SessionPanel.svelte';
  import AgentConnectPanel from '$lib/components/AgentConnectPanel.svelte';
  import AgentsPanel       from '$lib/components/AgentsPanel.svelte';
  import TerminalPanel   from '$lib/components/TerminalPanel.svelte';
  import VscodeViewer    from '$lib/components/VscodeViewer.svelte';
  import MobileLayout    from '$lib/components/MobileLayout.svelte';
  import DownloadProgress from '$lib/components/DownloadProgress.svelte';
  import BootstrapScreen from '$lib/components/BootstrapScreen.svelte';

  import { showTerminal, toggleTerminal } from '$lib/stores/terminal';
  import { isBootstrapping, initModelStores } from '$lib/stores/models';
  import { currentSessionTitle, clearCurrentSession, restorePersistentSession } from '$lib/stores/chat';
  import { loadAgentConfigs, loadPersonas } from '$lib/stores/agents';
  import Toasts from '$lib/components/Toast.svelte';

  // ── Layout toggles ────────────────────────────────────────────────────────
  let showFileTree  = true;
  let showChat      = true;
  let showSettings  = false;
  let showSession   = false;
  let showAgentConnect = false;
  let showAgents       = false;
  let showVscode    = false;
  let sidebarWidth  = 280;
  let chatWidth     = 360;
  let resizingPane: 'sidebar' | 'chat' | null = null;
  let pointerStartX = 0;
  let pointerStartWidth = 0;

  const MIN_PANE_WIDTH = 120;
  const MIN_EDITOR_WIDTH = 260;
  const RESIZER_WIDTH = 10;

  function visibleResizerCount() {
    let count = 0;
    if (showFileTree) count += 1;
    if (showChat) count += 1;
    return count;
  }

  function paneMaxWidth(pane: 'sidebar' | 'chat') {
    const viewportWidth = typeof window !== 'undefined' ? window.innerWidth : 1280;
    const sidePaneCount = (showFileTree ? 1 : 0) + (showChat ? 1 : 0);
    const shareLimit = sidePaneCount >= 2 ? 0.5 : 0.75;
    const maxByShare = Math.floor(viewportWidth * shareLimit);
    const otherPaneWidth = pane === 'sidebar'
      ? (showChat ? chatWidth : 0)
      : (showFileTree ? sidebarWidth : 0);
    const maxByEditor = Math.max(
      MIN_PANE_WIDTH,
      viewportWidth - otherPaneWidth - MIN_EDITOR_WIDTH - (visibleResizerCount() * RESIZER_WIDTH),
    );
    return Math.max(MIN_PANE_WIDTH, Math.min(maxByShare, maxByEditor));
  }

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
      sidebarWidth = Math.min(paneMaxWidth('sidebar'), Math.max(MIN_PANE_WIDTH, pointerStartWidth + delta));
    } else if (resizingPane === 'chat') {
      chatWidth = Math.min(paneMaxWidth('chat'), Math.max(MIN_PANE_WIDTH, pointerStartWidth - delta));
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

  // Mobile detection — true when running inside Tauri on Android.
  // Falls back to user-agent check so it works in dev/browser previews too.
  let isMobile = false;

  // Sync theme on mount; initialise model/bootstrap listeners
  import { onMount, onDestroy } from 'svelte';

  function globalKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      showSession = true;
    }
  }

  function openSessionEvent() {
    showSession = true;
  }

  onMount(() => {
    document.documentElement.dataset.theme = theme;
    initModelStores();
    void restorePersistentSession();
    void loadAgentConfigs();
    void loadPersonas();
    window.addEventListener('keydown', globalKey);
    window.addEventListener('open-session', openSessionEvent);
    // Detect Android — works both in Tauri mobile and browser dev preview.
    isMobile = /android/i.test(navigator.userAgent);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', globalKey);
    window.removeEventListener('open-session', openSessionEvent);
  });

  $: if (showFileTree) {
    sidebarWidth = Math.max(MIN_PANE_WIDTH, Math.min(sidebarWidth, paneMaxWidth('sidebar')));
  }

  $: if (showChat) {
    chatWidth = Math.max(MIN_PANE_WIDTH, Math.min(chatWidth, paneMaxWidth('chat')));
  }
</script>

<!-- Android: render the mobile tab-bar layout instead of the desktop shell -->
{#if isMobile}
  <MobileLayout />
{:else}

<!-- Root shell -->
<div class="root">

  <!-- Top toolbar -->
  <header class="toolbar">
    <span class="logo">🌿 Bonsai</span>
    {#if $currentSessionTitle}
      <div class="toolbar-session">
        <button class="toolbar-session-open" on:click={() => (showSession = true)} title="Open session manager" type="button">
          Session: {$currentSessionTitle}
        </button>
        <button class="toolbar-session-clear" on:click|stopPropagation={clearCurrentSession} aria-label="Clear current session" type="button">×</button>
      </div>
    {/if}
    <div class="toolbar-actions">
      <button class="btn-icon" title="Toggle File Tree (Ctrl+B)"
        on:click={() => (showFileTree = !showFileTree)}>
        {showFileTree ? '◀ Tree' : '▶ Tree'}
      </button>
      <button class="btn-icon" class:active={showAgents} title="Open Agents"
        on:click={() => (showAgents = true)}>⚡ Agents</button>
      <button class="btn-icon" title="Toggle Terminal (Ctrl+`)"
        on:click={toggleTerminal}>Terminal</button>
      <button class="btn-icon" title="Toggle Chat"
        on:click={() => (showChat = !showChat)}>Chat</button>
      <button class="btn-icon" title="Settings"
        on:click={() => (showSettings = !showSettings)}>⚙</button>
      <button class="btn-icon" title="Agent Connect"
        on:click={() => (showAgentConnect = true)}>Agent Connect</button>
      <button class="btn-icon" title="Toggle VSCode Viewer"
        on:click={() => (showVscode = !showVscode)}>VSCode</button>
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

      {#if showVscode}
        <div
          class="split-resizer right-resizer"
          role="separator"
          aria-orientation="vertical"
          on:pointerdown={startResizingChat}
          title="Resize VSCode pane"
        >
          <span>⋮</span>
        </div>
        <div class="pane chat-pane" style="width: {chatWidth}px">
          <VscodeViewer />
        </div>
      {:else if showChat}
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
          <ChatPanel on:openSession={() => (showSession = true)} />
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

  <Toasts />

  <!-- Status bar -->
  <StatusBar />

  <!-- Overlays -->
  <CommandPalette />
  {#if showSettings}<SettingsPanel on:close={() => (showSettings = false)} />{/if}
  {#if showSession}<SessionPanel on:close={() => (showSession = false)} />{/if}
  {#if showAgentConnect}<AgentConnectPanel on:close={() => (showAgentConnect = false)} />{/if}
  {#if showAgents}<AgentsPanel on:close={() => (showAgents = false)} />{/if}
  <DownloadProgress />
  {#if $isBootstrapping}<BootstrapScreen />{/if}

</div>

{/if}

<style>
  /* ── CSS custom properties ── */
  :global([data-theme='dark']) {
    --bg:        #18181b;
    --bg2:       #1c1c1f;
    --bg-hover:  #27272a;
    --text:      #e4e4e7;
    --text-dim:  #71717a;
    --border:    #3f3f46;
    --accent:    #16a34a;
    --accent-hl: #4ade80;
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
    --accent:    #15803d;
    --accent-hl: #16a34a;
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
    --accent:    #4ade80;
    --accent-hl: #86efac;
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
  .btn-icon.active {
    color: var(--accent-hl);
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }

  .toolbar-session {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: rgba(34, 197, 94, 0.14);
    border: 1px solid rgba(34, 197, 94, 0.3);
    border-radius: 999px;
    color: var(--text);
    padding: 6px 8px;
    font-size: 12px;
  }

  .toolbar-session-open,
  .toolbar-session-clear {
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 0;
  }

  .toolbar-session-open {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 6px 10px;
    transition: background 0.1s, border-color 0.1s;
  }

  .toolbar-session-open:hover {
    background: rgba(34, 197, 94, 0.18);
  }

  .toolbar-session-clear {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(255,255,255,0.12);
    font-size: 12px;
    transition: background 0.1s;
  }

  .toolbar-session-clear:hover {
    background: rgba(255,255,255,0.2);
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
    min-width: 120px;
    max-width: none;
    border-right: 1px solid var(--border);
  }

  .editor-pane {
    flex: 1;
    min-width: 0;
  }

  .chat-pane {
    width: 22%;
    min-width: 120px;
    max-width: none;
    overflow: visible;
    border-left: 1px solid var(--border);
  }

  .terminal-drawer {
    height: 240px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg);
  }
</style>
