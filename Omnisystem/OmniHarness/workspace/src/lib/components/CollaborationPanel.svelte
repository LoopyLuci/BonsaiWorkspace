<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import GroupChatPanel from './GroupChatPanel.svelte';
  import CallPanel from './CallPanel.svelte';
  import SharedTerminal from './SharedTerminal.svelte';

  // ── session state ─────────────────────────────────────────────────────────────
  let sessionId = '';
  let peerId = '';
  let displayName = '';
  let inviteCode = '';
  let participants: any[] = [];
  let phase: 'setup' | 'session' = 'setup';
  let activeTab: 'chat' | 'call' | 'terminal' = 'chat';
  let isHost = false;
  let copied = false;

  // setup form
  let setupMode: 'create' | 'join' = 'create';
  let formName = '';
  let formCode = '';
  let busy = false;
  let error = '';

  let unlisteners: Array<() => void> = [];

  onMount(async () => {
    unlisteners.push(await listen('collab-participant-joined', (e: any) => {
      if (!participants.find((p: any) => p.peer_id === e.payload.peer_id)) {
        participants = [...participants, e.payload];
      }
    }));
    unlisteners.push(await listen('collab-participant-left', (e: any) => {
      participants = participants.filter((p: any) => p.peer_id !== e.payload.peer_id);
    }));
    unlisteners.push(await listen('collab-session-closed', () => {
      leaveSession();
    }));
  });

  onDestroy(() => unlisteners.forEach(fn => fn()));

  async function createSession() {
    if (!formName.trim()) { error = 'Display name required'; return; }
    busy = true; error = '';
    try {
      const result: any = await invoke('create_collaboration_session', {
        hostName: formName.trim(),
        allowEdit: true,
        allowVoice: true,
        allowVideo: true,
      });
      sessionId = result.session_id;
      inviteCode = result.invitation_code;
      peerId = result.participants[0].peer_id;
      displayName = formName.trim();
      participants = result.participants;
      isHost = true;
      phase = 'session';
    } catch (e: any) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function joinSession() {
    if (!formName.trim() || !formCode.trim()) { error = 'Name and code required'; return; }
    busy = true; error = '';
    try {
      const result: any = await invoke('join_collaboration_session', {
        invitationCode: formCode.trim(),
        displayName: formName.trim(),
      });
      sessionId = result.session_id;
      inviteCode = result.invitation_code;
      const me = result.participants[result.participants.length - 1];
      peerId = me.peer_id;
      displayName = formName.trim();
      participants = result.participants;
      isHost = false;
      phase = 'session';
    } catch (e: any) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function leaveSession() {
    if (sessionId && peerId) {
      await invoke('leave_collaboration_session', { sessionId, peerId }).catch(() => {});
    }
    sessionId = ''; peerId = ''; inviteCode = ''; participants = [];
    phase = 'setup'; isHost = false;
  }

  async function closeSession() {
    if (sessionId) await invoke('close_collaboration_session', { sessionId }).catch(() => {});
    leaveSession();
  }

  async function copyInvite() {
    await navigator.clipboard.writeText(inviteCode);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="collab-panel" data-workspace-action="Collaboration:Panel">
  {#if phase === 'setup'}
    <!-- ── Setup ─────────────────────────────────────────── -->
    <div class="setup">
      <h3 class="setup-title">Collaborate</h3>

      <div class="mode-tabs">
        <button class:active={setupMode === 'create'} on:click={() => (setupMode = 'create')}>Create</button>
        <button class:active={setupMode === 'join'} on:click={() => (setupMode = 'join')}>Join</button>
      </div>

      <label class="field">
        <span>Your name</span>
        <input bind:value={formName} placeholder="Display name" />
      </label>

      {#if setupMode === 'join'}
        <label class="field">
          <span>Invitation code</span>
          <input bind:value={formCode} placeholder="word-word-0000" />
        </label>
      {/if}

      {#if error}<p class="err">{error}</p>{/if}

      <button
        class="action-btn"
        on:click={setupMode === 'create' ? createSession : joinSession}
        disabled={busy}
      >
        {busy ? '…' : setupMode === 'create' ? 'Create Session' : 'Join Session'}
      </button>
    </div>
  {:else}
    <!-- ── Active session ──────────────────────────────────── -->
    <div class="session-header">
      <div class="invite-row">
        <span class="invite-label">Code:</span>
        <code class="invite-code">{inviteCode}</code>
        <button class="copy-btn" on:click={copyInvite} title="Copy">{copied ? '✓' : '📋'}</button>
      </div>
      <div class="header-actions">
        <span class="peer-count">👥 {participants.length}</span>
        {#if isHost}
          <button class="danger-btn" on:click={closeSession}>End</button>
        {:else}
          <button class="danger-btn" on:click={leaveSession}>Leave</button>
        {/if}
      </div>
    </div>

    <div class="tab-bar">
      <button class:active={activeTab === 'chat'} on:click={() => (activeTab = 'chat')}>💬 Chat</button>
      <button class:active={activeTab === 'call'} on:click={() => (activeTab = 'call')}>📞 Call</button>
      <button class:active={activeTab === 'terminal'} on:click={() => (activeTab = 'terminal')}>⌨ Terminal</button>
    </div>

    <div class="tab-content">
      {#if activeTab === 'chat'}
        <GroupChatPanel {sessionId} {peerId} {displayName} />
      {:else if activeTab === 'call'}
        <CallPanel {sessionId} {peerId} {participants} />
      {:else}
        <SharedTerminal {sessionId} />
      {/if}
    </div>
  {/if}
</div>

<style>
  .collab-panel { display: flex; flex-direction: column; height: 100%; font-size: 0.875rem; }

  /* ── Setup ── */
  .setup { display: flex; flex-direction: column; gap: 0.75rem; padding: 1.25rem; max-width: 320px; margin: 0 auto; width: 100%; }
  .setup-title { font-size: 1rem; font-weight: 700; margin: 0 0 0.25rem; }

  .mode-tabs { display: flex; border-radius: 8px; overflow: hidden; border: 1px solid var(--border, #333); }
  .mode-tabs button { flex: 1; padding: 0.4rem; background: none; border: none; cursor: pointer; color: inherit; font-size: 0.82rem; }
  .mode-tabs button.active { background: var(--accent, #3b82f6); color: #fff; }

  .field { display: flex; flex-direction: column; gap: 0.25rem; }
  .field span { font-size: 0.78rem; opacity: 0.7; }
  .field input { padding: 0.4rem 0.65rem; border-radius: 6px; border: 1px solid var(--border, #333); background: var(--bg-secondary, #1e1e2e); color: inherit; font-size: 0.875rem; }

  .err { color: #f87171; font-size: 0.8rem; margin: 0; }

  .action-btn { padding: 0.55rem; border-radius: 8px; border: none; background: var(--accent, #3b82f6); color: #fff; cursor: pointer; font-size: 0.9rem; }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  /* ── Session header ── */
  .session-header { display: flex; justify-content: space-between; align-items: center; padding: 0.45rem 0.75rem; background: var(--bg-secondary, #1e1e2e); border-bottom: 1px solid var(--border, #333); flex-shrink: 0; }
  .invite-row { display: flex; align-items: center; gap: 0.4rem; }
  .invite-label { font-size: 0.72rem; opacity: 0.6; }
  .invite-code { font-size: 0.78rem; font-family: monospace; background: var(--bg-primary, #13131f); padding: 0.15rem 0.4rem; border-radius: 4px; }
  .copy-btn { background: none; border: none; cursor: pointer; font-size: 0.8rem; opacity: 0.6; }
  .copy-btn:hover { opacity: 1; }
  .header-actions { display: flex; align-items: center; gap: 0.5rem; }
  .peer-count { font-size: 0.78rem; opacity: 0.7; }
  .danger-btn { padding: 0.2rem 0.6rem; border-radius: 5px; border: 1px solid #ef4444; background: none; color: #ef4444; font-size: 0.75rem; cursor: pointer; }
  .danger-btn:hover { background: #ef4444; color: #fff; }

  /* ── Tabs ── */
  .tab-bar { display: flex; border-bottom: 1px solid var(--border, #333); flex-shrink: 0; }
  .tab-bar button { flex: 1; padding: 0.45rem; background: none; border: none; cursor: pointer; color: inherit; font-size: 0.8rem; opacity: 0.6; border-bottom: 2px solid transparent; }
  .tab-bar button.active { opacity: 1; border-bottom-color: var(--accent, #3b82f6); }

  .tab-content { flex: 1; min-height: 0; overflow: hidden; display: flex; flex-direction: column; }
</style>
