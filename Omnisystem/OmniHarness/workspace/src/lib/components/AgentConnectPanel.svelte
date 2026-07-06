<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { currentWorkspace } from '$lib/stores/workspace';

  type AgentConnectSession = {
    id: string;
    goal?: string | null;
    workspace_path?: string | null;
    status: string;
    created_at_ms: number;
    updated_at_ms: number;
    last_event_summary?: string | null;
  };

  type AgentConnectEvent = {
    seq: number;
    session_id: string;
    event_type: string;
    summary: string;
    details: Record<string, unknown>;
    ts_ms: number;
  };

  const dispatch = createEventDispatcher<{ close: void }>();

  let goalInput = '';
  let loading = false;
  let error = '';
  let sessions: AgentConnectSession[] = [];
  let activeSession: AgentConnectSession | null = null;
  let selectedSessionId = '';
  let timeline: AgentConnectEvent[] = [];
  let filteredTimeline: AgentConnectEvent[] = [];
  let unlistenEvent: (() => void) | null = null;
  let timelineFilter: 'all' | 'chat' | 'tool' | 'hitl' | 'session' = 'all';

  function eventMatchesFilter(ev: AgentConnectEvent): boolean {
    if (timelineFilter === 'all') return true;
    return ev.event_type.startsWith(`${timelineFilter}.`);
  }

  $: filteredTimeline = timeline.filter(eventMatchesFilter);

  function close() {
    dispatch('close');
  }

  function formatTime(ms: number): string {
    return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  async function refreshSessions() {
    sessions = await invoke<AgentConnectSession[]>('agent_connect_list_sessions');
    activeSession = await invoke<AgentConnectSession | null>('agent_connect_get_active_session');
    if (!selectedSessionId) selectedSessionId = activeSession?.id ?? '';
    if (selectedSessionId && !sessions.some((s) => s.id === selectedSessionId)) {
      selectedSessionId = activeSession?.id ?? '';
    }
  }

  async function refreshTimeline(afterSeq?: number) {
    const sessionId = selectedSessionId || activeSession?.id;
    if (!sessionId) {
      timeline = [];
      return;
    }

    const events = await invoke<AgentConnectEvent[]>('agent_connect_get_timeline', {
      sessionId,
      afterSeq,
      limit: 300,
    });

    if (afterSeq && events.length > 0) {
      timeline = [...timeline, ...events].slice(-300);
      return;
    }
    timeline = events;
  }

  async function startSession() {
    loading = true;
    error = '';
    try {
      const session = await invoke<AgentConnectSession>('agent_connect_start_session', {
        goal: goalInput.trim() || null,
        workspacePath: $currentWorkspace?.path ?? null,
      });
      selectedSessionId = session.id;
      goalInput = '';
      await refreshSessions();
      await refreshTimeline();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function setActiveSession(sessionId: string) {
    loading = true;
    error = '';
    try {
      await invoke<AgentConnectSession>('agent_connect_set_active_session', { sessionId });
      selectedSessionId = sessionId;
      await refreshSessions();
      await refreshTimeline();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function endSession() {
    loading = true;
    error = '';
    try {
      await invoke<AgentConnectSession>('agent_connect_end_session', {
        sessionId: selectedSessionId || null,
        status: 'completed',
      });
      await refreshSessions();
      await refreshTimeline();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      await refreshSessions();
      await refreshTimeline();
      unlistenEvent = await listen<AgentConnectEvent>('agent-connect-event', async (evt) => {
        const payload = evt.payload;
        const selected = selectedSessionId || activeSession?.id;
        if (!selected) {
          await refreshSessions();
          return;
        }
        if (payload.session_id === selected) {
          const lastSeq = timeline.length ? timeline[timeline.length - 1].seq : 0;
          if (payload.seq > lastSeq) {
            timeline = [...timeline, payload].slice(-300);
          }
        }
        await refreshSessions();
      });
    } catch (e) {
      error = String(e);
    }
  });

  onDestroy(() => {
    if (unlistenEvent) unlistenEvent();
  });
</script>

<div class="agent-connect-overlay" on:click|self={close} role="presentation">
  <section class="agent-connect-panel" role="dialog" aria-modal="true" aria-label="Agent Connect Timeline">
    <header class="panel-header">
      <h2>Agent Connect</h2>
      <button class="close-btn" on:click={close} aria-label="Close Agent Connect">✕</button>
    </header>

    <div class="controls">
      <input
        bind:value={goalInput}
        placeholder="Session goal (optional)"
        aria-label="Session goal"
        disabled={loading}
      />
      <button on:click={startSession} disabled={loading}>Start Session</button>
      <button on:click={() => refreshSessions()} disabled={loading}>Refresh</button>
      <button on:click={endSession} disabled={loading || !(selectedSessionId || activeSession?.id)}>End Session</button>
    </div>

    <div class="meta-row">
      <div><strong>Active:</strong> {activeSession ? activeSession.id : 'none'}</div>
      <div><strong>Workspace:</strong> {$currentWorkspace?.name ?? 'none'}</div>
    </div>

    <div class="timeline-filters">
      <span>Filter</span>
      <select bind:value={timelineFilter} aria-label="Timeline filter">
        <option value="all">All</option>
        <option value="session">Session</option>
        <option value="chat">Chat</option>
        <option value="tool">Tool</option>
        <option value="hitl">HITL</option>
      </select>
    </div>

    <div class="session-list">
      {#if sessions.length === 0}
        <div class="empty">No Agent Connect sessions yet.</div>
      {:else}
        {#each sessions as s (s.id)}
          <button
            class="session-item"
            class:selected={s.id === (selectedSessionId || activeSession?.id)}
            on:click={() => setActiveSession(s.id)}
            disabled={loading}
            title={s.id}
          >
            <div class="session-main">
              <span>{s.id}</span>
              <span class="status">{s.status}</span>
            </div>
            <div class="session-sub">{s.last_event_summary ?? 'No events yet'}</div>
          </button>
        {/each}
      {/if}
    </div>

    <div class="timeline">
      {#if filteredTimeline.length === 0}
        <div class="empty">No events yet for selected session.</div>
      {:else}
        {#each filteredTimeline as ev (ev.seq)}
          <article class="event-row">
            <div class="event-top">
              <span class="event-type">{ev.event_type}</span>
              <span class="event-time">{formatTime(ev.ts_ms)}</span>
            </div>
            <div class="event-summary">{ev.summary}</div>
          </article>
        {/each}
      {/if}
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}
  </section>
</div>

<style>
  .agent-connect-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 25;
    padding: 16px;
  }

  .agent-connect-panel {
    width: min(860px, 100%);
    max-height: min(88vh, 860px);
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .close-btn {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    padding: 4px 8px;
    cursor: pointer;
  }

  .controls {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    gap: 8px;
  }

  .controls input,
  .controls button {
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    padding: 8px 10px;
    font-size: 12px;
  }

  .controls button {
    cursor: pointer;
  }

  .meta-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-dim);
  }

  .timeline-filters {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 12px;
    color: var(--text-dim);
  }

  .timeline-filters select {
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    padding: 6px 8px;
    font-size: 12px;
  }

  .session-list {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    max-height: 140px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
    background: var(--bg);
  }

  .session-item {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    text-align: left;
    padding: 8px;
    cursor: pointer;
    min-height: 58px;
  }

  .session-item.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px color-mix(in oklab, var(--accent), transparent 45%);
  }

  .session-main {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    gap: 8px;
  }

  .status {
    color: var(--accent-hl);
    text-transform: uppercase;
    font-size: 10px;
  }

  .session-sub {
    margin-top: 4px;
    font-size: 11px;
    color: var(--text-dim);
  }

  .timeline {
    flex: 1;
    min-height: 220px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    overflow: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .event-row {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
    background: color-mix(in oklab, var(--bg2), black 4%);
  }

  .event-top {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-dim);
  }

  .event-type {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    color: var(--accent-hl);
  }

  .event-summary {
    margin-top: 6px;
    font-size: 12px;
  }

  .empty {
    color: var(--text-dim);
    font-size: 12px;
    padding: 8px;
  }

  .error {
    color: var(--red);
    font-size: 12px;
    border: 1px solid color-mix(in oklab, var(--red), transparent 65%);
    border-radius: 8px;
    padding: 8px;
    background: color-mix(in oklab, var(--red), transparent 92%);
  }
</style>
