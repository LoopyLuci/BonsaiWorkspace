<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';

  // ── Tab state ────────────────────────────────────────────────────────────
  type Tab = 'overview' | 'data' | 'history' | 'curriculum' | 'federated' | 'settings';
  let activeTab: Tab = 'overview';

  // ── Data ──────────────────────────────────────────────────────────────────
  let stats: any = null;
  let dimensions: any[] = [];
  let ciqHistory: any[] = [];
  let alerts: any[] = [];
  let examples: any[] = [];
  let exampleTotal = 0;
  let exampleOffset = 0;
  const exampleLimit = 30;
   const stageNames: Record<number,string> = {
     1: 'Foundation',
     2: 'Tool Mastery',
     3: 'Multi-Modal',
     4: 'Advanced Reasoning',
     5: 'Creative Generation',
     6: 'Collaborative Intelligence',
     7: 'Autonomous Operation'
   };
  let exampleDomain = '';
  let loopHistory: any[] = [];
  let preferences: any = null;
  let forgettingBaseline: any = null;
  let selfPlayState: any = null;
  let curriculumStatus: any = null;

  // ── UI state ──────────────────────────────────────────────────────────────
  let loading = false;
  let trainTriggering = false;
  let toast = '';
  let toastTimeout: ReturnType<typeof setTimeout>;
  let pollingInterval: ReturnType<typeof setInterval>;
  let bulkDeleteFrom = '';
  let bulkDeleteTo = '';
  let exportPath = '';

  // ── Toast helper ──────────────────────────────────────────────────────────
  function showToast(msg: string) {
    toast = msg;
    clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => (toast = ''), 3000);
  }

  // ── Data loading ──────────────────────────────────────────────────────────
  async function loadAll() {
    try {
      [stats, dimensions, loopHistory, preferences, curriculumStatus, selfPlayState] =
        await Promise.all([
          invoke('get_training_stats').catch(() => null),
          invoke('get_evaluation_results').catch(() => []),
          invoke('get_training_loop_history').catch(() => []),
          invoke('get_training_preferences').catch(() => ({})),
          invoke('get_curriculum_status').catch(() => null),
          invoke('get_self_play_state').catch(() => ({})),
        ]);
      alerts = stats?.alerts ?? [];
    } catch (e) {
      console.error('[TrainingDashboard] loadAll error:', e);
    }
  }

  async function loadExamples() {
    try {
      const res: any = await invoke('get_training_examples', {
        offset: exampleOffset,
        limit: exampleLimit,
        domain: exampleDomain || null,
      });
      examples = res?.examples ?? [];
      exampleTotal = res?.total ?? 0;
    } catch (e) {
      examples = [];
    }
  }

  async function loadCiqHistory() {
    ciqHistory = await invoke('get_ciq_history').catch(() => []);
  }

  onMount(async () => {
    await loadAll();
    await loadExamples();
    await loadCiqHistory();
    pollingInterval = setInterval(loadAll, 15_000);
  });

  onDestroy(() => {
    clearInterval(pollingInterval);
    clearTimeout(toastTimeout);
  });

  // ── Tab switching ─────────────────────────────────────────────────────────
  function setTab(t: Tab) {
    activeTab = t;
    if (t === 'data') loadExamples();
    if (t === 'history') loadCiqHistory();
  }

  // ── Actions ───────────────────────────────────────────────────────────────
  async function triggerTraining() {
    trainTriggering = true;
    try {
      await invoke('trigger_training_cycle');
      showToast('Training cycle triggered');
      setTimeout(loadAll, 2000);
    } catch (e: any) {
      showToast('Error: ' + e);
    } finally {
      trainTriggering = false;
    }
  }

  async function deleteExample(id: string) {
    await invoke('delete_training_example', { exampleId: id });
    showToast('Example deleted');
    await loadExamples();
  }

  async function boostExample(id: string) {
    await invoke('boost_training_example', { exampleId: id });
    showToast('Example boosted to max priority');
    await loadExamples();
  }

  async function bulkDelete() {
    const removed: number = await invoke('bulk_delete_training_data', {
      request: {
        from_timestamp: bulkDeleteFrom ? new Date(bulkDeleteFrom).getTime() * 1000 : null,
        to_timestamp:   bulkDeleteTo   ? new Date(bulkDeleteTo).getTime()   * 1000 : null,
        source_filter:  null,
        domain_filter:  exampleDomain || null,
      },
    });
    showToast(`Deleted ${removed} examples`);
    await loadExamples();
  }

  async function exportData() {
    if (!exportPath) return showToast('Enter an export path first');
    const count: number = await invoke('export_training_data', { outputPath: exportPath });
    showToast(`Exported ${count} examples to ${exportPath}`);
  }

  async function wipeDatabase() {
    if (!confirm('Wipe ALL training data? This cannot be undone.')) return;
    await invoke('wipe_training_database');
    showToast('Training database wiped');
    await loadAll();
    await loadExamples();
  }

  async function rollback() {
    try {
      const msg: string = await invoke('rollback_adapter');
      showToast(msg);
    } catch (e: any) {
      showToast('Rollback failed: ' + e);
    }
  }

  async function savePreferences() {
    await invoke('set_training_preferences', { prefs: preferences });
    showToast('Preferences saved');
  }

  // ── CIQ colour ────────────────────────────────────────────────────────────
  function ciqColor(v: number): string {
    if (v >= 0.90) return '#22c55e';
    if (v >= 0.75) return '#eab308';
    return '#ef4444';
  }

  function pct(v: number | undefined | null): string {
    if (v == null) return '—';
    return (v * 100).toFixed(1) + '%';
  }

  function trend(v: number | undefined | null): string {
    if (v == null) return '';
    if (v > 0.001) return '↗';
    if (v < -0.001) return '↘';
    return '↔';
  }

  function trendColor(v: number | undefined | null): string {
    if (v == null) return 'text-gray-500';
    if (v > 0.001) return 'text-green-400';
    if (v < -0.001) return 'text-red-400';
    return 'text-gray-400';
  }

  function sourceLabel(s: string): string {
    return s?.replace(/_/g, ' ') ?? '';
  }

  $: bufferTotal = stats?.collector?.buffer?.total ?? 0;
  $: bufferByDomain = stats?.collector?.buffer?.by_domain ?? {};
  $: qualityThreshold = stats?.collector?.buffer?.quality_threshold ?? 0.4;
  $: loopRunning = stats?.loop_running ?? false;
  $: currentCiq = stats?.ciq;

  // ── Reasoning performance ─────────────────────────────────────────────────
  let reasoningReport: any = null;
  let reasoningTraining = false;

  async function loadReasoningReport() {
    try {
      reasoningReport = await invoke('get_metacognitive_report').catch(() => null);
    } catch (_) { /* not available yet */ }
  }

  async function triggerReasoningTraining() {
    reasoningTraining = true;
    try {
      await invoke('train_reasoning');
      showToast('Reasoning training cycle queued');
    } catch (e: any) {
      showToast('Error: ' + e);
    } finally {
      reasoningTraining = false;
    }
  }

  onMount(async () => { loadReasoningReport(); });
  $: strategyRows = reasoningReport?.strategy_stats
    ? Object.entries(reasoningReport.strategy_stats as Record<string, any>)
    : [];
