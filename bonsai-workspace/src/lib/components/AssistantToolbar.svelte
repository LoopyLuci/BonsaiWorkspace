<script lang="ts">
  import { activeProfile } from '$lib/stores/assistant';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher<{ showDiagnostics: void; showProfile: void; showBackup: void; showHistory: void }>();

  let pinned = true;

  async function togglePin() {
    pinned = !pinned;
    await invoke('set_assistant_always_on_top', { onTop: pinned });
  }

  async function close() {
    await getCurrentWindow().hide();
  }
</script>

<div class="toolbar">
  <span class="title">
    <span class="leaf">🌿</span>
    {$activeProfile?.name ?? 'Bonsai Buddy'}
  </span>
  <div class="actions">
    <button class="icon-btn" on:click={() => dispatch('showHistory')} title="Session history">🕐</button>
    <button class="icon-btn" on:click={() => dispatch('showBackup')} title="Backup & restore">⬡</button>
    <button class="icon-btn" on:click={() => dispatch('showProfile')} title="Profile settings">👤</button>
    <button class="icon-btn" on:click={() => dispatch('showDiagnostics')} title="Diagnostics">⚙</button>
    <button class="icon-btn" on:click={togglePin} title={pinned ? 'Unpin' : 'Pin on top'}>
      {pinned ? '📌' : '📍'}
    </button>
    <button class="icon-btn close" on:click={close} title="Hide">✕</button>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 44px;
    padding: 0 10px;
    background: var(--bg, #1e1e1e);
    border-bottom: 1px solid var(--border, #3e3e42);
    user-select: none;
    -webkit-app-region: drag;
  }
  .title { font-weight: 600; font-size: 0.9rem; display: flex; align-items: center; gap: 6px; }
  .leaf { font-size: 1.1rem; }
  .actions { display: flex; gap: 4px; -webkit-app-region: no-drag; }
  .icon-btn {
    width: 28px; height: 28px;
    border: none; border-radius: 6px;
    background: transparent;
    color: var(--fg, #ccc);
    cursor: pointer;
    font-size: 0.85rem;
    display: flex; align-items: center; justify-content: center;
  }
  .icon-btn:hover { background: var(--bg2, #252526); }
  .close:hover { background: var(--danger, #e05260); color: #fff; }
</style>
