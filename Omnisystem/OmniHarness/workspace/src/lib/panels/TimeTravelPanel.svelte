<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  type EventCategory =
    | 'FileChange' | 'ConfigChange' | 'ModelChange' | 'AgentAction'
    | 'SwarmEvent' | 'CollaborationEvent' | 'ComputeEvent' | 'ExtensionEvent'
    | 'SurvivalEvent' | 'CreditTransaction' | 'Checkpoint' | 'Reversion';

  interface UniverseEvent {
    event_id: string;
    timestamp_ns: number;
    category: EventCategory;
    summary: string;
    target: string;
    before_hash: string | null;
    after_hash: string | null;
    delta_hash: string | null;
    metadata: unknown;
    source: unknown;
    device_id: string;
  }

  interface UniverseSnapshot {
    snapshot_id: string;
    timestamp_ns: number;
    label: string | null;
    trigger_event_id: string;
    event_count_at_creation: number;
    state_hashes: Record<string, string>;
  }

  interface RevertPreview {
    target_event_id: string | null;
    target_snapshot_id: string | null;
    affected_files: string[];
    affected_configs: string[];
    model_changes: string[];
    agent_changes: string[];
    event_count_to_undo: number;
    estimated_duration_ms: number;
  }

  let activeTab: 'timeline' | 'snapshots' = 'timeline';
  let events: UniverseEvent[] = [];
  let snapshots: UniverseSnapshot[] = [];
  let eventCount = 0;
  let loading = false;
  let categoryFilter: EventCategory | '' = '';
  let targetFilter = '';
  let selectedEvent: UniverseEvent | null = null;
  let revertPreview: RevertPreview | null = null;
  let confirmingRevert = false;
  let snapshotLabel = '';
  let creatingSnapshot = false;

  const CATEGORY_COLORS: Record<EventCategory, string> = {
    FileChange: '#6ee7b7',
    ConfigChange: '#93c5fd',
    ModelChange: '#c4b5fd',
    AgentAction: '#fde68a',
    SwarmEvent: '#fdba74',
    CollaborationEvent: '#67e8f9',
    ComputeEvent: '#a5f3fc',
    ExtensionEvent: '#d8b4fe',
    SurvivalEvent: '#f87171',
    CreditTransaction: '#4ade80',
    Checkpoint: '#e2e8f0',
    Reversion: '#fb923c',
  };

  const CATEGORY_ICONS: Record<EventCategory, string> = {
    FileChange: '📄', ConfigChange: '⚙️', ModelChange: '🧠', AgentAction: '🤖',
    SwarmEvent: '🐝', CollaborationEvent: '👥', ComputeEvent: '💻',
    ExtensionEvent: '🧩', SurvivalEvent: '🛡️', CreditTransaction: '💰',
    Checkpoint: '📸', Reversion: '⏪',
  };

  function categoryIcon(cat: string): string {
    return (CATEGORY_ICONS as Record<string, string>)[cat] ?? '●';
  }
  function categoryColor(cat: string): string {
    return (CATEGORY_COLORS as Record<string, string>)[cat] ?? '#888';
  }

  function formatNs(ns: number): string {
    const ms = Math.floor(ns / 1_000_000);
    return new Date(ms).toLocaleString();
  }

  function formatAgo(ns: number): string {
    const diffMs = Date.now() - Math.floor(ns / 1_000_000);
    if (diffMs < 60_000) return `${Math.floor(diffMs / 1000)}s ago`;
    if (diffMs < 3_600_000) return `${Math.floor(diffMs / 60_000)}m ago`;
    if (diffMs < 86_400_000) return `${Math.floor(diffMs / 3_600_000)}h ago`;
    return `${Math.floor(diffMs / 86_400_000)}d ago`;
  }

  async function loadTimeline() {
    loading = true;
    try {
      events = await invoke<UniverseEvent[]>('get_timeline', {
        category: categoryFilter || null,
        targetPrefix: targetFilter || null,
        sinceNs: null,
        untilNs: null,
        limit: 200,
      });
      eventCount = await invoke<number>('universe_event_count');
    } catch (e) {
      console.error('load timeline:', e);
    } finally {
      loading = false;
    }
  }

  async function loadSnapshots() {
    loading = true;
    try {
      snapshots = await invoke<UniverseSnapshot[]>('get_snapshots', { limit: 50 });
    } catch (e) {
      console.error('load snapshots:', e);
    } finally {
      loading = false;
    }
  }

  async function selectEvent(ev: UniverseEvent) {
    selectedEvent = selectedEvent?.event_id === ev.event_id ? null : ev;
    revertPreview = null;
    confirmingRevert = false;
  }

  async function previewRevertEvent(eventId: string) {
    try {
      revertPreview = await invoke<RevertPreview>('revert_preview_event', { eventId });
    } catch (e) {
      console.error('preview revert:', e);
    }
  }

  async function previewRevertSnapshot(snapshotId: string) {
    try {
      revertPreview = await invoke<RevertPreview>('revert_preview_snapshot', { snapshotId });
      confirmingRevert = true;
    } catch (e) {
      console.error('preview revert snapshot:', e);
    }
  }

  async function takeSnapshot() {
    creatingSnapshot = true;
    try {
      await invoke('create_snapshot', { label: snapshotLabel || null });
      snapshotLabel = '';
      await loadSnapshots();
    } catch (e) {
      console.error('create snapshot:', e);
    } finally {
      creatingSnapshot = false;
    }
  }

  onMount(() => {
    loadTimeline();
  });

  $: if (activeTab === 'timeline') loadTimeline();
  $: if (activeTab === 'snapshots') loadSnapshots();
