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
  import MonacoEditor from './MonacoEditor.svelte';
  import VscodeViewer from './VscodeViewer.svelte';

  type Tab = 'chat' | 'files' | 'editor' | 'vscode';
  let activeTab: Tab = 'chat';

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: 'chat',   label: 'Chat',   icon: '💬' },
    { id: 'files',  label: 'Files',  icon: '📁' },
    { id: 'editor', label: 'Editor', icon: '✏️' },
    { id: 'vscode', label: 'VSCode', icon: '⚡' },
  ];
</script>

<div class="mobile-root">
  <!-- Content area -->
  <div class="mobile-content">
    <div id="mobile-tabpanel-chat" class="tab-panel" class:active={activeTab === 'chat'} role="tabpanel" aria-labelledby="mobile-tab-chat" aria-hidden={activeTab !== 'chat'}>
      <ChatPanel />
    </div>
    <div id="mobile-tabpanel-files" class="tab-panel" class:active={activeTab === 'files'} role="tabpanel" aria-labelledby="mobile-tab-files" aria-hidden={activeTab !== 'files'}>
      <FileTree />
    </div>
    <div id="mobile-tabpanel-editor" class="tab-panel" class:active={activeTab === 'editor'} role="tabpanel" aria-labelledby="mobile-tab-editor" aria-hidden={activeTab !== 'editor'}>
      <MonacoEditor theme="dark" />
    </div>
    <div id="mobile-tabpanel-vscode" class="tab-panel" class:active={activeTab === 'vscode'} role="tabpanel" aria-labelledby="mobile-tab-vscode" aria-hidden={activeTab !== 'vscode'}>
      <VscodeViewer />
    </div>
  </div>

  <!-- Bottom tab bar -->
  <nav class="tab-bar" aria-label="Mobile sections">
    <div class="tablist" role="tablist" aria-orientation="horizontal">
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
          on:click={() => (activeTab = tab.id)}
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
    height: 100vh;
    width: 100vw;
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

  /* Bottom tab bar */
  .tab-bar {
    background: var(--bg2);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    /* Safe area for phones with home indicators */
    padding-bottom: env(safe-area-inset-bottom, 0px);
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
