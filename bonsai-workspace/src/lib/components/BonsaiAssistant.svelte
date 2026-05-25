<script lang="ts">
  import { onMount } from 'svelte';
  import { assistantInitError, assistantError, initAssistantStores } from '$lib/stores/assistant';
  import AssistantToolbar from './AssistantToolbar.svelte';
  import AssistantMessageList from './AssistantMessageList.svelte';
  import AssistantInputBar from './AssistantInputBar.svelte';
  import QuickActionChips from './QuickActionChips.svelte';
  import ToolConfirmCard from './ToolConfirmCard.svelte';
  import AssistantDiagnostics from './AssistantDiagnostics.svelte';
  import AssistantAvatar from './AssistantAvatar.svelte';
  import ProfileManager from './ProfileManager.svelte';
  import BackupManager from './BackupManager.svelte';
  import AssistantSessionHistory from './AssistantSessionHistory.svelte';
  import { currentSessionId } from '$lib/stores/assistantSessions';
  import { initModelStores } from '$lib/stores/models';
  import { loadApiSettings } from '$lib/stores/settings';
  import { invoke } from '@tauri-apps/api/core';

  let ready = false;
  let showDiagnostics = false;
  let showProfile = false;
  let showBackup = false;
  let showHistory = false;

  async function loadSession(id: string) {
    await invoke('load_assistant_session', { sessionId: id });
    currentSessionId.set(id);
  }

  onMount(async () => {
    try {
      await loadApiSettings();   // resolve actual port before any fetch
      initModelStores();
      await initAssistantStores();
    } finally {
      ready = true;
    }
  });
</script>

<div class="assistant-root">
  <AssistantToolbar
    on:showDiagnostics={() => showDiagnostics = true}
    on:showProfile={() => showProfile = true}
    on:showBackup={() => showBackup = true}
    on:showHistory={() => showHistory = true}
  />

  {#if showDiagnostics}
    <div class="overlay">
      <AssistantDiagnostics onClose={() => showDiagnostics = false} />
    </div>
  {:else if showProfile}
    <div class="overlay">
      <ProfileManager onClose={() => showProfile = false} />
    </div>
  {:else if showBackup}
    <div class="overlay">
      <BackupManager onClose={() => showBackup = false} />
    </div>
  {:else if showHistory}
    <div class="overlay">
      <AssistantSessionHistory
        onClose={() => showHistory = false}
        onLoadSession={loadSession}
      />
    </div>
  {:else}
    <div class="avatar-placeholder">
      <AssistantAvatar />
    </div>

    {#if ready}
      {#if $assistantInitError}
        <div class="loading error">{$assistantInitError}</div>
      {/if}
      {#if $assistantError}
        <div class="error-banner" role="alert">
          ⚠ {$assistantError}
          <button class="error-dismiss" on:click={() => assistantError.set('')}>✕</button>
        </div>
      {/if}
      <QuickActionChips />
      <ToolConfirmCard />
      <AssistantMessageList />
      <AssistantInputBar />
    {:else}
      <div class="loading">Loading...</div>
    {/if}
  {/if}
</div>

<style>
  :global(body) {
    margin: 0;
    background: var(--bg, #1e1e1e);
    color: var(--fg, #cccccc);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    height: 100vh;
    overflow: hidden;

    /* Design tokens */
    --bg:          #1e1e1e;
    --bg2:         #252526;
    --border:      #3e3e42;
    --fg:          #cccccc;
    --fg-dim:      #888888;
    --accent:      #5ca4ea;
    --accent-hover:#4a93d9;
    --danger:      #e05260;
  }

  .assistant-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .avatar-placeholder {
    height: 100px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg2);
    border-bottom: 1px solid var(--border);
  }
.loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-dim);
    font-size: 0.9rem;
  }

  .overlay {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 12px;
    background: #4a1a1a;
    border-bottom: 1px solid #7a2020;
    color: #fca5a5;
    font-size: 0.82rem;
    flex-shrink: 0;
  }
  .error-dismiss {
    background: none;
    border: none;
    color: #fca5a5;
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px;
    line-height: 1;
  }
</style>