</script>

<!-- ── Layout ──────────────────────────────────────────────────────────────── -->
<div class="flex flex-col h-full bg-gray-950 text-gray-100 font-sans select-none">

  <!-- Header -->
  <div class="flex items-center justify-between px-5 py-3 border-b border-gray-800 shrink-0">
    <div class="flex items-center gap-3">
      <div class="w-2 h-2 rounded-full {loopRunning ? 'bg-green-400 animate-pulse' : 'bg-gray-600'}"></div>
      <span class="font-semibold text-white tracking-wide">BonsAI Continuous Training</span>
      {#if alerts.length > 0}
        <span class="bg-red-600 text-white text-xs font-bold px-2 py-0.5 rounded-full">{alerts.length} ALERT{alerts.length > 1 ? 'S' : ''}</span>
      {/if}
    </div>
    <button
      class="flex items-center gap-2 px-3 py-1.5 text-sm rounded bg-violet-600 hover:bg-violet-500 text-white transition disabled:opacity-50"
      on:click={triggerTraining}
      disabled={trainTriggering}>
      {#if trainTriggering}
        <svg class="w-3.5 h-3.5 animate-spin" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" stroke-dasharray="30 70"/></svg>
        Training…
      {:else}
        ⚡ Train Now
      {/if}
    </button>
  </div>

  <!-- Tabs -->
  <div class="flex gap-0.5 px-4 pt-2 border-b border-gray-800 shrink-0 text-sm">
    {#each [['overview','Overview'],['data','Data'],['history','History'],['curriculum','Curriculum'],['federated','Federated'],['settings','Settings']] as [id, label]}
      <button
        class="px-3 py-2 rounded-t transition {activeTab === id ? 'text-white border-b-2 border-violet-500 bg-gray-900' : 'text-gray-400 hover:text-gray-200'}"
        on:click={() => setTab(id)}>{label}</button>
    {/each}
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-4 space-y-4">

    <!-- ── Overview tab ───────────────────────────────────────────────────── -->
    {#if activeTab === 'overview'}

      <!-- CIQ hero card -->
      {#if currentCiq}
        <div class="bg-gray-900 rounded-xl p-4 border border-gray-800">
          <div class="flex items-end gap-4">
            <div>
              <div class="text-xs text-gray-400 mb-1">Composite Intelligence Quotient</div>
              <div class="text-5xl font-bold" style="color:{ciqColor(currentCiq.overall)}">{pct(currentCiq.overall)}</div>
            </div>
            <div class="grid grid-cols-5 gap-3 flex-1">
              {#each [['Intelligence',currentCiq.intelligence],['Effectiveness',currentCiq.effectiveness],['Efficiency',currentCiq.efficiency],['Robustness',currentCiq.robustness],['Capability',currentCiq.capability]] as [label, val]}
                <div class="bg-gray-800 rounded-lg p-2 text-center">
                  <div class="text-xs text-gray-400">{label}</div>
                  <div class="text-lg font-semibold mt-1" style="color:{ciqColor(val ?? 0)}">{pct(val)}</div>
                </div>
              {/each}
            </div>
          </div>
        </div>
      {/if}

      <!-- Alerts -->
      {#if alerts.length > 0}
        <div class="space-y-2">
          {#each alerts as alert}
            <div class="bg-red-900/40 border border-red-700 rounded-lg px-4 py-2 flex items-center justify-between">
              <div>
                <span class="text-red-300 font-medium">{alert.dimension.replace(/_/g,' ')}</span>
                <span class="text-gray-400 text-sm ml-2">is at {pct(alert.current_value)} (threshold {pct(alert.alert_threshold)})</span>
              </div>
              <span class="text-xs text-red-400">gap: {pct(alert.gap)}</span>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Dimension radar table -->
      <div class="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
        <div class="px-4 py-3 border-b border-gray-800 text-xs font-semibold text-gray-400 uppercase tracking-wider">
          Capability Dimensions
        </div>
        <div class="divide-y divide-gray-800">
          {#if dimensions.length === 0}
            <div class="px-4 py-8 text-center text-gray-500 text-sm">No evaluation data yet — run a training cycle to populate metrics.</div>
          {/if}
          {#each dimensions as dim}
            {@const v = dim.current ?? 0}
            {@const target = dim.target ?? 1}
            {@const fill = Math.min(v / target, 1) * 100}
            <div class="px-4 py-3 flex items-center gap-4">
              <div class="w-36 shrink-0">
                <div class="text-sm text-gray-200">{dim.display}</div>
                {#if dim.is_alerting}
                  <span class="text-xs text-red-400">● ALERT</span>
                {/if}
              </div>
              <div class="flex-1 min-w-0">
                <div class="h-2 bg-gray-800 rounded-full">
                  <div class="h-2 rounded-full transition-all duration-500"
                    style="width:{fill}%; background:{ciqColor(v/target)}"></div>
                </div>
              </div>
              <div class="w-20 text-right">
                <span class="text-sm font-mono text-gray-200">{pct(dim.current)}</span>
              </div>
              <div class="w-16 text-right text-xs text-gray-500">/{pct(dim.target)}</div>
              <div class="w-8 text-right text-sm {trendColor(dim.trend)}">{trend(dim.trend)}</div>
              <div class="w-16 text-right text-xs text-gray-500">{dim.samples ?? 0} samples</div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Buffer stats -->
      <div class="grid grid-cols-3 gap-3">
        <div class="bg-gray-900 rounded-xl p-4 border border-gray-800">
          <div class="text-xs text-gray-400 mb-1">Buffer Total</div>
          <div class="text-3xl font-bold text-white">{bufferTotal.toLocaleString()}</div>
          <div class="text-xs text-gray-500 mt-1">Quality threshold: {pct(qualityThreshold)}</div>
        </div>
        <div class="bg-gray-900 rounded-xl p-4 border border-gray-800">
          <div class="text-xs text-gray-400 mb-1">Accepted / Rejected</div>
          <div class="text-xl font-bold text-green-400">{(stats?.collector?.events_accepted ?? 0).toLocaleString()}</div>
          <div class="text-sm text-red-400">{(stats?.collector?.events_rejected ?? 0).toLocaleString()} rejected</div>
        </div>
        <div class="bg-gray-900 rounded-xl p-4 border border-gray-800">
          <div class="text-xs text-gray-400 mb-2">Domain Balance</div>
          <div class="space-y-1">
            {#each Object.entries(bufferByDomain).slice(0,5) as [dom, cnt]}
                <div class="flex justify-between text-xs">
                  <span class="text-gray-300">{dom}</span>
                  <span class="text-gray-400">{cnt}</span>
                </div>
            {/each}
          </div>
        </div>
      </div>

      <!-- Reasoning Performance card -->
      <div class="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
        <div class="flex items-center justify-between px-4 py-3 border-b border-gray-800">
          <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">Reasoning Performance</span>
          <button
            class="px-2.5 py-1 text-xs bg-violet-700 hover:bg-violet-600 text-white rounded transition disabled:opacity-50"
            on:click={triggerReasoningTraining}
            disabled={reasoningTraining}>
            {reasoningTraining ? 'Queuing…' : '⚡ Train Reasoning'}
          </button>
        </div>
        {#if reasoningReport}
          <div class="p-4 space-y-4">
            <!-- ECE + overall stats -->
            <div class="grid grid-cols-3 gap-3">
              <div class="bg-gray-800 rounded-lg p-3 text-center">
                <div class="text-xs text-gray-400 mb-1">Calibration Error (ECE)</div>
                <div class="text-2xl font-bold {(reasoningReport.ece ?? 1) < 0.1 ? 'text-green-400' : (reasoningReport.ece ?? 1) < 0.2 ? 'text-yellow-400' : 'text-red-400'}">
                  {((reasoningReport.ece ?? 0) * 100).toFixed(1)}%
                </div>
                <div class="text-xs text-gray-500 mt-1">lower is better</div>
              </div>
              <div class="bg-gray-800 rounded-lg p-3 text-center">
                <div class="text-xs text-gray-400 mb-1">Total Evaluations</div>
                <div class="text-2xl font-bold text-white">{reasoningReport.total_evaluations ?? 0}</div>
              </div>
              <div class="bg-gray-800 rounded-lg p-3 text-center">
                <div class="text-xs text-gray-400 mb-1">Overall Accuracy</div>
                <div class="text-2xl font-bold text-white">{pct(reasoningReport.overall_accuracy)}</div>
              </div>
            </div>
            <!-- Per-strategy table -->
            {#if strategyRows.length > 0}
              <table class="w-full text-sm">
                <thead class="text-xs text-gray-400 uppercase bg-gray-800">
                  <tr>
                    <th class="px-3 py-2 text-left">Strategy</th>
                    <th class="px-3 py-2 text-right">Attempts</th>
                    <th class="px-3 py-2 text-right">Accuracy</th>
                    <th class="px-3 py-2 text-right">Avg Confidence</th>
                    <th class="px-3 py-2 text-right">Overconfident?</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-800">
                  {#each strategyRows as [strat, s]}
                    <tr class="hover:bg-gray-800/50">
                      <td class="px-3 py-2 text-gray-200 capitalize">{strat.toLowerCase().replace(/_/g,' ')}</td>
                      <td class="px-3 py-2 text-right text-gray-300">{s.total_attempts ?? 0}</td>
                      <td class="px-3 py-2 text-right font-mono {(s.rolling_accuracy ?? 0) >= 0.7 ? 'text-green-400' : 'text-yellow-400'}">{pct(s.rolling_accuracy)}</td>
                      <td class="px-3 py-2 text-right font-mono text-gray-300">{pct(s.avg_confidence)}</td>
                      <td class="px-3 py-2 text-right {s.is_overconfident ? 'text-red-400' : 'text-gray-500'}">{s.is_overconfident ? '⚠ Yes' : 'No'}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <p class="text-sm text-gray-500 text-center py-2">No strategy data yet — reasoning self-play runs every 15 training cycles.</p>
            {/if}
            <!-- Recommended strategy -->
            {#if reasoningReport.recommended_strategy}
              <div class="text-xs text-gray-400">
                Recommended strategy: <span class="text-violet-300 font-medium">{reasoningReport.recommended_strategy}</span>
              </div>
            {/if}
          </div>
        {:else}
          <div class="px-4 py-8 text-center text-gray-500 text-sm">Reasoning metrics not available yet.</div>
        {/if}
      </div>

    <!-- ── Data tab ────────────────────────────────────────────────────────── -->
    {:else if activeTab === 'data'}

      <!-- Filters + actions row -->
      <div class="flex items-center gap-3 flex-wrap">
        <select bind:value={exampleDomain}
          on:change={loadExamples}
          class="bg-gray-800 border border-gray-700 rounded px-2 py-1.5 text-sm text-gray-200">
          <option value="">All domains</option>
          {#each ['code','reasoning','safety','tool_use','vision','music','audio','document','memory','planning','swarm','writing','math'] as d}
            <option value={d}>{d}</option>
          {/each}
        </select>
        <span class="text-sm text-gray-400">{exampleTotal.toLocaleString()} examples</span>
        <div class="flex-1"></div>
        <input type="text" bind:value={exportPath} placeholder="/path/to/export.jsonl"
          class="bg-gray-800 border border-gray-700 rounded px-2 py-1.5 text-sm text-gray-200 w-56"/>
        <button on:click={exportData}
          class="px-3 py-1.5 text-sm bg-blue-700 hover:bg-blue-600 text-white rounded transition">Export</button>
        <button on:click={wipeDatabase}
          class="px-3 py-1.5 text-sm bg-red-800 hover:bg-red-700 text-white rounded transition">Wipe All</button>
      </div>

      <!-- Bulk delete -->
      <div class="flex items-center gap-3 bg-gray-900 rounded-xl border border-gray-800 p-3 text-sm">
        <span class="text-gray-400">Bulk delete from</span>
        <input type="date" bind:value={bulkDeleteFrom} class="bg-gray-800 border border-gray-700 rounded px-2 py-1 text-gray-200"/>
        <span class="text-gray-400">to</span>
        <input type="date" bind:value={bulkDeleteTo} class="bg-gray-800 border border-gray-700 rounded px-2 py-1 text-gray-200"/>
        <button on:click={bulkDelete} class="px-3 py-1 bg-orange-700 hover:bg-orange-600 text-white rounded transition">Delete range</button>
      </div>

      <!-- Examples table -->
      <div class="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
        <table class="w-full text-sm">
          <thead class="bg-gray-800 text-gray-400 text-xs uppercase">
            <tr>
              <th class="px-4 py-2 text-left">Domain</th>
              <th class="px-4 py-2 text-left">Source</th>
              <th class="px-4 py-2 text-left">Strategy</th>
              <th class="px-4 py-2 text-left w-1/3">Input (preview)</th>
              <th class="px-4 py-2 text-right">Quality</th>
              <th class="px-4 py-2 text-right">Priority</th>
              <th class="px-4 py-2 text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#if examples.length === 0}
              <tr><td colspan="7" class="px-4 py-8 text-center text-gray-500">No examples match the current filter.</td></tr>
            {/if}
            {#each examples as ex}
              {@const inputPreview = ex.input?.text?.slice(0,60) ?? ex.input?.code?.slice(0,60) ?? '—'}
              <tr class="hover:bg-gray-800/50 transition">
                <td class="px-4 py-2 text-blue-400">{ex.dimensions?.[0] ?? '—'}</td>
                <td class="px-4 py-2 text-gray-300">{sourceLabel(ex.source)}</td>
                <td class="px-4 py-2 text-purple-400">{ex.suitable_strategies?.[0]?.toUpperCase() ?? '—'}</td>
                <td class="px-4 py-2 text-gray-400 truncate max-w-xs">{inputPreview}…</td>
                <td class="px-4 py-2 text-right font-mono" style="color:{ciqColor(ex.quality_score)}">{pct(ex.quality_score)}</td>
                <td class="px-4 py-2 text-right font-mono text-gray-300">{(ex.priority ?? 0).toFixed(2)}</td>
                <td class="px-4 py-2 text-right">
                  <button on:click={() => boostExample(ex.id)} title="Boost"
                    class="text-yellow-500 hover:text-yellow-300 mr-2 text-xs">↑</button>
                  <button on:click={() => deleteExample(ex.id)} title="Delete"
                    class="text-red-500 hover:text-red-300 text-xs">✕</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        <!-- Pagination -->
        <div class="px-4 py-3 border-t border-gray-800 flex items-center justify-between text-sm text-gray-400">
          <button disabled={exampleOffset === 0}
            on:click={() => { exampleOffset = Math.max(0, exampleOffset - exampleLimit); loadExamples(); }}
            class="px-3 py-1 bg-gray-800 rounded hover:bg-gray-700 disabled:opacity-40">← Prev</button>
          <span>{exampleOffset + 1}–{Math.min(exampleOffset + exampleLimit, exampleTotal)} of {exampleTotal}</span>
          <button disabled={exampleOffset + exampleLimit >= exampleTotal}
            on:click={() => { exampleOffset += exampleLimit; loadExamples(); }}
            class="px-3 py-1 bg-gray-800 rounded hover:bg-gray-700 disabled:opacity-40">Next →</button>
        </div>
      </div>

    <!-- ── History tab ─────────────────────────────────────────────────────── -->
    {:else if activeTab === 'history'}

      <!-- CIQ sparkline (simple text representation) -->
      {#if ciqHistory.length > 0}
        <div class="bg-gray-900 rounded-xl border border-gray-800 p-4">
          <div class="text-xs text-gray-400 mb-3 uppercase font-semibold">CIQ Over Time</div>
          <div class="flex items-end gap-1 h-16">
            {#each ciqHistory.slice(-60) as entry}
              {@const h = Math.round((entry.overall ?? 0) * 100)}
              <div title="{pct(entry.overall)}"
                class="flex-1 rounded-sm min-w-[3px] transition-all"
                style="height:{h}%; background:{ciqColor(entry.overall ?? 0)}"></div>
            {/each}
          </div>
          <div class="flex justify-between text-xs text-gray-500 mt-1">
            <span>{ciqHistory.length} data points</span>
            <span>Latest: {pct(ciqHistory[ciqHistory.length - 1]?.overall)}</span>
          </div>
        </div>
      {/if}

      <!-- Loop cycle log -->
      <div class="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
        <div class="px-4 py-3 border-b border-gray-800 flex items-center justify-between">
          <span class="text-xs font-semibold text-gray-400 uppercase">Training Cycle Log</span>
          <button on:click={rollback} class="text-xs px-2 py-1 bg-orange-800 hover:bg-orange-700 text-white rounded transition">
            Rollback Adapter
          </button>
        </div>
        <div class="divide-y divide-gray-800 max-h-96 overflow-y-auto">
          {#if loopHistory.length === 0}
            <div class="px-4 py-8 text-center text-gray-500 text-sm">No training history yet.</div>
          {/if}
          {#each [...loopHistory].reverse() as cycle}
            <div class="px-4 py-3 flex items-center gap-4 text-sm">
              <span class="text-gray-500 font-mono text-xs w-16">#{cycle.cycle}</span>
              <div class="flex-1">
                <span class="text-green-400">+{cycle.self_play_added} examples</span>
                {#if cycle.alerts_handled > 0}
                  <span class="text-yellow-400 ml-2">⚠ {cycle.alerts_handled} alerts</span>
                {/if}
                {#if cycle.promotion}
                  <span class="text-violet-400 ml-2">● Promoted: {cycle.promotion}</span>
                {/if}
              </div>
              <span class="text-gray-500 text-xs">{cycle.elapsed_ms}ms</span>
              <span class="text-gray-600 text-xs">{new Date(cycle.timestamp / 1000).toLocaleTimeString()}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Self-play stats -->
      {#if selfPlayState}
        <div class="grid grid-cols-4 gap-3">
          {#each [['Rounds',selfPlayState.rounds_completed],['Examples Generated',selfPlayState.examples_generated],['Violations Fixed',selfPlayState.constitutional_violations_fixed],['Adversarial Failures',selfPlayState.adversarial_failures]] as [label, val]}
            <div class="bg-gray-900 rounded-xl p-3 border border-gray-800 text-center">
              <div class="text-2xl font-bold text-white">{(val ?? 0).toLocaleString()}</div>
              <div class="text-xs text-gray-400 mt-1">{label}</div>
            </div>
          {/each}
        </div>
      {/if}

    <!-- ── Curriculum tab ──────────────────────────────────────────────────── -->
    {:else if activeTab === 'curriculum'}

      {#if curriculumStatus}
        <!-- Stage banner -->
        <div class="bg-violet-900/40 border border-violet-700 rounded-xl p-4">
          <div class="text-xs text-violet-300 mb-1">Current Stage</div>
          <div class="text-2xl font-bold text-white">Stage {curriculumStatus.current_stage}: {curriculumStatus.stage_name}</div>
          <div class="mt-3 bg-gray-800 rounded-full h-2">
            <div class="h-2 rounded-full bg-violet-500 transition-all"
              style="width:{(curriculumStatus.progress_pct ?? 0).toFixed(1)}%"></div>
          </div>
          <div class="text-xs text-gray-400 mt-1">{(curriculumStatus.progress_pct ?? 0).toFixed(1)}% to next stage</div>
        </div>

        <!-- Gates -->
        <div class="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
          <div class="px-4 py-3 border-b border-gray-800 text-xs font-semibold text-gray-400 uppercase">
            Stage Advancement Gates
          </div>
          <div class="divide-y divide-gray-800">
            {#each curriculumStatus.gates as gate}
              <div class="px-4 py-3 flex items-center gap-4">
                <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold
                  {gate.passed ? 'bg-green-700 text-green-100' : 'bg-gray-700 text-gray-400'}">
                  {gate.passed ? '✓' : '○'}
                </div>
                <div class="flex-1">
                  <span class="text-sm text-gray-200">{gate.dimension.replace(/_/g,' ')}</span>
                  <span class="text-xs text-gray-500 ml-2">— {gate.metric.replace(/_/g,' ')}</span>
                </div>
                <div class="text-sm font-mono">
                  <span class="{gate.passed ? 'text-green-400' : 'text-gray-300'}">{pct(gate.current)}</span>
                  <span class="text-gray-500"> / {pct(gate.threshold)}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Stage map -->
        <div class="bg-gray-900 rounded-xl border border-gray-800 p-4">
          <div class="text-xs text-gray-400 uppercase font-semibold mb-3">All Stages</div>
          <div class="flex gap-2 flex-wrap">
            {#each [1,2,3,4,5,6,7] as stg}
              <div class="px-3 py-2 rounded-lg text-xs text-center min-w-[80px]
                {stg < curriculumStatus.current_stage ? 'bg-green-900/50 border border-green-700 text-green-300' :
                 stg === curriculumStatus.current_stage ? 'bg-violet-900/60 border border-violet-500 text-violet-200 font-bold' :
                 'bg-gray-800 border border-gray-700 text-gray-500'}">
                <div>{stg}</div>
                <div class="text-[10px] mt-0.5">{stageNames[stg]}</div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="text-center text-gray-500 py-12">Loading curriculum status…</div>
      {/if}

    <!-- ── Federated tab ───────────────────────────────────────────────────── -->
    {:else if activeTab === 'federated'}

      <div class="bg-gray-900 rounded-xl border border-gray-800 p-5 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-white font-semibold">Federated Improvement</div>
            <div class="text-gray-400 text-sm mt-1">Share anonymised LoRA weight deltas with opted-in peers. Raw data never leaves your device.</div>
          </div>
          {#if preferences}
            <label class="relative inline-flex items-center cursor-pointer">
              <input type="checkbox" class="sr-only peer" bind:checked={preferences.federated_opt_in}/>
              <div class="w-10 h-5 rounded-full bg-gray-700 peer-checked:bg-violet-600 after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-5"></div>
            </label>
          {/if}
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="bg-gray-800 rounded-lg p-3">
            <div class="text-xs text-gray-400">Privacy Mechanism</div>
            <div class="text-sm text-white mt-1">Gaussian DP (ε=8, δ=1e-5)</div>
          </div>
          <div class="bg-gray-800 rounded-lg p-3">
            <div class="text-xs text-gray-400">What is shared</div>
            <div class="text-sm text-white mt-1">LoRA weight deltas only</div>
          </div>
          <div class="bg-gray-800 rounded-lg p-3">
            <div class="text-xs text-gray-400">Peer Trust Model</div>
            <div class="text-sm text-white mt-1">Reputation-weighted FedAvg</div>
          </div>
          <div class="bg-gray-800 rounded-lg p-3">
            <div class="text-xs text-gray-400">Poisoning Defence</div>
            <div class="text-sm text-white mt-1">Adversarial probe pre-check</div>
          </div>
        </div>

        <div class="bg-blue-900/30 border border-blue-700 rounded-lg px-4 py-3 text-sm text-blue-300">
          Federated participation is completely optional and disabled by default. You can opt out at any time.
        </div>
      </div>

    <!-- ── Settings tab ───────────────────────────────────────────────────── -->
    {:else if activeTab === 'settings'}

      {#if preferences}
        <div class="bg-gray-900 rounded-xl border border-gray-800 p-5 space-y-5">
          <div class="text-white font-semibold border-b border-gray-800 pb-3">Training Preferences</div>

          <div class="flex items-center justify-between">
            <div>
              <div class="text-sm text-gray-200">Continuous Improvement</div>
              <div class="text-xs text-gray-400">Enable the eternal training loop</div>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input type="checkbox" class="sr-only peer" bind:checked={preferences.enabled}/>
              <div class="w-10 h-5 rounded-full bg-gray-700 peer-checked:bg-green-600 after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-5"></div>
            </label>
          </div>

          <div class="flex items-center justify-between">
            <div>
              <div class="text-sm text-gray-200">Train on Battery</div>
              <div class="text-xs text-gray-400">Allow training when not plugged in</div>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input type="checkbox" class="sr-only peer" bind:checked={preferences.train_on_battery}/>
              <div class="w-10 h-5 rounded-full bg-gray-700 peer-checked:bg-yellow-600 after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-5"></div>
            </label>
          </div>

          <div>
            <label class="text-sm text-gray-200">Minimum battery % before training stops</label>
            <input type="range" min="5" max="50" step="5" bind:value={preferences.min_battery_pct}
              class="w-full mt-2 accent-violet-500"/>
            <div class="text-xs text-gray-400 mt-1">{preferences.min_battery_pct}%</div>
          </div>

          <div>
            <label class="text-sm text-gray-200">GPU VRAM reserved for inference (MB)</label>
            <input type="range" min="1024" max="16384" step="512" bind:value={preferences.gpu_vram_reserve_mb}
              class="w-full mt-2 accent-violet-500"/>
            <div class="text-xs text-gray-400 mt-1">{(preferences.gpu_vram_reserve_mb / 1024).toFixed(1)} GB</div>
          </div>

          <div>
            <label class="text-sm text-gray-200">GPU idle seconds before training starts</label>
            <input type="range" min="60" max="1800" step="60" bind:value={preferences.idle_seconds_needed}
              class="w-full mt-2 accent-violet-500"/>
            <div class="text-xs text-gray-400 mt-1">{preferences.idle_seconds_needed}s</div>
          </div>

          <button on:click={savePreferences}
            class="w-full py-2 bg-violet-700 hover:bg-violet-600 text-white rounded-lg transition font-medium">
            Save Preferences
          </button>
        </div>
      {/if}

    {/if}
  </div>
</div>

<!-- Toast notification -->
{#if toast}
  <div class="fixed bottom-6 left-1/2 -translate-x-1/2 bg-gray-800 text-white text-sm px-4 py-2.5 rounded-xl shadow-lg border border-gray-700 z-50 pointer-events-none">
    {toast}
  </div>
{/if}