</script>

<div class="tt-panel">
  <!-- Header -->
  <div class="tt-header">
    <div class="tt-title">
      <span class="tt-icon">⏱</span>
      Time Travel
      <span class="tt-count">{eventCount.toLocaleString()} events</span>
    </div>
    <div class="tt-tabs">
      <button class="tab" class:active={activeTab === 'timeline'} on:click={() => activeTab = 'timeline'}>
        Timeline
      </button>
      <button class="tab" class:active={activeTab === 'snapshots'} on:click={() => activeTab = 'snapshots'}>
        Snapshots
      </button>
    </div>
  </div>

  <!-- Timeline tab -->
  {#if activeTab === 'timeline'}
    <div class="tt-filters">
      <select bind:value={categoryFilter} on:change={loadTimeline}>
        <option value="">All categories</option>
        {#each Object.keys(CATEGORY_ICONS) as cat}
          <option value={cat}>{categoryIcon(cat)} {cat}</option>
        {/each}
      </select>
      <input
        bind:value={targetFilter}
        on:input={loadTimeline}
        placeholder="Filter by target…"
        class="filter-input"
      />
      <button class="btn-refresh" on:click={loadTimeline}>↻</button>
    </div>

    <div class="tt-timeline">
      {#if loading}
        <div class="tt-empty">Loading…</div>
      {:else if events.length === 0}
        <div class="tt-empty">No events recorded yet.</div>
      {:else}
        {#each events as ev (ev.event_id)}
          <button
            class="tt-event"
            class:selected={selectedEvent?.event_id === ev.event_id}
            on:click={() => selectEvent(ev)}
            data-workspace-action="select-event"
          >
            <span
              class="ev-dot"
              style="background:{categoryColor(ev.category)}"
            >
              {categoryIcon(ev.category)}
            </span>
            <div class="ev-body">
              <div class="ev-summary">{ev.summary}</div>
              <div class="ev-meta">
                <span class="ev-target">{ev.target}</span>
                <span class="ev-time">{formatAgo(ev.timestamp_ns)}</span>
              </div>
            </div>
          </button>

          {#if selectedEvent?.event_id === ev.event_id}
            <div class="ev-detail">
              <div class="detail-row"><b>ID:</b> <code>{ev.event_id.slice(0, 16)}…</code></div>
              <div class="detail-row"><b>Time:</b> {formatNs(ev.timestamp_ns)}</div>
              {#if ev.before_hash}<div class="detail-row"><b>Before:</b> <code class="hash">{ev.before_hash.slice(0, 12)}…</code></div>{/if}
              {#if ev.after_hash}<div class="detail-row"><b>After:</b> <code class="hash">{ev.after_hash.slice(0, 12)}…</code></div>{/if}
              {#if ev.delta_hash}<div class="detail-row"><b>Delta:</b> <code class="hash">{ev.delta_hash.slice(0, 12)}…</code></div>{/if}
              <div class="detail-actions">
                <button class="btn-revert" on:click={() => previewRevertEvent(ev.event_id)}>
                  ⏪ Preview revert to here
                </button>
              </div>

              {#if revertPreview && revertPreview.target_event_id === ev.event_id}
                <div class="revert-preview">
                  <div class="rp-title">Revert Preview — {revertPreview.event_count_to_undo} events to undo</div>
                  {#if revertPreview.affected_files.length}
                    <div class="rp-section"><b>Files:</b>
                      {#each revertPreview.affected_files.slice(0, 8) as f}<div class="rp-item">{f}</div>{/each}
                    </div>
                  {/if}
                  {#if revertPreview.affected_configs.length}
                    <div class="rp-section"><b>Configs:</b>
                      {#each revertPreview.affected_configs as c}<div class="rp-item">{c}</div>{/each}
                    </div>
                  {/if}
                  <div class="rp-note">
                    ⚠ Revert execution requires full implementation (Phase 2). This preview shows what would change.
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Snapshots tab -->
  {#if activeTab === 'snapshots'}
    <div class="snap-create">
      <input bind:value={snapshotLabel} placeholder="Snapshot label (optional)" class="filter-input" />
      <button class="btn-snap" on:click={takeSnapshot} disabled={creatingSnapshot}>
        {creatingSnapshot ? 'Creating…' : '📸 Create Snapshot'}
      </button>
    </div>

    <div class="snap-grid">
      {#if loading}
        <div class="tt-empty">Loading…</div>
      {:else if snapshots.length === 0}
        <div class="tt-empty">No snapshots yet. Create one above.</div>
      {:else}
        {#each snapshots as snap (snap.snapshot_id)}
          <div class="snap-card">
            <div class="snap-label">{snap.label ?? 'Auto snapshot'}</div>
            <div class="snap-time">{formatNs(snap.timestamp_ns)}</div>
            <div class="snap-meta">{snap.event_count_at_creation.toLocaleString()} events at creation</div>
            <button class="btn-revert" on:click={() => previewRevertSnapshot(snap.snapshot_id)}>
              ⏪ Restore
            </button>

            {#if revertPreview?.target_snapshot_id === snap.snapshot_id}
              <div class="revert-preview">
                <div class="rp-title">{revertPreview.event_count_to_undo} events to undo</div>
                {#if revertPreview.affected_files.length}
                  <b>Files changed since snapshot:</b>
                  {#each revertPreview.affected_files.slice(0, 5) as f}<div class="rp-item">{f}</div>{/each}
                {/if}
                <div class="rp-note">⚠ Full revert execution in Phase 2.</div>
              </div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tt-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0f0f1a;
    color: #e2e8f0;
    font-size: 13px;
  }
  .tt-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 8px;
    border-bottom: 1px solid #2d2d3f;
  }
  .tt-title { display: flex; align-items: center; gap: 8px; font-weight: 600; font-size: 15px; }
  .tt-icon { font-size: 18px; }
  .tt-count { font-size: 11px; color: #64748b; font-weight: 400; }
  .tt-tabs { display: flex; gap: 4px; }
  .tab {
    background: none; border: none; color: #64748b; padding: 4px 12px;
    border-radius: 6px; cursor: pointer; font-size: 13px;
  }
  .tab.active { background: #1e1e35; color: #c9b8ff; }
  .tt-filters {
    display: flex; gap: 8px; padding: 8px 16px;
    border-bottom: 1px solid #1e1e35;
  }
  .tt-filters select, .filter-input {
    background: #1a1a2e; border: 1px solid #2d2d3f; color: #e2e8f0;
    border-radius: 6px; padding: 4px 8px; font-size: 12px; flex: 1;
  }
  .btn-refresh {
    background: #1a1a2e; border: 1px solid #2d2d3f; color: #94a3b8;
    border-radius: 6px; padding: 4px 10px; cursor: pointer;
  }
  .tt-timeline { flex: 1; overflow-y: auto; padding: 8px 0; }
  .tt-empty { color: #4a5568; text-align: center; padding: 40px; }
  .tt-event {
    display: flex; align-items: flex-start; gap: 10px;
    padding: 8px 16px; cursor: pointer; width: 100%;
    background: none; border: none; color: inherit; text-align: left;
    border-left: 2px solid transparent; transition: background 0.1s;
  }
  .tt-event:hover { background: #1a1a2e; }
  .tt-event.selected { background: #1e1e35; border-left-color: #c9b8ff; }
  .ev-dot {
    width: 24px; height: 24px; border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    font-size: 12px; flex-shrink: 0; margin-top: 1px;
  }
  .ev-body { flex: 1; min-width: 0; }
  .ev-summary { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ev-meta { display: flex; gap: 12px; font-size: 11px; color: #64748b; margin-top: 2px; }
  .ev-target { max-width: 200px; overflow: hidden; text-overflow: ellipsis; }
  .ev-detail {
    background: #12122a; border-top: 1px solid #1e1e35;
    padding: 10px 16px 10px 52px; font-size: 12px;
  }
  .detail-row { margin-bottom: 4px; }
  .hash { color: #94a3b8; font-family: monospace; }
  .detail-actions { margin-top: 8px; }
  .btn-revert {
    background: #1e293b; border: 1px solid #334155; color: #c9b8ff;
    padding: 4px 12px; border-radius: 6px; cursor: pointer; font-size: 12px;
  }
  .btn-revert:hover { background: #2d3748; }
  .revert-preview {
    background: #0f172a; border: 1px solid #334155; border-radius: 8px;
    padding: 10px; margin-top: 8px;
  }
  .rp-title { font-weight: 600; color: #f59e0b; margin-bottom: 6px; }
  .rp-section { margin-bottom: 6px; }
  .rp-item { color: #94a3b8; font-family: monospace; font-size: 11px; padding: 1px 0 1px 12px; }
  .rp-note { color: #f87171; font-size: 11px; margin-top: 6px; }
  .snap-create {
    display: flex; gap: 8px; padding: 10px 16px;
    border-bottom: 1px solid #1e1e35;
  }
  .btn-snap {
    background: #4c1d95; border: none; color: #e9d5ff;
    padding: 6px 14px; border-radius: 6px; cursor: pointer; white-space: nowrap;
  }
  .btn-snap:disabled { opacity: 0.5; }
  .snap-grid { flex: 1; overflow-y: auto; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; }
  .snap-card {
    background: #1a1a2e; border: 1px solid #2d2d3f; border-radius: 10px;
    padding: 12px 14px;
  }
  .snap-label { font-weight: 600; margin-bottom: 4px; }
  .snap-time { font-size: 11px; color: #64748b; }
  .snap-meta { font-size: 11px; color: #94a3b8; margin-bottom: 8px; }
</style>
