<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let onClose: () => void = () => {};

  // ── Types ──────────────────────────────────────────────────────────────────

  interface TrainingRun {
    id: string;
    started_at: number;
    finished_at: number | null;
    base_model: string;
    data_path: string | null;
    adapter_path: string | null;
    status: string;
    metrics: string | null;
    total_examples: number | null;
    curated_examples: number | null;
  }

  interface InferenceStats {
    total_requests: number;
    success_rate: number;
    avg_latency_ms: number;
    total_prompt_tokens: number;
    total_completion_tokens: number;
    window_hours: number;
  }

  interface CoreStats {
    adapter_loaded: boolean;
    avg_latency_ms: number;
    curator_buffered: number;
    curator_total_seen: number;
    memory_entries: number;
    queue_depth: number;
    active_tasks: number;
  }

  // ── State ──────────────────────────────────────────────────────────────────

  type Tab = 'runs' | 'inference' | 'curated';
  let activeTab: Tab = 'runs';

  let runs: TrainingRun[] = [];
  let infStats: InferenceStats | null = null;
  let coreStats: CoreStats | null = null;
  let curatedLines: string[] = [];
  let curatedFilter = '';

  let loading = false;
  let error = '';

  let apiPort = 11373;
  let pairToken = '';

  // ── Helpers ────────────────────────────────────────────────────────────────

  function fmtTs(ts: number | null): string {
    if (!ts) return '—';
    return new Date(ts * 1000).toLocaleString();
  }

  function fmtDuration(run: TrainingRun): string {
    if (!run.finished_at) return 'running…';
    const secs = run.finished_at - run.started_at;
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
    return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  }

  function statusClass(status: string): string {
    if (status === 'completed') return 'status-ok';
    if (status === 'failed') return 'status-err';
    return 'status-running';
  }

  async function apiGet(path: string): Promise<unknown> {
    const res = await fetch(`http://127.0.0.1:${apiPort}${path}`, {
      headers: { Authorization: `Bearer ${pairToken}` },
    });
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    return res.json();
  }

  async function apiText(path: string): Promise<string> {
    const res = await fetch(`http://127.0.0.1:${apiPort}${path}`, {
      headers: { Authorization: `Bearer ${pairToken}` },
    });
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    return res.text();
  }

  // ── Data loading ───────────────────────────────────────────────────────────

  async function loadRuns() {
    const data = await apiGet('/api/v1/telemetry/training') as { runs: TrainingRun[] };
    runs = data.runs ?? [];
  }

  async function loadInference() {
    [infStats, coreStats] = await Promise.all([
      apiGet('/api/v1/telemetry/inference') as Promise<InferenceStats>,
      apiGet('/api/v1/core/stats') as Promise<CoreStats>,
    ]);
  }

  async function loadCurated() {
    const text = await apiText('/api/v1/telemetry/curated');
    curatedLines = text.split('\n').filter(l => l.trim());
  }

  async function refresh() {
    error = '';
    loading = true;
    try {
      if (activeTab === 'runs')      await loadRuns();
      if (activeTab === 'inference') await loadInference();
      if (activeTab === 'curated')   await loadCurated();
    } catch (e: unknown) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function flushCurator() {
    await fetch(`http://127.0.0.1:${apiPort}/api/v1/curator/flush`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${pairToken}` },
    });
    await loadCurated();
    await loadInference();
  }

  function downloadCurated() {
    const blob = new Blob([curatedLines.join('\n')], { type: 'application/x-ndjson' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'curated_examples.jsonl';
    a.click();
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  onMount(async () => {
    try {
      const cfg = await invoke<{ api_port: number; pair_token: string }>('get_api_config');
      apiPort    = cfg.api_port;
      pairToken  = cfg.pair_token ?? '';
    } catch {
      // Use defaults if invoke fails
    }
    await refresh();
  });

  $: filteredLines = curatedFilter
    ? curatedLines.filter(l => l.toLowerCase().includes(curatedFilter.toLowerCase()))
    : curatedLines;
</script>

<!-- ── Markup ───────────────────────────────────────────────────────────────── -->

<div class="dashboard">
  <div class="header">
    <h2>Training Dashboard</h2>
    <div class="header-actions">
      <button class="btn-icon" on:click={refresh} disabled={loading} title="Refresh">
        {loading ? '…' : '↺'}
      </button>
      <button class="btn-icon close-btn" on:click={onClose} title="Close">✕</button>
    </div>
  </div>

  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  <!-- Tabs -->
  <div class="tabs">
    <button class="tab" class:active={activeTab === 'runs'}
      on:click={() => { activeTab = 'runs'; refresh(); }}>
      Training Runs
    </button>
    <button class="tab" class:active={activeTab === 'inference'}
      on:click={() => { activeTab = 'inference'; refresh(); }}>
      Inference Stats
    </button>
    <button class="tab" class:active={activeTab === 'curated'}
      on:click={() => { activeTab = 'curated'; refresh(); }}>
      Curated Data
      {#if coreStats}
        <span class="badge">{coreStats.curator_total_seen}</span>
      {/if}
    </button>
  </div>

  <!-- Training Runs -->
  {#if activeTab === 'runs'}
    {#if runs.length === 0}
      <p class="empty">No training runs yet. Start one from Settings → Advanced → Train New Adapter.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Base Model</th>
              <th>Status</th>
              <th>Started</th>
              <th>Duration</th>
              <th>Examples</th>
              <th>Adapter</th>
            </tr>
          </thead>
          <tbody>
            {#each runs as run}
              <tr>
                <td class="mono small">{run.id}</td>
                <td class="small">{run.base_model.split('/').at(-1)}</td>
                <td><span class="status-pill {statusClass(run.status)}">{run.status}</span></td>
                <td class="small">{fmtTs(run.started_at)}</td>
                <td class="small">{fmtDuration(run)}</td>
                <td class="small">{run.total_examples ?? '—'} (+{run.curated_examples ?? 0} curated)</td>
                <td class="mono small">{run.adapter_path ? run.adapter_path.split('\\').at(-1) : '—'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}

  <!-- Inference Stats -->
  {#if activeTab === 'inference'}
    {#if coreStats}
      <div class="stat-grid">
        <div class="stat-card">
          <div class="stat-label">Adapter Loaded</div>
          <div class="stat-value" class:ok={coreStats.adapter_loaded}
            class:warn={!coreStats.adapter_loaded}>
            {coreStats.adapter_loaded ? 'Yes' : 'No'}
          </div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Curator Seen</div>
          <div class="stat-value">{coreStats.curator_total_seen}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Buffered</div>
          <div class="stat-value">{coreStats.curator_buffered}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Memory Entries</div>
          <div class="stat-value">{coreStats.memory_entries}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Queue Depth</div>
          <div class="stat-value">{coreStats.queue_depth}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Avg Latency</div>
          <div class="stat-value">{coreStats.avg_latency_ms.toFixed(1)} ms</div>
        </div>
      </div>
    {/if}
    {#if infStats}
      <h3 class="section-title">Last {infStats.window_hours}h Inference</h3>
      <div class="stat-grid">
        <div class="stat-card">
          <div class="stat-label">Total Requests</div>
          <div class="stat-value">{infStats.total_requests}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Success Rate</div>
          <div class="stat-value"
            class:ok={infStats.success_rate >= 0.95}
            class:warn={infStats.success_rate < 0.95}>
            {(infStats.success_rate * 100).toFixed(1)}%
          </div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Avg Latency</div>
          <div class="stat-value">{infStats.avg_latency_ms.toFixed(0)} ms</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Prompt Tokens</div>
          <div class="stat-value">{infStats.total_prompt_tokens.toLocaleString()}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Completion Tokens</div>
          <div class="stat-value">{infStats.total_completion_tokens.toLocaleString()}</div>
        </div>
      </div>
    {/if}
    {#if !coreStats && !infStats}
      <p class="empty">No inference data yet.</p>
    {/if}
  {/if}

  <!-- Curated Data -->
  {#if activeTab === 'curated'}
    <div class="curated-toolbar">
      <input class="search-input" placeholder="Filter examples…" bind:value={curatedFilter} />
      <button class="btn-sm" on:click={flushCurator}>Flush Buffer</button>
      <button class="btn-sm" on:click={downloadCurated} disabled={curatedLines.length === 0}>
        Download JSONL ({curatedLines.length})
      </button>
    </div>
    {#if filteredLines.length === 0}
      <p class="empty">No curated examples yet. Use BonsaiCore to generate some.</p>
    {:else}
      <div class="curated-list">
        {#each filteredLines.slice(0, 200) as line, i}
          <div class="curated-row">
            <span class="curated-idx">{i + 1}</span>
            <span class="curated-text">{line.slice(0, 200)}{line.length > 200 ? '…' : ''}</span>
          </div>
        {/each}
        {#if filteredLines.length > 200}
          <p class="empty">Showing first 200 of {filteredLines.length}. Download JSONL for full set.</p>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .dashboard {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface-1, #1a1a2e);
    color: var(--text-1, #e0e0e0);
    font-family: var(--font-sans, system-ui, sans-serif);
    border-radius: 8px;
    overflow: hidden;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border, #2a2a3e);
  }
  .header h2 { margin: 0; font-size: 1rem; font-weight: 600; }
  .header-actions { display: flex; gap: 6px; }

  .btn-icon {
    background: none;
    border: 1px solid var(--border, #2a2a3e);
    border-radius: 4px;
    color: inherit;
    cursor: pointer;
    padding: 4px 8px;
    font-size: 0.85rem;
  }
  .btn-icon:hover { background: var(--surface-2, #25253a); }
  .btn-icon:disabled { opacity: 0.4; cursor: default; }
  .close-btn { border-color: transparent; }

  .error-banner {
    padding: 8px 16px;
    background: #3d1a1a;
    color: #ff8080;
    font-size: 0.8rem;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border, #2a2a3e);
    padding: 0 8px;
  }
  .tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-2, #aaa);
    cursor: pointer;
    padding: 8px 14px;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tab:hover { color: var(--text-1, #e0e0e0); }
  .tab.active { color: var(--accent, #7c6bff); border-bottom-color: var(--accent, #7c6bff); }

  .badge {
    background: var(--accent, #7c6bff);
    color: #fff;
    border-radius: 10px;
    padding: 1px 7px;
    font-size: 0.72rem;
  }

  .empty {
    color: var(--text-2, #aaa);
    font-size: 0.85rem;
    padding: 24px 16px;
    text-align: center;
  }

  /* Table */
  .table-wrap { overflow: auto; flex: 1; }
  table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
  th {
    background: var(--surface-2, #25253a);
    padding: 8px 12px;
    text-align: left;
    font-weight: 600;
    font-size: 0.75rem;
    color: var(--text-2, #aaa);
    position: sticky;
    top: 0;
  }
  td { padding: 7px 12px; border-bottom: 1px solid var(--border, #2a2a3e); }
  tr:hover td { background: var(--surface-2, #25253a); }
  .mono { font-family: var(--font-mono, monospace); }
  .small { font-size: 0.78rem; }

  .status-pill {
    border-radius: 10px;
    padding: 2px 8px;
    font-size: 0.72rem;
    font-weight: 600;
  }
  .status-ok      { background: #1a3a1a; color: #6fbf6f; }
  .status-err     { background: #3a1a1a; color: #bf6f6f; }
  .status-running { background: #1a2a3a; color: #6f9fbf; }

  /* Stats */
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 10px;
    padding: 16px;
  }
  .stat-card {
    background: var(--surface-2, #25253a);
    border-radius: 6px;
    padding: 12px;
  }
  .stat-label { font-size: 0.72rem; color: var(--text-2, #aaa); margin-bottom: 4px; }
  .stat-value { font-size: 1.2rem; font-weight: 700; }
  .stat-value.ok   { color: #6fbf6f; }
  .stat-value.warn { color: #bf9f3f; }

  .section-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-2, #aaa);
    padding: 0 16px;
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Curated */
  .curated-toolbar {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border, #2a2a3e);
  }
  .search-input {
    flex: 1;
    background: var(--surface-2, #25253a);
    border: 1px solid var(--border, #2a2a3e);
    border-radius: 4px;
    color: inherit;
    font-size: 0.82rem;
    padding: 5px 10px;
  }
  .btn-sm {
    background: var(--surface-2, #25253a);
    border: 1px solid var(--border, #2a2a3e);
    border-radius: 4px;
    color: inherit;
    cursor: pointer;
    font-size: 0.78rem;
    padding: 5px 10px;
    white-space: nowrap;
  }
  .btn-sm:hover { background: var(--surface-3, #30304a); }
  .btn-sm:disabled { opacity: 0.4; cursor: default; }

  .curated-list { overflow: auto; flex: 1; }
  .curated-row {
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: 5px 16px;
    border-bottom: 1px solid var(--border, #2a2a3e);
    font-size: 0.78rem;
  }
  .curated-row:hover { background: var(--surface-2, #25253a); }
  .curated-idx { color: var(--text-2, #aaa); min-width: 28px; text-align: right; }
  .curated-text { font-family: var(--font-mono, monospace); word-break: break-all; }
</style>
