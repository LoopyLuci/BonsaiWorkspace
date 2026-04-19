<script lang="ts">
  /**
   * Mobile-first bottom-tab layout for the Bonsai Android app.
   *
   * Tabs: Chat | Files | Editor | VSCode
   *
   * To build for Android run:
   *   cargo tauri android init   (once, to generate src-tauri/gen/android/)
   *   cargo tauri android dev    (for development)
   *   cargo tauri android build  (for release APK)
   */
  import ChatPanel    from './ChatPanel.svelte';
  import FileTree     from './FileTree.svelte';
  import VscodeViewer from './VscodeViewer.svelte';
  import MobileSettingsPanel from './MobileSettingsPanel.svelte';
  import AssistantMobile from './AssistantMobile.svelte';
  import TerminalPanel from './TerminalPanel.svelte';
  import AgentsPanel from './AgentsPanel.svelte';
  import SessionPanel from './SessionPanel.svelte';
  import MobileViewPanel from './MobileViewPanel.svelte';
  import CodeCanvas from './CodeCanvas.svelte';
  import AgentVisionPanel from './AgentVisionPanel.svelte';
  import MobileHome from './MobileHome.svelte';
  import { initMobileDisplaySettings, mobileDisplayStyle } from '$lib/stores/mobileDisplay';
  import { showTerminal } from '$lib/stores/terminal';
  import { onMount } from 'svelte';

  type Tab = 'home' | 'chat' | 'files' | 'editor' | 'vscode' | 'settings' | 'buddy';
  const TAB_STORAGE_KEY = 'bonsai.mobile.activeTab.v1';

  let activeTab: Tab = 'home';
  let monacoEditorComponent: any = null;
  let monacoLoadError = '';
  let visitedTabs: Record<Tab, boolean> = {
    home: true,
    chat: false,
    files: false,
    editor: false,
    vscode: false,
    settings: false,
    buddy: false,
  };

  let showAgents = false;
  let showSession = false;
  let showMobileView = false;
  let showVision = false;
  let showCanvas = false;

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: 'home',     label: 'Home',     icon: '🏠' },
    { id: 'chat',     label: 'Chat',     icon: '💬' },
    { id: 'files',    label: 'Files',    icon: '📁' },
    { id: 'editor',   label: 'Editor',   icon: '✏️' },
    { id: 'vscode',   label: 'VSCode',   icon: '⚡' },
    { id: 'buddy',    label: 'Buddy',    icon: '🌿' },
    { id: 'settings', label: 'Settings', icon: '⚙️' },
  ];

  function setActiveTab(tab: Tab) {
    activeTab = tab;
    if (tab === 'editor') {
      void ensureMonacoEditorLoaded();
    }
    if (!visitedTabs[tab]) {
      visitedTabs = { ...visitedTabs, [tab]: true };
    }
    try {
      window.localStorage.setItem(TAB_STORAGE_KEY, tab);
    } catch {
      // Ignore storage failures (private mode / storage restrictions).
    }
  }

  async function ensureMonacoEditorLoaded() {
    if (monacoEditorComponent) return;
    try {
      const mod = await import('./MonacoEditor.svelte');
      monacoEditorComponent = mod.default;
      monacoLoadError = '';
    } catch (error) {
      monacoLoadError = String(error);
    }
  }

  function onTabKeyDown(event: KeyboardEvent) {
    const idx = tabs.findIndex((t) => t.id === activeTab);
    if (idx < 0) return;
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      setActiveTab(tabs[(idx + 1) % tabs.length].id);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      setActiveTab(tabs[(idx - 1 + tabs.length) % tabs.length].id);
    } else if (event.key === 'Home') {
      event.preventDefault();
      setActiveTab(tabs[0].id);
    } else if (event.key === 'End') {
      event.preventDefault();
      setActiveTab(tabs[tabs.length - 1].id);
    }
  }

  function handleHomeNavigate(event: CustomEvent<{ tab: Exclude<Tab, 'home'> }>) {
    setActiveTab(event.detail.tab);
  }

  onMount(() => {
    try {
      const saved = window.localStorage.getItem(TAB_STORAGE_KEY) as Tab | null;
      if (saved && tabs.some((t) => t.id === saved)) {
        activeTab = saved;
        visitedTabs = { ...visitedTabs, [saved]: true };
        if (saved === 'editor') {
          void ensureMonacoEditorLoaded();
        }
      }
    } catch {
      // Ignore storage failures.
    }

    const cleanup = initMobileDisplaySettings();
    return cleanup;
  });
</script>

