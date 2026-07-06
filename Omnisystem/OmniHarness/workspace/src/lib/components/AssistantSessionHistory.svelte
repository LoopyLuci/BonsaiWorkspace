<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { currentSessionId } from '$lib/stores/assistantSessions';

  export let onClose: () => void = () => {};
  export let onLoadSession: (id: string) => void = () => {};

  interface AssistantSession {
    id: string; profile_id: string | null; title: string;
    created_at: number; updated_at: number;
  }

  let sessions: AssistantSession[] = [];
  let search = '';
  let loading = false;
  let error = '';
  let selected = new Set<string>();
  let deleting = false;
  let undoVisible = false;
  let undoCountdown = 30;

  interface PendingDelete {
    session: AssistantSession;
    commitTimer: ReturnType<typeof setTimeout>;
  }

  let pendingDeletes: PendingDelete[] = [];
  let countdownTimer: ReturnType<typeof setInterval> | null = null;

  $: displayed = sessions.filter(s =>
    s.title.toLowerCase().includes(search.toLowerCase())
  );

  async function load() {
    loading = true; error = '';
    try {
      sessions = await invoke<AssistantSession[]>('list_assistant_sessions', { profileId: null });
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function deleteSelected() {
    if (selected.size === 0) return;
    deleting = true;
    const ids = Array.from(selected);
    for (const id of ids) {
      const session = sessions.find(s => s.id === id);
      if (!session) continue;
      queueDelete(session);
    }
    selected = new Set();
    deleting = false;
  }

  function deleteSingle(id: string) {
    const session = sessions.find(s => s.id === id);
    if (!session) return;
    queueDelete(session);
  }

  function toggleSelect(id: string) {
    const s = new Set(selected);
    if (s.has(id)) s.delete(id); else s.add(id);
    selected = s;
  }

  function fmtDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString(undefined, {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
    });
  }

  function queueDelete(session: AssistantSession) {
    sessions = sessions.filter(s => s.id !== session.id);
    selected.delete(session.id);
    selected = new Set(selected);

    const commitTimer = setTimeout(async () => {
      try {
        await invoke('delete_assistant_session', { id: session.id });
      } catch (e) {
        error = `Delete failed: ${e}`;
        sessions = [session, ...sessions.filter(s => s.id !== session.id)];
      } finally {
        pendingDeletes = pendingDeletes.filter(p => p.session.id !== session.id);
        if (pendingDeletes.length === 0) stopUndoTimer();
      }
    }, 30000);

    pendingDeletes = [...pendingDeletes, { session, commitTimer }];
    startUndoTimer();
  }

  function startUndoTimer() {
    undoVisible = true;
    undoCountdown = 30;
    if (countdownTimer) clearInterval(countdownTimer);
    countdownTimer = setInterval(() => {
      undoCountdown = Math.max(0, undoCountdown - 1);
      if (undoCountdown === 0) {
        stopUndoTimer();
      }
    }, 1000);
  }

  function stopUndoTimer() {
    undoVisible = false;
    undoCountdown = 30;
    if (countdownTimer) {
      clearInterval(countdownTimer);
      countdownTimer = null;
    }
  }

  function dismissUndoPopup() {
    stopUndoTimer();
  }

  function undoLastDelete() {
    const pending = pendingDeletes[pendingDeletes.length - 1];
    if (!pending) return;
    clearTimeout(pending.commitTimer);
    pendingDeletes = pendingDeletes.slice(0, -1);
    sessions = [pending.session, ...sessions.filter(s => s.id !== pending.session.id)];
    if (pendingDeletes.length === 0) stopUndoTimer();
  }

  async function createAndLoadSession() {
    try {
      const newSession = await invoke<AssistantSession>('create_assistant_session', {
        profileId: null,
        title: 'New conversation',
      });
      sessions = [newSession, ...sessions.filter(s => s.id !== newSession.id)];
      onLoadSession(newSession.id);
      onClose();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(load);
  onDestroy(() => {
    if (countdownTimer) clearInterval(countdownTimer);
  });
</script>

<div class="history">
  <div class="h-header">
    <div class="header-left">
      <span>Session History</span>
      <button class="new-chat-btn" on:click={createAndLoadSession} title="Create a new chat">+ New Chat</button>
    </div>
    <button class="close-btn" on:click={onClose}>✕</button>
  </div>

  <div class="h-toolbar">
    <input class="search" type="text" bind:value={search} placeholder="Search sessions…" />
    {#if selected.size > 0}
      <button class="danger-btn" on:click={deleteSelected} disabled={deleting}>
        Delete {selected.size}
      </button>
    {/if}
  </div>

  {#if error}<div class="err">{error}</div>{/if}

  <div class="list">
    {#if loading}
      <div class="notice">Loading…</div>
    {:else if displayed.length === 0}
      <div class="notice dim">No sessions found</div>
    {:else}
      {#each displayed as s (s.id)}
        <div class="row" class:active={$currentSessionId === s.id} class:sel={selected.has(s.id)}>
          <input type="checkbox" checked={selected.has(s.id)}
            on:change={() => toggleSelect(s.id)} class="check" />
          <div class="info" role="button" tabindex="0"
            on:click={() => { onLoadSession(s.id); onClose(); }}
            on:keydown={(e) => e.key === 'Enter' && (onLoadSession(s.id), onClose())}
          >
            <span class="title">{s.title}</span>
            <span class="date">{fmtDate(s.updated_at)}</span>
          </div>
          <button class="del-btn" on:click={() => deleteSingle(s.id)} title="Delete">✕</button>
        </div>
      {/each}
    {/if}
  </div>

  <div class="h-footer">
    <span class="dim">{displayed.length} session{displayed.length !== 1 ? 's' : ''}</span>
    <button on:click={load}>Refresh</button>
  </div>

  {#if undoVisible && pendingDeletes.length > 0}
    <div class="undo-popup" role="status" aria-live="polite">
      <div class="undo-copy">
        Deleted {pendingDeletes.length} chat{pendingDeletes.length !== 1 ? 's' : ''}. Undo ({undoCountdown}s)
      </div>
      <div class="undo-actions">
        <button class="undo-btn" on:click={undoLastDelete}>Undo</button>
        <button class="undo-close" on:click={dismissUndoPopup} aria-label="Dismiss undo popup">✕</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .history { display: flex; flex-direction: column; height: 100%; background: var(--bg); color: var(--fg); font-size: 0.82rem; position: relative; }
  .h-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-bottom: 1px solid var(--border); font-weight: 600;
  }
  .header-left { display: flex; align-items: center; gap: 8px; }
  .new-chat-btn {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
    color: var(--fg);
    padding: 3px 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 600;
  }
  .new-chat-btn:hover { background: color-mix(in srgb, var(--accent) 28%, transparent); }
  .close-btn { background: none; border: none; color: var(--fg-dim); cursor: pointer; font-size: 1rem; }
  .h-toolbar { display: flex; gap: 8px; padding: 8px 12px; border-bottom: 1px solid var(--border); }
  .search {
    flex: 1; background: var(--bg2); border: 1px solid var(--border); border-radius: 6px;
    color: var(--fg); padding: 5px 8px; font-size: 0.82rem;
  }
  .danger-btn {
    background: var(--danger); border: none; color: #fff;
    padding: 5px 12px; border-radius: 6px; cursor: pointer; font-size: 0.8rem;
  }
  .list { flex: 1; overflow-y: auto; }
  .row {
    display: flex; align-items: center; gap: 8px; padding: 8px 12px;
    border-bottom: 1px solid var(--border); cursor: default;
  }
  .row:hover { background: var(--bg2); }
  .row.active { background: color-mix(in srgb, var(--accent) 10%, var(--bg2)); }
  .row.sel { background: color-mix(in srgb, var(--accent) 15%, var(--bg2)); }
  .check { cursor: pointer; flex-shrink: 0; }
  .info { flex: 1; display: flex; flex-direction: column; gap: 2px; cursor: pointer; min-width: 0; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .date { font-size: 0.72rem; color: var(--fg-dim); }
  .del-btn {
    background: none; border: none; color: var(--fg-dim); cursor: pointer;
    font-size: 0.8rem; opacity: 0; padding: 2px 4px; border-radius: 4px;
  }
  .row:hover .del-btn { opacity: 1; }
  .del-btn:hover { color: var(--danger); background: color-mix(in srgb, var(--danger) 15%, transparent); }
  .notice { padding: 16px 12px; text-align: center; }
  .dim { color: var(--fg-dim); }
  .err { color: var(--danger); padding: 6px 12px; font-size: 0.78rem; }
  .h-footer {
    display: flex; justify-content: space-between; align-items: center;
    padding: 6px 12px; border-top: 1px solid var(--border);
  }
  .h-footer button {
    background: var(--bg2); border: 1px solid var(--border); color: var(--fg);
    padding: 3px 10px; border-radius: 4px; cursor: pointer; font-size: 0.78rem;
  }

  .undo-popup {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--accent) 50%, var(--border));
    background: color-mix(in srgb, var(--bg2) 88%, black);
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
  }
  .undo-copy { color: var(--fg); font-size: 0.78rem; }
  .undo-actions { display: flex; align-items: center; gap: 6px; }
  .undo-btn {
    background: var(--accent);
    border: none;
    color: white;
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 600;
  }
  .undo-close {
    background: transparent;
    border: none;
    color: var(--fg-dim);
    cursor: pointer;
    font-size: 0.9rem;
  }
</style>
