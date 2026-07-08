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

  type ChatSession = {
    id: string;
    title: string;
    workspace_path?: string;
    tags?: string[];
    is_locked?: boolean;
    is_favorite?: boolean;
    is_deleted?: boolean;
    group_ids?: string[];
    created_at: number;
    updated_at: number;
  };

  type ChatSessionGroup = {
    id: string;
    title: string;
    tags?: string[];
    is_locked?: boolean;
    is_favorite?: boolean;
    is_deleted?: boolean;
    chat_count?: number;
    created_at: number;
    updated_at: number;
  };

  let sessionTitle = '';
  let selectedChatId = '';
  let selectedGroupId = '';
  let sessionQuery = '';
  let groupQuery = '';
  let chats: ChatSession[] = [];
  let groups: ChatSessionGroup[] = [];
  let includeArchived = false;
  let titleInput: HTMLInputElement;
  let sessionError = '';
  let loading = false;
  let deleteConfirmation = false;
  let createGroupTitle = '';
  let chatTagInput = '';
  let groupTagInput = '';
  let selectedChat: ChatSession | undefined;
  let selectedGroup: ChatSessionGroup | undefined;
  let filteredChats: ChatSession[] = [];
  let filteredGroups: ChatSessionGroup[] = [];

  $: selectedChat = chats.find((s) => s.id === selectedChatId);
  $: selectedGroup = groups.find((g) => g.id === selectedGroupId);

  $: filteredChats = chats.filter((chat) => {
    const query = sessionQuery.trim().toLowerCase();
    if (!query) return true;
    return (
      chat.title.toLowerCase().includes(query)
      || (chat.workspace_path ?? '').toLowerCase().includes(query)
      || (chat.tags ?? []).some((t) => t.toLowerCase().includes(query))
    );
  });

  $: filteredGroups = groups.filter((group) => {
    const query = groupQuery.trim().toLowerCase();
    if (!query) return true;
    return group.title.toLowerCase().includes(query)
      || (group.tags ?? []).some((t) => t.toLowerCase().includes(query));
  });

  function formatSessionDate(ts: number): string {
    return new Date(ts).toLocaleString([], {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function relativeTime(ms: number): string {
    const diff = Date.now() - ms;
    const secs = Math.floor(diff / 1000);
    if (secs < 60) return 'just now';
    const mins = Math.floor(secs / 60);
    if (mins < 60) return `${mins} min ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs} hr${hrs === 1 ? '' : 's'} ago`;
    const days = Math.floor(hrs / 24);
    if (days < 7) return `${days} day${days === 1 ? '' : 's'} ago`;
    const weeks = Math.floor(days / 7);
    if (weeks < 5) return `${weeks} wk${weeks === 1 ? '' : 's'} ago`;
    return new Date(ms).toLocaleDateString([], { month: 'short', day: 'numeric' });
  }

  async function refreshAll() {
    try {
      [chats, groups] = await Promise.all([
        invoke<ChatSession[]>('list_chat_sessions_detailed', { includeDeleted: includeArchived }),
        invoke<ChatSessionGroup[]>('list_chat_session_groups', { includeDeleted: includeArchived }),
      ]);
      sessionError = '';
      if (selectedChatId && !chats.some((c) => c.id === selectedChatId)) selectedChatId = '';
      if (selectedGroupId && !groups.some((g) => g.id === selectedGroupId)) selectedGroupId = '';
      if (!selectedChatId && chats.length) selectedChatId = chats[0].id;
      if (!selectedGroupId && groups.length) selectedGroupId = groups[0].id;
      syncDetailInputs();
    } catch (e) {
      sessionError = `Manager load failed: ${e}`;
      chats = [];
      groups = [];
    }
  }

  function syncDetailInputs() {
    sessionTitle = selectedChat?.title ?? $currentSessionTitle ?? '';
    chatTagInput = (selectedChat?.tags ?? []).join(', ');
    groupTagInput = (selectedGroup?.tags ?? []).join(', ');
  }

  function parseTags(raw: string): string[] {
    return raw
      .split(',')
      .map((x) => x.trim())
      .filter((x) => x.length > 0)
      .slice(0, 20);
  }

  async function saveCurrentChat() {
    const title = sessionTitle.trim() || `Chat ${new Date().toLocaleString()}`;
    const history = $messages.map((msg) => ({
      role: msg.role,
      content: msg.content,
      stats: msg.stats,
      tools_used: msg.tools_used,
      agent_id: msg.agent_id,
      agent_label: msg.agent_label,
      agent_color: msg.agent_color,
      agent_icon: msg.agent_icon,
      agent_slot: msg.agent_slot,
      created_at: msg.timestamp?.getTime?.() ?? Date.now(),
    }));

    loading = true;
    try {
      const result = await invoke<{ id: string }>('save_chat_session', {
        sessionId: selectedChatId || undefined,
        title,
        workspacePath: $currentWorkspace?.path,
        messages: history,
      });
      selectedChatId = result.id;
      setCurrentSession(result.id, title);
      sessionError = '';
      addToast('Chat saved.', 'success');
      await refreshAll();
    } catch (e) {
      sessionError = `Save failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function loadChat() {
    if (!selectedChatId) return;
    loading = true;
    try {
      const result = await invoke<any>('load_chat_session', { sessionId: selectedChatId });
      const loadedMessages = result.messages.map((msg: any) => ({
        id: crypto.randomUUID(),
        role: msg.role,
        content: msg.content,
        timestamp: new Date(msg.created_at ?? Date.now()),
        stats: msg.stats ?? undefined,
        tools_used: Array.isArray(msg.tools_used) ? msg.tools_used : undefined,
        agent_id: typeof msg.agent_id === 'string' ? msg.agent_id : undefined,
        agent_label: typeof msg.agent_label === 'string' ? msg.agent_label : undefined,
        agent_color: typeof msg.agent_color === 'string' ? msg.agent_color : undefined,
        agent_icon: typeof msg.agent_icon === 'string' ? msg.agent_icon : undefined,
        agent_slot: typeof msg.agent_slot === 'number' && Number.isFinite(msg.agent_slot) ? msg.agent_slot : undefined,
      }));

      if (typeof result.workspace_path === 'string' && result.workspace_path.trim()) {
        let branch = 'main';
        try {
          branch = await invoke<string>('get_git_branch', { workspacePath: result.workspace_path });
        } catch {
          // Keep fallback branch.
        }
        setWorkspace(result.workspace_path, branch);
      }

      messages.set(loadedMessages);
      setCurrentSession(result.id, result.title ?? selectedChat?.title ?? '');
      addToast('Chat loaded.', 'success');
      close();
    } catch (e) {
      sessionError = `Load failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function renameChat() {
    if (!selectedChatId) return;
    loading = true;
    try {
      await invoke('rename_chat_session', { sessionId: selectedChatId, newTitle: sessionTitle.trim() || 'Untitled chat' });
      await refreshAll();
      if (selectedChatId === $currentSessionId) {
        setCurrentSession(selectedChatId, sessionTitle.trim() || 'Untitled chat');
      }
      addToast('Chat renamed.', 'success');
    } catch (e) {
      sessionError = `Rename failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function duplicateChat() {
    if (!selectedChatId) return;
    loading = true;
    try {
      const result = await invoke<{ id: string }>('duplicate_chat_session', { sessionId: selectedChatId });
      selectedChatId = result.id;
      await refreshAll();
      addToast('Chat duplicated.', 'success');
    } catch (e) {
      sessionError = `Duplicate failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function permanentlyDeleteChat() {
    if (!selectedChatId) return;
    loading = true;
    try {
      await invoke('delete_chat_session', { sessionId: selectedChatId });
      const deletedCurrent = selectedChatId === $currentSessionId;
      selectedChatId = '';
      deleteConfirmation = false;
      if (deletedCurrent) clearCurrentSession();
      await refreshAll();
      addToast('Chat permanently deleted.', 'success');
    } catch (e) {
      sessionError = `Delete failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function updateChatMeta(patch: {
    title?: string;
    tags?: string[];
    is_locked?: boolean;
    is_favorite?: boolean;
    is_deleted?: boolean;
  }) {
    if (!selectedChatId) return;
    loading = true;
    try {
      await invoke('update_chat_session_meta', {
        sessionId: selectedChatId,
        title: patch.title ?? null,
        tags: patch.tags ?? null,
        isLocked: patch.is_locked ?? null,
        isFavorite: patch.is_favorite ?? null,
        isDeleted: patch.is_deleted ?? null,
      });
      await refreshAll();
    } catch (e) {
      sessionError = `Chat update failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function createSessionGroup() {
    const title = createGroupTitle.trim() || 'New session';
    loading = true;
    try {
      const result = await invoke<{ id: string }>('create_chat_session_group', { title });
      createGroupTitle = '';
      selectedGroupId = result.id;
      await refreshAll();
      addToast('Session created.', 'success');
    } catch (e) {
      sessionError = `Session create failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function updateGroupMeta(patch: {
    title?: string;
    tags?: string[];
    is_locked?: boolean;
    is_favorite?: boolean;
    is_deleted?: boolean;
  }) {
    if (!selectedGroupId) return;
    loading = true;
    try {
      await invoke('update_chat_session_group_meta', {
        groupId: selectedGroupId,
        title: patch.title ?? null,
        tags: patch.tags ?? null,
        isLocked: patch.is_locked ?? null,
        isFavorite: patch.is_favorite ?? null,
        isDeleted: patch.is_deleted ?? null,
      });
      await refreshAll();
    } catch (e) {
      sessionError = `Session update failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  function onGroupTitleChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void updateGroupMeta({ title: input.value.trim() });
  }

  async function linkSelectedChatToSelectedGroup() {
    if (!selectedChatId || !selectedGroupId) return;
    loading = true;
    try {
      await invoke('link_chat_to_session_group', {
        groupId: selectedGroupId,
        chatId: selectedChatId,
      });
      await refreshAll();
      addToast('Chat attached to session.', 'success');
    } catch (e) {
      sessionError = `Attach failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function unlinkChatFromGroup(groupId: string, chatId: string) {
    loading = true;
    try {
      await invoke('unlink_chat_from_session_group', { groupId, chatId });
      await refreshAll();
      addToast('Chat detached from session.', 'success');
    } catch (e) {
      sessionError = `Detach failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  function clearActiveChatLink() {
    clearCurrentSession();
    addToast('Detached active chat from canvas.', 'info');
  }

  function requestDeleteChat() {
    deleteConfirmation = true;
  }

  function cancelDeleteChat() {
    deleteConfirmation = false;
  }

  function close() {
    dispatch('close');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }

    if (event.key === 'Enter' && document.activeElement === titleInput && !loading) {
      event.preventDefault();
      void saveCurrentChat();
    }
  }

  onMount(async () => {
    selectedChatId = $currentSessionId || '';
    await refreshAll();
    window.addEventListener('keydown', handleKeydown);
    await tick();
    titleInput?.focus();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="session-overlay" on:click|self={close} role="presentation">
  <div class="session-panel" role="dialog" aria-modal="true" aria-label="Chats and Sessions manager">
    <header class="panel-header">
      <div>
        <h2>Chats and Sessions</h2>
        <div class="current-session">Active chat: <strong>{$currentSessionTitle || 'none'}</strong></div>
        <div class="current-session">Chats: <strong>{chats.length}</strong> · Sessions: <strong>{groups.length}</strong></div>
      </div>
      <button class="close-btn" on:click={close} aria-label="Close manager">✕</button>
    </header>

    <div class="manager-controls">
      <label class="archive-toggle">
        <input type="checkbox" bind:checked={includeArchived} on:change={refreshAll} />
        Include archived
      </label>
      <button class="btn-sm" on:click={refreshAll} disabled={loading}>Refresh</button>
      <button class="btn-sm" on:click={clearActiveChatLink} disabled={loading}>Detach Active</button>
    </div>

    <div class="manager-grid">
      <section class="panel-col">
        <h3>Chats</h3>
        <div class="search-row">
          <input class="session-search" bind:value={sessionQuery} placeholder="Search chats" disabled={loading} />
        </div>
        <div class="session-list" role="listbox" aria-label="Chats">
          {#if filteredChats.length === 0}
            <div class="session-empty">No chats found.</div>
          {:else}
            {#each filteredChats as c (c.id)}
              <button
                class="session-entry"
                class:selected={c.id === selectedChatId}
                class:active={c.id === $currentSessionId}
                class:archived={c.is_deleted}
                disabled={loading}
                on:click={() => { selectedChatId = c.id; syncDetailInputs(); }}
                on:dblclick={loadChat}
              >
                <div class="se-main">
                  <span class="se-title">{c.title}</span>
                  {#if c.is_favorite}<span class="chip fav">♥</span>{/if}
                  {#if c.is_locked}<span class="chip lock">🔒</span>{/if}
                  {#if c.is_deleted}<span class="chip archived">archived</span>{/if}
                </div>
                <div class="se-meta">
                  <span>{relativeTime(Number(c.updated_at))}</span>
                  {#if c.workspace_path}
                    <span class="se-ws">📁 {c.workspace_path.split(/[/\\]/).pop()}</span>
                  {/if}
                </div>
                {#if (c.tags ?? []).length > 0}
                  <div class="tag-row">
                    {#each c.tags ?? [] as tag}
                      <span class="tag-chip">#{tag}</span>
                    {/each}
                  </div>
                {/if}
              </button>
            {/each}
          {/if}
        </div>
      </section>

      <section class="panel-col">
        <h3>Sessions</h3>
        <div class="search-row">
          <input class="session-search" bind:value={groupQuery} placeholder="Search sessions" disabled={loading} />
        </div>
        <div class="group-create-row">
          <input class="session-title" bind:value={createGroupTitle} placeholder="New session title" disabled={loading} />
          <button class="btn-sm" on:click={createSessionGroup} disabled={loading}>Create</button>
        </div>
        <div class="session-list" role="listbox" aria-label="Sessions">
          {#if filteredGroups.length === 0}
            <div class="session-empty">No sessions found.</div>
          {:else}
            {#each filteredGroups as g (g.id)}
              <button
                class="session-entry"
                class:selected={g.id === selectedGroupId}
                class:archived={g.is_deleted}
                disabled={loading}
                on:click={() => { selectedGroupId = g.id; syncDetailInputs(); }}
              >
                <div class="se-main">
                  <span class="se-title">{g.title}</span>
                  {#if g.is_favorite}<span class="chip fav">♥</span>{/if}
                  {#if g.is_locked}<span class="chip lock">🔒</span>{/if}
                  {#if g.is_deleted}<span class="chip archived">archived</span>{/if}
                </div>
                <div class="se-meta">
                  <span>{g.chat_count ?? 0} chats</span>
                  <span>{relativeTime(Number(g.updated_at))}</span>
                </div>
                {#if (g.tags ?? []).length > 0}
                  <div class="tag-row">
                    {#each g.tags ?? [] as tag}
                      <span class="tag-chip">#{tag}</span>
                    {/each}
                  </div>
                {/if}
              </button>
            {/each}
          {/if}
        </div>
      </section>
    </div>

    <div class="detail-grid">
      <section class="detail-card">
        <h4>Selected Chat</h4>
        {#if selectedChat}
          <div class="input-row">
            <input class="session-title" bind:this={titleInput} bind:value={sessionTitle} placeholder="Chat title" disabled={loading} />
            <button class="btn-sm" on:click={renameChat} disabled={loading}>Rename</button>
            <button class="btn-sm" on:click={saveCurrentChat} disabled={loading}>Save</button>
            <button class="btn-sm" on:click={loadChat} disabled={loading}>Load</button>
          </div>
          <div class="input-row">
            <input class="session-title" bind:value={chatTagInput} placeholder="Tags (comma separated)" disabled={loading} />
            <button class="btn-sm" on:click={() => updateChatMeta({ tags: parseTags(chatTagInput) })} disabled={loading}>Set Tags</button>
          </div>
          <div class="action-row">
            <button class="btn-sm" on:click={() => updateChatMeta({ is_favorite: !(selectedChat.is_favorite ?? false) })} disabled={loading}>
              {(selectedChat.is_favorite ?? false) ? 'Unheart' : 'Heart'}
            </button>
            <button class="btn-sm" on:click={() => updateChatMeta({ is_locked: !(selectedChat.is_locked ?? false) })} disabled={loading}>
              {(selectedChat.is_locked ?? false) ? 'Unlock' : 'Lock'}
            </button>
            <button class="btn-sm" on:click={() => updateChatMeta({ is_deleted: !(selectedChat.is_deleted ?? false) })} disabled={loading}>
              {(selectedChat.is_deleted ?? false) ? 'Restore' : 'Archive'}
            </button>
            <button class="btn-sm" on:click={duplicateChat} disabled={loading}>Duplicate</button>
            {#if deleteConfirmation}
              <button class="btn-sm danger" on:click={permanentlyDeleteChat} disabled={loading}>Confirm delete</button>
              <button class="btn-sm cancel" on:click={cancelDeleteChat} disabled={loading}>Cancel</button>
            {:else}
              <button class="btn-sm danger" on:click={requestDeleteChat} disabled={loading || (selectedChat.is_locked ?? false)}>Delete forever</button>
            {/if}
          </div>
          <div class="meta-row">
            <span>Updated {formatSessionDate(Number(selectedChat.updated_at))}</span>
            {#if selectedChat.workspace_path}
              <span>Path: {selectedChat.workspace_path}</span>
            {/if}
          </div>
        {:else}
          <div class="session-empty">Select a chat to manage.</div>
        {/if}
      </section>

      <section class="detail-card">
        <h4>Selected Session</h4>
        {#if selectedGroup}
          <div class="input-row">
            <input class="session-title" value={selectedGroup.title} on:change={onGroupTitleChange} placeholder="Session title" disabled={loading} />
          </div>
          <div class="input-row">
            <input class="session-title" bind:value={groupTagInput} placeholder="Tags (comma separated)" disabled={loading} />
            <button class="btn-sm" on:click={() => updateGroupMeta({ tags: parseTags(groupTagInput) })} disabled={loading}>Set Tags</button>
          </div>
          <div class="action-row">
            <button class="btn-sm" on:click={() => updateGroupMeta({ is_favorite: !(selectedGroup.is_favorite ?? false) })} disabled={loading}>
              {(selectedGroup.is_favorite ?? false) ? 'Unheart' : 'Heart'}
            </button>
            <button class="btn-sm" on:click={() => updateGroupMeta({ is_locked: !(selectedGroup.is_locked ?? false) })} disabled={loading}>
              {(selectedGroup.is_locked ?? false) ? 'Unlock' : 'Lock'}
            </button>
            <button class="btn-sm" on:click={() => updateGroupMeta({ is_deleted: !(selectedGroup.is_deleted ?? false) })} disabled={loading}>
              {(selectedGroup.is_deleted ?? false) ? 'Restore' : 'Archive'}
            </button>
          </div>
          <div class="meta-row">
            <span>{selectedGroup.chat_count ?? 0} chats linked</span>
            <span>Updated {formatSessionDate(Number(selectedGroup.updated_at))}</span>
          </div>
        {:else}
          <div class="session-empty">Select a session to manage.</div>
        {/if}
      </section>
    </div>

    <section class="linking-card">
      <h4>Linking</h4>
      <div class="action-row">
        <button class="btn-sm" on:click={linkSelectedChatToSelectedGroup} disabled={loading || !selectedChatId || !selectedGroupId}>
          Attach selected chat to selected session
        </button>
      </div>

      {#if selectedChat && (selectedChat.group_ids ?? []).length > 0}
        <div class="tag-row">
          {#each selectedChat.group_ids ?? [] as groupId}
            {@const g = groups.find((x) => x.id === groupId)}
            <span class="linked-chip">
              {g?.title ?? groupId}
              <button class="chip-x" on:click={() => unlinkChatFromGroup(groupId, selectedChat.id)} disabled={loading} aria-label="Detach">×</button>
            </span>
          {/each}
        </div>
      {:else}
        <div class="session-empty">Selected chat is not linked to any session yet.</div>
      {/if}
    </section>

    {#if sessionError}
      <div class="session-error">{sessionError}</div>
    {/if}
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
    z-index: var(--z-modal, 800);
    isolation: isolate;
    padding: 18px;
  }

  .session-panel {
    width: min(1080px, 100%);
    max-height: min(92vh, 980px);
    overflow: auto;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 18px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
    padding-bottom: 18px;
  }

  .panel-header {
    position: sticky;
    top: 0;
    z-index: 2;
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

  .current-session {
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-dim);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 18px;
    cursor: pointer;
  }

  .manager-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 20px;
  }

  .archive-toggle {
    display: inline-flex;
    gap: 7px;
    align-items: center;
    color: var(--text-dim);
    font-size: 12px;
  }

  .manager-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    padding: 0 20px;
  }

  .panel-col {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 220px;
  }

  .panel-col h3,
  .detail-card h4,
  .linking-card h4 {
    margin: 0;
    font-size: 13px;
    color: var(--text);
  }

  .search-row,
  .group-create-row,
  .input-row,
  .action-row,
  .meta-row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .session-title,
  .session-search {
    width: 100%;
    min-width: 140px;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text);
    padding: 9px 11px;
    font-size: 13px;
  }

  .session-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 260px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 6px;
    background: var(--bg2);
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

  .session-entry.archived {
    opacity: 0.7;
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

  .se-meta {
    display: flex;
    gap: 8px;
    margin-top: 3px;
    font-size: 11px;
    color: var(--text-dim);
    flex-wrap: wrap;
  }

  .se-ws {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip {
    font-size: 10px;
    border-radius: 999px;
    padding: 1px 6px;
  }

  .chip.fav {
    background: rgba(236, 72, 153, 0.18);
    color: #fbcfe8;
  }

  .chip.lock {
    background: rgba(250, 204, 21, 0.2);
    color: #fde68a;
  }

  .chip.archived {
    background: rgba(156, 163, 175, 0.2);
    color: #d1d5db;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    padding: 12px 20px 0;
  }

  .detail-card,
  .linking-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .linking-card {
    margin: 12px 20px 0;
  }

  .tag-row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .tag-chip {
    font-size: 11px;
    border: 1px solid rgba(34, 197, 94, 0.35);
    color: #bbf7d0;
    background: rgba(34, 197, 94, 0.14);
    border-radius: 999px;
    padding: 2px 8px;
  }

  .linked-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    border: 1px solid rgba(59, 130, 246, 0.35);
    color: #bfdbfe;
    background: rgba(59, 130, 246, 0.16);
    border-radius: 999px;
    padding: 2px 8px;
  }

  .chip-x {
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font-size: 12px;
    padding: 0;
    line-height: 1;
  }

  .btn-sm {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 7px 11px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-sm:hover:not(:disabled) { opacity: 0.9; }
  .btn-sm:disabled { opacity: 0.55; cursor: not-allowed; }
  .btn-sm.danger { background: var(--red); }
  .btn-sm.cancel { background: var(--bg-hover); color: var(--text); border: 1px solid var(--border); }

  .session-empty {
    color: var(--text-dim);
    font-size: 12px;
    padding: 6px 0;
  }

  .session-error {
    margin: 12px 20px 0;
    color: var(--red);
    font-size: 13px;
  }

  .meta-row {
    font-size: 12px;
    color: var(--text-dim);
  }

  @media (max-width: 900px) {
    .manager-grid,
    .detail-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
