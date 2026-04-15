<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    messages,
    currentSessionId,
    currentSessionTitle,
    setCurrentSession,
    clearCurrentSession,
  } from '$lib/stores/chat';
  import { currentWorkspace, setWorkspace } from '$lib/stores/workspace';
  import { addToast } from '$lib/stores/toast';

  const dispatch = createEventDispatcher<{ close: void }>();

  let sessionTitle = '';
  let selectedSessionId = '';
  let sessionQuery = '';
  let sessions: Array<{ id: string; title: string; workspace_path?: string; created_at: string; updated_at: string }> = [];
  let filteredSessions: typeof sessions = [];
  let titleInput: HTMLInputElement;
  let sessionError = '';
  let loading = false;
  let deleteConfirmation = false;

  function updateSelectedSessionTitle() {
    const selected = sessions.find((s) => s.id === selectedSessionId);
    if (selected) sessionTitle = selected.title;
    deleteConfirmation = false;
  }

  $: filteredSessions = sessions.filter((session) => {
    const query = sessionQuery.trim().toLowerCase();
    if (!query) return true;
    return session.title.toLowerCase().includes(query) || (session.workspace_path ?? '').toLowerCase().includes(query);
  });

  $: selectedSession = sessions.find((s) => s.id === selectedSessionId);

  function formatSessionDate(dateString: string) {
    return new Date(dateString).toLocaleString([], { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }

  /** Human-friendly relative time: "just now", "5 min ago", "2 hrs ago", etc. */
  function relativeTime(ms: number): string {
    const diff = Date.now() - ms;
    const secs = Math.floor(diff / 1000);
    if (secs < 60)  return 'just now';
    const mins = Math.floor(secs / 60);
    if (mins < 60)  return `${mins} min ago`;
    const hrs  = Math.floor(mins / 60);
    if (hrs  < 24)  return `${hrs} hr${hrs === 1 ? '' : 's'} ago`;
    const days = Math.floor(hrs / 24);
    if (days < 7)   return `${days} day${days === 1 ? '' : 's'} ago`;
    const wks  = Math.floor(days / 7);
    if (wks  < 5)   return `${wks} wk${wks === 1 ? '' : 's'} ago`;
    return new Date(ms).toLocaleDateString([], { month: 'short', day: 'numeric' });
  }

  async function refreshSessions() {
    try {
      sessions = await invoke('list_chat_sessions');
      sessionError = '';
    } catch (e) {
      sessionError = `Session load failed: ${e}`;
      sessions = [];
    }
  }

  function resetSessionForm() {
    selectedSessionId = '';
    sessionTitle = $currentSessionTitle || '';
    sessionError = '';
    deleteConfirmation = false;
  }

  function requestDeleteSession() {
    deleteConfirmation = true;
    sessionError = '';
  }

  function cancelDeleteConfirmation() {
    deleteConfirmation = false;
  }

  async function saveSession() {
    const title = sessionTitle.trim() || `Chat ${new Date().toLocaleString()}`;
    const history = $messages.map((msg) => ({ role: msg.role, content: msg.content }));
    loading = true;
    try {
      const result = await invoke<{ id: string }>('save_chat_session', {
        sessionId: selectedSessionId || undefined,
        title,
        workspacePath: $currentWorkspace?.path,
        messages: history,
      });
      selectedSessionId = result.id;
      sessionTitle = title;
      setCurrentSession(result.id, title);
      sessionError = '';
      addToast('Session saved.', 'success');
      await refreshSessions();
    } catch (e) {
      sessionError = `Save failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function renameSession() {
    if (!selectedSessionId) return;
    const title = sessionTitle.trim() || `Chat ${new Date().toLocaleString()}`;
    loading = true;
    try {
      await invoke('rename_chat_session', { sessionId: selectedSessionId, newTitle: title });
      sessionTitle = title;
      setCurrentSession(selectedSessionId, title);
      sessionError = '';
      addToast('Session renamed.', 'success');
      await refreshSessions();
    } catch (e) {
      sessionError = `Rename failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function duplicateSession() {
    if (!selectedSessionId) return;
    loading = true;
    try {
      const result = await invoke<{ id: string }>('duplicate_chat_session', { sessionId: selectedSessionId });
      selectedSessionId = result.id;
      sessionTitle = selectedSession ? selectedSession.title : sessionTitle;
      setCurrentSession(result.id, sessionTitle);
      sessionError = '';
      addToast('Session duplicated.', 'success');
      await refreshSessions();
    } catch (e) {
      sessionError = `Duplicate failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function loadSession() {
    if (!selectedSessionId) return;
    loading = true;
    try {
      const result = await invoke<any>('load_chat_session', { sessionId: selectedSessionId });
      const loadedMessages = result.messages.map((msg: any) => ({
        id: crypto.randomUUID(),
        role: msg.role,
        content: msg.content,
        timestamp: new Date(),
      }));

      if (typeof result.workspace_path === 'string' && result.workspace_path.trim()) {
        let branch = 'main';
        try {
          branch = await invoke<string>('get_git_branch', { workspacePath: result.workspace_path });
        } catch {
          // Keep fallback branch when path is not a git repo.
        }
        setWorkspace(result.workspace_path, branch);
      }

      messages.set(loadedMessages);
      sessionTitle = result.title ?? sessionTitle;
      setCurrentSession(result.id, result.title ?? sessionTitle);
      sessionError = '';
      addToast('Session loaded.', 'success');
      close();
    } catch (e) {
      sessionError = `Load failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function deleteSession() {
    if (!selectedSessionId) return;
    loading = true;
    try {
      await invoke('delete_chat_session', { sessionId: selectedSessionId });
      const deletedCurrent = selectedSessionId && selectedSessionId === $currentSessionId;
      selectedSessionId = '';
      deleteConfirmation = false;
      sessionError = '';
      if (deletedCurrent) clearCurrentSession();
      await refreshSessions();
    } catch (e) {
      sessionError = `Delete failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  function clearActiveSession() {
    clearCurrentSession();
    selectedSessionId = '';
    sessionError = '';
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }

    if (event.key === 'Enter' && document.activeElement === titleInput && !loading) {
      event.preventDefault();
      saveSession();
    }
  }

  function close() {
    dispatch('close');
  }

  onMount(async () => {
    selectedSessionId = $currentSessionId || '';
    sessionTitle = $currentSessionTitle || '';
    refreshSessions();
    window.addEventListener('keydown', handleKeydown);
    await tick();
    titleInput?.focus();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="session-overlay" on:click|self={close} role="presentation">
  <div class="session-panel" role="dialog" aria-modal="true" aria-label="Session manager">
    <header class="panel-header">
      <div>
        <h2>Sessions</h2>
        {#if $currentSessionTitle}
          <div class="current-session">Active session: <strong>{$currentSessionTitle}</strong></div>
        {/if}
        <div class="current-session">Saved sessions: <strong>{sessions.length}</strong></div>
      </div>
      <button class="close-btn" on:click={close} aria-label="Close sessions">✕</button>
    </header>

    <div class="section">
      <div class="session-row">
        <input
          class="session-title"
          bind:this={titleInput}
          bind:value={sessionTitle}
          placeholder="Session title"
          aria-label="Session title"
          disabled={loading}
        />
        <button class="btn-sm" on:click={saveSession} disabled={loading}>Save</button>
        <button class="btn-sm" on:click={resetSessionForm} disabled={loading}>New</button>
        <button class="btn-sm" on:click={refreshSessions} disabled={loading}>Refresh</button>
      </div>

      <div class="session-row">
        <input
          class="session-search"
          bind:value={sessionQuery}
          placeholder="Search sessions"
          aria-label="Search sessions"
          disabled={loading}
        />
      </div>

      <!-- Session list -->
      {#if sessions.length === 0}
        <div class="session-empty">No saved sessions yet. Send a message to auto-save.</div>
      {:else if filteredSessions.length === 0}
        <div class="session-empty">No sessions match "{sessionQuery}".</div>
      {:else}
        <div class="session-list" role="listbox" aria-label="Saved sessions">
          {#each filteredSessions as s (s.id)}
            {@const isActive = s.id === $currentSessionId}
            {@const isSelected = s.id === selectedSessionId}
            <button
              class="session-entry"
              class:active={isActive}
              class:selected={isSelected}
              role="option"
              aria-selected={isSelected}
              disabled={loading}
              on:click={() => { selectedSessionId = s.id; updateSelectedSessionTitle(); }}
              on:dblclick={loadSession}
              title="Double-click to load"
            >
              <div class="se-main">
                <span class="se-title">{s.title}</span>
                {#if isActive}<span class="se-active-badge">active</span>{/if}
              </div>
              <div class="se-meta">
                <span class="se-time">{relativeTime(Number(s.updated_at))}</span>
                {#if s.workspace_path}
                  <span class="se-ws">📁 {s.workspace_path.split(/[/\\]/).pop()}</span>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      {/if}

      <!-- Action bar for selected session -->
      {#if selectedSessionId}
        <div class="session-actions">
          <button class="btn-sm" on:click={loadSession}      disabled={loading}>↩ Load</button>
          <button class="btn-sm" on:click={renameSession}    disabled={loading}>✎ Rename</button>
          <button class="btn-sm" on:click={duplicateSession} disabled={loading}>⊕ Duplicate</button>
          {#if deleteConfirmation}
            <button class="btn-sm danger" on:click={deleteSession}           disabled={loading}>Confirm delete</button>
            <button class="btn-sm cancel" on:click={cancelDeleteConfirmation} disabled={loading}>Cancel</button>
          {:else}
            <button class="btn-sm danger" on:click={requestDeleteSession} disabled={loading}>🗑 Delete</button>
          {/if}
        </div>
        {#if selectedSession}
          <div class="session-detail-row">
            {#if selectedSession.workspace_path}
              <span class="sd-item">📁 {selectedSession.workspace_path}</span>
            {/if}
            <span class="sd-item">Updated {formatSessionDate(String(selectedSession.updated_at))}</span>
          </div>
        {/if}
      {/if}

      {#if $currentSessionId}
        <button class="btn-sm danger btn-clear-session" on:click={clearActiveSession} disabled={loading}>
          × Detach active session
        </button>
      {/if}

      {#if sessionError}
        <div class="session-error">{sessionError}</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .session-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 20;
    padding: 18px;
  }

  .session-panel {
    width: min(680px, 100%);
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 18px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 20px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }

  .panel-header h2 {
    margin: 0;
    font-size: 16px;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 18px;
    cursor: pointer;
  }

  .section {
    padding: 18px 20px 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .session-row {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    gap: 10px;
    align-items: center;
  }

  .session-title,
  .session-search {
    width: 100%;
    min-width: 140px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text);
    padding: 10px 12px;
    font-size: 13px;
  }

  .session-search {
    grid-column: 1 / -1;
  }

  .session-error {
    color: var(--red);
    font-size: 13px;
  }

  .session-empty {
    color: var(--text-dim);
    font-size: 13px;
    padding: 10px 0 0;
  }

  .current-session {
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-dim);
  }

  /* Session list */
  .session-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 320px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 6px;
    background: var(--bg);
  }

  .session-entry {
    width: 100%;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 8px 10px;
    cursor: pointer;
    color: var(--text);
    font-size: 13px;
    transition: background 0.1s, border-color 0.1s;
  }

  .session-entry:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--border);
  }

  .session-entry.selected {
    background: rgba(59, 130, 246, 0.12);
    border-color: rgba(59, 130, 246, 0.35);
  }

  .session-entry.active .se-title {
    color: var(--accent-hl);
  }

  .se-main {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .se-title {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .se-active-badge {
    font-size: 10px;
    background: var(--accent);
    color: #fff;
    border-radius: 999px;
    padding: 1px 6px;
    flex-shrink: 0;
  }

  .se-meta {
    display: flex;
    gap: 8px;
    margin-top: 3px;
    font-size: 11px;
    color: var(--text-dim);
  }

  .se-ws {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Action bar */
  .session-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .session-detail-row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    font-size: 12px;
    color: var(--text-dim);
    padding-top: 4px;
  }

  .btn-clear-session {
    align-self: flex-start;
  }

  .btn-sm {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 10px;
    padding: 10px 14px;
    cursor: pointer;
    font-size: 13px;
  }

  .btn-sm:hover:not(:disabled) { opacity: 0.92; }
  .btn-sm:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-sm.danger { background: var(--red); }
  .btn-sm.cancel {
    background: transparent;
    color: var(--text);
    border: 1px solid var(--border);
  }
</style>