<div class="mobile-root" style={$mobileDisplayStyle}>
  <!-- Content area -->
  <div class="mobile-content">
    <div id="mobile-tabpanel-home" class="tab-panel" class:active={activeTab === 'home'} role="tabpanel" aria-labelledby="mobile-tab-home" aria-hidden={activeTab !== 'home'}>
      {#if visitedTabs.home}
        <MobileHome
          on:navigate={handleHomeNavigate}
          on:openAgents={() => (showAgents = true)}
          on:openSession={() => (showSession = true)}
          on:openMobileView={() => (showMobileView = true)}
          on:openTerminal={() => showTerminal.set(true)}
          on:openVision={() => (showVision = true)}
          on:openCanvas={() => (showCanvas = true)}
        />
      {/if}
    </div>
    <div id="mobile-tabpanel-chat" class="tab-panel" class:active={activeTab === 'chat'} role="tabpanel" aria-labelledby="mobile-tab-chat" aria-hidden={activeTab !== 'chat'}>
      {#if visitedTabs.chat}
        <ChatPanel />
      {/if}
    </div>
    <div id="mobile-tabpanel-files" class="tab-panel" class:active={activeTab === 'files'} role="tabpanel" aria-labelledby="mobile-tab-files" aria-hidden={activeTab !== 'files'}>
      {#if visitedTabs.files}
        <FileTree />
      {/if}
    </div>
    <div id="mobile-tabpanel-editor" class="tab-panel" class:active={activeTab === 'editor'} role="tabpanel" aria-labelledby="mobile-tab-editor" aria-hidden={activeTab !== 'editor'}>
      {#if visitedTabs.editor}
        {#if monacoEditorComponent}
          <svelte:component this={monacoEditorComponent} theme="dark" />
        {:else if monacoLoadError}
          <div class="panel-state panel-state-error">Editor failed to load: {monacoLoadError}</div>
        {:else}
          <div class="panel-state">Loading editor...</div>
        {/if}
      {/if}
    </div>
    <div id="mobile-tabpanel-vscode" class="tab-panel" class:active={activeTab === 'vscode'} role="tabpanel" aria-labelledby="mobile-tab-vscode" aria-hidden={activeTab !== 'vscode'}>
      {#if visitedTabs.vscode}
        <VscodeViewer />
      {/if}
    </div>
    <div id="mobile-tabpanel-buddy" class="tab-panel" class:active={activeTab === 'buddy'} role="tabpanel" aria-labelledby="mobile-tab-buddy" aria-hidden={activeTab !== 'buddy'}>
      {#if visitedTabs.buddy}
        <AssistantMobile />
      {/if}
    </div>
    <div id="mobile-tabpanel-settings" class="tab-panel" class:active={activeTab === 'settings'} role="tabpanel" aria-labelledby="mobile-tab-settings" aria-hidden={activeTab !== 'settings'}>
      {#if visitedTabs.settings}
        <MobileSettingsPanel />
      {/if}
    </div>
  </div>

  {#if $showTerminal}
    <div class="terminal-drawer">
      <TerminalPanel />
    </div>
  {/if}

  {#if showSession}
    <SessionPanel on:close={() => (showSession = false)} />
  {/if}
  {#if showAgents}
    <AgentsPanel on:close={() => (showAgents = false)} />
  {/if}
  {#if showMobileView}
    <MobileViewPanel on:close={() => (showMobileView = false)} />
  {/if}
  {#if showVision}
    <AgentVisionPanel on:close={() => (showVision = false)} on:openChat={() => setActiveTab('chat')} />
  {/if}
  {#if showCanvas}
    <CodeCanvas onClose={() => (showCanvas = false)} />
  {/if}

  <!-- Bottom tab bar -->
  <nav class="tab-bar" aria-label="Mobile sections">
    <div class="tablist" role="tablist" aria-orientation="horizontal" tabindex="0" on:keydown={onTabKeyDown}>
      {#each tabs as tab}
        <button
          id={`mobile-tab-${tab.id}`}
          class="tab-btn"
          class:active={activeTab === tab.id}
          role="tab"
          type="button"
          aria-selected={activeTab === tab.id}
          aria-controls={`mobile-tabpanel-${tab.id}`}
          tabindex={activeTab === tab.id ? 0 : -1}
          on:click={() => setActiveTab(tab.id)}
        >
          <span class="tab-icon">{tab.icon}</span>
          <span class="tab-label">{tab.label}</span>
        </button>
      {/each}
    </div>
  </nav>
</div>

<style>
  .mobile-root {
    display: flex;
    flex-direction: column;
    height: var(--bonsai-mobile-vh, 100dvh);
    width: 100vw;
    padding-top: var(--bonsai-mobile-safe-top, env(safe-area-inset-top, 0px));
    padding-left: var(--bonsai-mobile-safe-left, env(safe-area-inset-left, 0px));
    padding-right: var(--bonsai-mobile-safe-right, env(safe-area-inset-right, 0px));
    background: var(--bg);
    color: var(--text);
    overflow: hidden;
  }

  .mobile-content {
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .tab-panel {
    position: absolute;
    inset: 0;
    display: none;
    flex-direction: column;
    overflow: hidden;
  }

  .tab-panel.active {
    display: flex;
  }

  .panel-state {
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-dim);
    font-size: 13px;
  }

  .panel-state-error {
    color: #fecaca;
    text-align: center;
    padding: 12px;
  }

  /* Bottom tab bar */
  .tab-bar {
    background: var(--bg2);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    /* Safe area for phones with home indicators */
    padding-bottom: var(--bonsai-mobile-safe-bottom, env(safe-area-inset-bottom, 0px));
  }

  .terminal-drawer {
    height: 42dvh;
    min-height: 240px;
    border-top: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
  }

  .tablist {
    display: flex;
  }

  .tab-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 8px 4px;
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    transition: color 0.15s;
    min-height: 56px;
  }

  .tab-btn.active {
    color: var(--accent-hl);
  }

  .tab-btn:hover:not(.active) {
    color: var(--text);
    background: var(--bg-hover);
  }

  .tab-icon {
    font-size: 20px;
    line-height: 1;
  }

  .tab-label {
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.02em;
  }
</style>
