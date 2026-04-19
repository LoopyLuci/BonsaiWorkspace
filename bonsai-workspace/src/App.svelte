<script lang="ts">
  import FileTree        from '$lib/components/FileTree.svelte';
  import ChatPanel       from '$lib/components/ChatPanel.svelte';
  import StatusBar       from '$lib/components/StatusBar.svelte';
  import CommandPalette  from '$lib/components/CommandPalette.svelte';
  import SettingsPanel   from '$lib/components/SettingsPanel.svelte';
  import SessionPanel    from '$lib/components/SessionPanel.svelte';
  import AgentConnectPanel from '$lib/components/AgentConnectPanel.svelte';
  import AgentsPanel       from '$lib/components/AgentsPanel.svelte';
  import ResourcesPanel    from '$lib/components/ResourcesPanel.svelte';
  import TerminalPanel   from '$lib/components/TerminalPanel.svelte';
  import VscodeViewer    from '$lib/components/VscodeViewer.svelte';
  import DownloadProgress from '$lib/components/DownloadProgress.svelte';
  import BootstrapScreen from '$lib/components/BootstrapScreen.svelte';
  import CodeCanvas from '$lib/components/CodeCanvas.svelte';
  import MobileViewPanel from '$lib/components/MobileViewPanel.svelte';
  import MobileLayout from '$lib/components/MobileLayout.svelte';
  import AndroidUsbLab from '$lib/components/AndroidUsbLab.svelte';

  import { showTerminal, toggleTerminal } from '$lib/stores/terminal';
  import { isBootstrapping, initModelStores } from '$lib/stores/models';
  import { restorePersistentSession } from '$lib/stores/chat';
  import { loadAgentConfigs, loadPersonas } from '$lib/stores/agents';
  import Toasts from '$lib/components/Toast.svelte';

  // ── Layout toggles ────────────────────────────────────────────────────────
  let showFileTree  = true;
  let showChat      = true;
  let showSettings  = false;
  let showSession   = false;
  let showAgentConnect = false;
  let showAgents       = false;
  let showResources    = false;
  let showAgentVision  = false;
  let showCanvas       = false;
  let showMobileView   = false;
  let showTools = false;
  let showAndroidUsbModal = false;
  let showVscode    = false;
  let sidebarWidth  = 280;
  let chatWidth     = 360;
  let resizingPane: 'sidebar' | 'chat' | null = null;
  let pointerStartX = 0;
  let pointerStartWidth = 0;
  let monacoEditorComponent: any = null;
  let monacoLoadError = '';
  let monacoLoadQueued = false;
  let agentVisionPanelComponent: any = null;
  let agentVisionLoadError = '';

  const MIN_PANE_WIDTH = 120;
  const MIN_EDITOR_WIDTH = 260;
  const RESIZER_WIDTH = 10;

  function hasRightPane() {
    return showChat || showVscode;
  }

  function visiblePaneCount() {
    // Editor is always present.
    return 1 + (showFileTree ? 1 : 0) + (hasRightPane() ? 1 : 0);
  }

  function visibleResizerCount() {
    let count = 0;
    if (showFileTree) count += 1;
    if (hasRightPane()) count += 1;
    return count;
  }

  function paneMaxWidth(pane: 'sidebar' | 'chat') {
    const viewportWidth = typeof window !== 'undefined' ? window.innerWidth : 1280;
    const paneCount = visiblePaneCount();
    const shareLimit = paneCount >= 3 ? 0.75 : paneCount === 2 ? 0.9 : 1.0;
    const maxByShare = Math.floor(viewportWidth * shareLimit);
    const otherPaneWidth = pane === 'sidebar'
      ? (hasRightPane() ? chatWidth : 0)
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

  async function loadMonacoEditorComponent() {
    if (monacoEditorComponent) return;
    try {
      const mod = await import('$lib/components/MonacoEditor.svelte');
      monacoEditorComponent = mod.default;
      monacoLoadError = '';
    } catch (error) {
      monacoLoadError = String(error);
    }
  }

  function queueMonacoEditorLoad() {
    if (monacoLoadQueued) return;
    monacoLoadQueued = true;
    window.setTimeout(() => {
      void loadMonacoEditorComponent();
    }, 0);
  }

  async function loadAgentVisionPanelComponent() {
    if (agentVisionPanelComponent) return;
    try {
      const mod = await import('$lib/components/AgentVisionPanel.svelte');
      agentVisionPanelComponent = mod.default;
      agentVisionLoadError = '';
    } catch (error) {
      agentVisionLoadError = String(error);
    }
  }

  function toggleAgentVisionPanel() {
    showAgentVision = !showAgentVision;
    if (showAgentVision) {
      void loadAgentVisionPanelComponent();
    }
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
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';

  function globalKey(e: KeyboardEvent) {
    if (((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') || e.key === 'F1') {
      e.preventDefault();
      window.dispatchEvent(new CustomEvent('open-command-palette'));
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      showSession = true;
    }
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      invoke('toggle_assistant_window');
    }
  }

  function openSessionEvent() {
    showSession = true;
  }

  function openAgentsEvent() {
    showAgents = true;
  }

  onMount(() => {
    document.documentElement.dataset.theme = theme;
    initModelStores();
    void restorePersistentSession();
    void loadAgentConfigs();
    void loadPersonas();
    window.addEventListener('keydown', globalKey, true);
    window.addEventListener('open-session', openSessionEvent);
    window.addEventListener('open-agents', openAgentsEvent);
    // Detect Android — works both in Tauri mobile and browser dev preview.
    isMobile = /android/i.test(navigator.userAgent);
    if (isMobile) {
      // Keep the desktop shell on mobile, but start with a cleaner viewport.
      showFileTree = false;
      showVscode = false;
      showChat = true;
      chatWidth = Math.min(chatWidth, Math.max(240, Math.floor(window.innerWidth * 0.92)));
    }
    queueMonacoEditorLoad();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', globalKey, true);
    window.removeEventListener('open-session', openSessionEvent);
    window.removeEventListener('open-agents', openAgentsEvent);
  });

  $: if (showFileTree) {
    sidebarWidth = Math.max(MIN_PANE_WIDTH, Math.min(sidebarWidth, paneMaxWidth('sidebar')));
  }

  $: if (showChat) {
    chatWidth = Math.max(MIN_PANE_WIDTH, Math.min(chatWidth, paneMaxWidth('chat')));
  }
</script>

<!-- Root shell -->
<div class="root" class:mobile-shell={isMobile}>

  {#if isMobile}
    <MobileLayout />
    <Toasts />
    <DownloadProgress />
    {#if $isBootstrapping}<BootstrapScreen />{/if}
  {:else}

  <!-- Top toolbar -->
  <header class="toolbar">
    <span class="logo">🌿</span>
    <div class="toolbar-actions">
      <button class="btn-icon" title="Toggle File Tree (Ctrl+B)"
        on:click={() => (showFileTree = !showFileTree)}>
        {showFileTree ? '◀ Tree' : '▶ Tree'}
      </button>
      <button class="btn-icon" title="Toggle Terminal (Ctrl+`)"
        on:click={toggleTerminal}>Terminal</button>
      <button class="btn-icon" title="Toggle Chat"
        on:click={() => (showChat = !showChat)}>Chat</button>
      <button class="btn-icon" class:active={showCanvas} title="Spatial Code Canvas"
        on:click={() => (showCanvas = !showCanvas)}>Canvas</button>
      <button class="btn-icon" class:active={showAgents} title="Open Agents"
        on:click={() => (showAgents = true)}>⚡ Agents</button>
      <button class="btn-icon" class:active={showResources} title="Open Resources"
        on:click={() => (showResources = true)}>Resources</button>
      <button class="btn-icon" title="Settings"
        on:click={() => (showSettings = !showSettings)}>⚙</button>

      <!-- Tools dropdown -->
      <div class="tools-dropdown" on:mouseleave={() => (showTools = false)}>
        <button class="btn-icon" title="Tools" on:click={() => (showTools = !showTools)}>Tools ▾</button>
        {#if showTools}
          <div class="tools-menu" role="menu">
            <button class="tools-item" title="Bonsai Buddy Assistant (Ctrl+Shift+B)" on:click={() => { invoke('toggle_assistant_window'); showTools=false; }}>
              🌿 Bonsai Buddy Assistant
            </button>
            <button class="tools-item" on:click={() => { toggleAgentVisionPanel(); showTools=false; }}>
              ⚡ Agent Vision
            </button>
            <button class="tools-item" on:click={async () => { try { await invoke('toggle_android_usb_lab_window'); } catch { showAndroidUsbModal = true; } showTools=false; }}>
              📱 Android USB Lab
            </button>
            <button class="tools-item" on:click={() => { showAgentConnect = true; showTools=false; }}>
              🔗 Agent Connect
            </button>
          </div>
        {/if}
      </div>
      <button class="btn-icon" class:active={showMobileView} title="Mobile Viewer"
        on:click={() => (showMobileView = !showMobileView)}>Mobile Viewer</button>
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
        {#if monacoEditorComponent}
          <svelte:component this={monacoEditorComponent} {theme} />
        {:else if monacoLoadError}
          <div class="pane-state pane-state-error">Editor failed to load: {monacoLoadError}</div>
        {:else}
          <div class="pane-state">Loading editor...</div>
        {/if}
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
  <div class="terminal-drawer" class:terminal-hidden={!$showTerminal}>
    <TerminalPanel />
  </div>

  <Toasts />

  <!-- Status bar -->
  <StatusBar />

  <!-- Overlays -->
  <CommandPalette />
  {#if showSettings}<SettingsPanel on:close={() => (showSettings = false)} on:openAndroidUsbLab={async () => { try { await invoke('toggle_android_usb_lab_window'); } catch { showAndroidUsbModal = true; } }} />{/if}
  {#if showSession}<SessionPanel on:close={() => (showSession = false)} />{/if}
  {#if showAgentConnect}<AgentConnectPanel on:close={() => (showAgentConnect = false)} />{/if}
  {#if showAgents}<AgentsPanel on:close={() => (showAgents = false)} />{/if}
  {#if showResources}<ResourcesPanel on:close={() => (showResources = false)} />{/if}
  {#if showAgentVision}
    {#if agentVisionPanelComponent}
      <svelte:component
        this={agentVisionPanelComponent}
        on:close={() => (showAgentVision = false)}
        on:openChat={() => {
          showChat = true;
          showVscode = false;
        }}
      />
    {:else if agentVisionLoadError}
      <div class="overlay-error" role="alert">
        <div>Agent Vision failed to load: {agentVisionLoadError}</div>
        <button class="btn-icon" type="button" on:click={() => (showAgentVision = false)}>Close</button>
      </div>
    {:else}
      <div class="overlay-loading" role="status">Loading Agent Vision...</div>
    {/if}
  {/if}
  {#if showCanvas}
    <CodeCanvas onClose={() => (showCanvas = false)} />
  {/if}
  {#if showMobileView}
    <MobileViewPanel on:close={() => (showMobileView = false)} />
  {/if}
  {#if showAndroidUsbModal}
    <AndroidUsbLab on:close={() => (showAndroidUsbModal = false)} />
  {/if}
  <DownloadProgress />
  {#if $isBootstrapping}<BootstrapScreen />{/if}

  {/if}

</div>

<style>
  /* ── CSS custom properties ── */
  :global(:root) {
    --z-canvas:   10;
    --z-inline:   20;
    --z-panel:   100;
    --z-dropdown: 300;
    --z-overlay:  500;
    --z-modal:    800;
    --z-context: 1000;
    --z-toast:   2000;
    --z-critical: 9999;
  }
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
    height: 100dvh;
    min-height: 100vh;
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
    height: 100dvh;
    min-height: 100vh;
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
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }

  .toolbar-actions::-webkit-scrollbar {
    display: none;
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

  .pane-state {
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-dim);
    font-size: 13px;
    background: color-mix(in srgb, var(--bg2) 80%, transparent);
  }

  .pane-state-error {
    color: #fecaca;
  }

  .overlay-loading,
  .overlay-error {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 170;
    max-width: min(420px, calc(100vw - 40px));
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg2);
    color: var(--text);
    padding: 10px 12px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.35);
    display: flex;
    gap: 10px;
    align-items: center;
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
    overflow: hidden;
    transition: height 0.16s ease;
  }

  .terminal-drawer.terminal-hidden {
    height: 0;
    border-top: 0;
  }

  .mobile-shell .toolbar {
    height: auto;
    min-height: 44px;
    padding: 6px 8px;
    gap: 6px;
  }

  .mobile-shell .logo {
    font-size: 13px;
    margin-right: 4px;
    white-space: nowrap;
  }

  .mobile-shell .toolbar-actions {
    gap: 6px;
    padding-bottom: 2px;
  }

  .mobile-shell .btn-icon {
    padding: 6px 8px;
    font-size: 11px;
  }

  .mobile-shell .sidebar-pane {
    width: min(76vw, 320px);
  }

  .mobile-shell .chat-pane {
    width: min(92vw, 430px);
  }

  @media (max-width: 900px) {
    .split-resizer {
      width: 12px;
    }

    .terminal-drawer {
      height: 220px;
    }
  }

  /* Tools dropdown */
  .tools-dropdown { position: relative; }
  .tools-menu {
    position: absolute;
    right: 0;
    top: 36px;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0,0,0,0.45);
    z-index: var(--z-dropdown);
    display: flex;
    flex-direction: column;
    min-width: 220px;
    padding: 6px;
  }
  .tools-item {
    background: transparent;
    border: none;
    text-align: left;
    padding: 8px 10px;
    color: var(--text);
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
  }
  .tools-item:hover { background: var(--bg-hover); }
</style>
