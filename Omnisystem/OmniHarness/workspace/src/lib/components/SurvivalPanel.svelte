<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { addToast } from '$lib/stores/toast';
  import BugDatabasePanel from '$lib/components/BugDatabasePanel.svelte';

  type Tab = 'overview' | 'bugs' | 'kb' | 'sns';
  let activeTab: Tab = 'overview';

  // ── Overview ────────────────────────────────────────────────────────────────
  interface SurvivalTargetInfo { name: string; rel_path: string; kind: string }
  let targets: SurvivalTargetInfo[] = [];
  let survivalEnabled = false;
  let selfUpgradeEnabled = false;
  let flagsLoading = true;

  async function loadFlags() {
    flagsLoading = true;
    try {
      const flags = await invoke<Record<string, unknown>>('get_feature_flags');
      survivalEnabled = Boolean(flags['survival_enabled']);
      selfUpgradeEnabled = Boolean(flags['self_upgrade_enabled']);
    } catch (e) {
      addToast(`Failed to load feature flags: ${e}`, 'error');
    } finally {
      flagsLoading = false;
    }
  }

  async function toggleSurvival() {
    try {
      const flags = await invoke<Record<string, unknown>>('get_feature_flags');
      flags['survival_enabled'] = survivalEnabled;
      await invoke('set_feature_flags', { flags });
      addToast(survivalEnabled ? 'Survival System enabled.' : 'Survival System disabled.', 'info');
    } catch (e) {
      addToast(`Failed to save: ${e}`, 'error');
      survivalEnabled = !survivalEnabled;
    }
  }

  async function loadTargets() {
    try {
      targets = await invoke<SurvivalTargetInfo[]>('list_survival_targets');
    } catch (e) {
      addToast(`Failed to load monitored targets: ${e}`, 'error');
    }
  }

  // ── Knowledge Base ──────────────────────────────────────────────────────────
  interface FixEntry {
    id: number;
    error_pattern: string;
    solution_type: string;
    solution_script: string;
    confidence: number;
    usage_count: number;
    success_count: number;
    created_by: string;
    verified: boolean;
  }
  let fixes: FixEntry[] = [];
  let newPattern = '';
  let newSolution = '';
  let kbBusy = false;

  async function loadFixes() {
    try {
      fixes = await invoke<FixEntry[]>('list_fixes');
    } catch (e) {
      addToast(`Failed to load knowledge base: ${e}`, 'error');
    }
  }

  async function submitFix() {
    if (!newPattern.trim() || !newSolution.trim()) return;
    kbBusy = true;
    try {
      await invoke('report_fix', { errorPattern: newPattern.trim(), solution: newSolution.trim(), createdBy: 'user' });
      newPattern = '';
      newSolution = '';
      addToast('Fix saved to the knowledge base.', 'success');
      await loadFixes();
    } catch (e) {
      addToast(`${e}`, 'error');
    } finally {
      kbBusy = false;
    }
  }

  // ── Sandbox Nervous System ──────────────────────────────────────────────────
  interface SandboxInfo {
    sandbox_id: string; component: string; tier: string; pid: number | null;
    status: string; cpu_pct: number; mem_bytes: number; crashes: number; violations: number;
  }
  interface CapabilityViolation {
    sandbox_id: string; component: string; violation_type: string;
    attempted_action: string; blocked: boolean; timestamp_ns: number;
  }
  interface FailureReport {
    id: string; campaign_id: string; target: string; strategy: string;
    error_pattern: string; reproduction_cmd: string; timestamp_ns: number;
  }
  interface OrchestratorStats {
    active_campaigns: number; total_iterations: number; total_crashes: number; rules_added_to_kb: number;
  }
  let sandboxes: SandboxInfo[] = [];
  let violations: CapabilityViolation[] = [];
  let failures: FailureReport[] = [];
  let stats: OrchestratorStats | null = null;

  async function loadSns() {
    try {
      [sandboxes, violations, failures, stats] = await Promise.all([
        invoke<SandboxInfo[]>('sns_list_sandboxes'),
        invoke<CapabilityViolation[]>('sns_list_violations'),
        invoke<FailureReport[]>('fff_list_failures'),
        invoke<OrchestratorStats>('fff_stats'),
      ]);
    } catch (e) {
      addToast(`Failed to load Sandbox Nervous System data: ${e}`, 'error');
    }
  }

  // ── Lifecycle ────────────────────────────────────────────────────────────────
  let unlistenFns: Array<() => void> = [];

  async function refreshActiveTab() {
    if (activeTab === 'overview') await loadTargets();
    else if (activeTab === 'kb') await loadFixes();
    else if (activeTab === 'sns') await loadSns();
  }

  function selectTab(tab: Tab) {
    activeTab = tab;
    void refreshActiveTab();
  }

  onMount(async () => {
    await loadFlags();
    await loadTargets();
    const u = await listen('bug-updated', () => void refreshActiveTab());
    unlistenFns = [u];
  });
  onDestroy(() => unlistenFns.forEach((fn) => fn()));
</script>

<div class="survival-panel">
  <header>
    <h2>🩺 Survival System</h2>
    <p class="lead">
      The one system that finds and fixes bugs across Omnisystem — Bug Hunter scans for compile/test/lint
      issues, the Sandbox Nervous System contributes fuzzing and capability-violation findings, everything
      is cataloged in the Bug Database, and fixable bugs are routed through the sandboxed self-upgrade
      pipeline. The shell-script Knowledge Base is the fast, synchronous path for known runtime failures.
    </p>
  </header>

  <nav class="tabs">
    <button class:active={activeTab === 'overview'} on:click={() => selectTab('overview')}>Overview</button>
    <button class:active={activeTab === 'bugs'} on:click={() => selectTab('bugs')}>Bug Database</button>
    <button class:active={activeTab === 'kb'} on:click={() => selectTab('kb')}>Knowledge Base</button>
    <button class:active={activeTab === 'sns'} on:click={() => selectTab('sns')}>Sandbox Nervous System</button>
  </nav>

  <div class="tab-body">
    {#if activeTab === 'overview'}
      <section class="toggle-row">
        {#if flagsLoading}
          <span class="muted">Loading settings…</span>
        {:else}
          <label class="toggle">
            <input type="checkbox" bind:checked={survivalEnabled} on:change={toggleSurvival} />
            <span>Survival System enabled (background scanning)</span>
          </label>
          <span class="muted">
            {selfUpgradeEnabled
              ? 'Self-Build is also on — discovered bugs may be auto-submitted for fixing.'
              : 'Self-Build is off — bugs are cataloged only, never auto-fixed. Enable it in the Self-Build panel.'}
          </span>
        {/if}
      </section>

      <h3>Monitored targets ({targets.length})</h3>
      <p class="muted">
        Round-robin scanned one per cycle — not every crate in the repo yet; see the Bug Hunter doc
        comment for why "the entire monorepo" is a work in progress.
      </p>
      <div class="target-list">
        {#each targets as t (t.name)}
          <div class="target-row">
            <span class="target-name">{t.name}</span>
            <span class="target-kind">{t.kind}</span>
            <span class="target-path">{t.rel_path}</span>
          </div>
        {/each}
      </div>
    {:else if activeTab === 'bugs'}
      <BugDatabasePanel />
    {:else if activeTab === 'kb'}
      <section class="propose-row">
        <input placeholder="Error pattern (substring match)" bind:value={newPattern} disabled={kbBusy} />
        <input placeholder="Fix script (shell command)" bind:value={newSolution} disabled={kbBusy} />
        <button on:click={submitFix} disabled={kbBusy || !newPattern.trim() || !newSolution.trim()}>
          {kbBusy ? 'Saving…' : 'Add Fix'}
        </button>
      </section>
      <h3>Knowledge base ({fixes.length})</h3>
      {#if fixes.length === 0}
        <p class="muted">No fixes recorded yet.</p>
      {/if}
      {#each fixes as f (f.id)}
        <div class="kb-entry">
          <div class="kb-header">
            <span class="pattern">{f.error_pattern}</span>
            <span class="badge">{f.solution_type}</span>
            {#if f.verified}<span class="badge verified">verified</span>{/if}
          </div>
          <pre class="script">{f.solution_script}</pre>
          <div class="meta-row">
            <span>confidence: {f.confidence.toFixed(2)}</span>
            <span>used {f.usage_count}× / succeeded {f.success_count}×</span>
            <span>by {f.created_by}</span>
          </div>
        </div>
      {/each}
    {:else if activeTab === 'sns'}
      {#if stats}
        <div class="stats-row">
          <span>Active campaigns: {stats.active_campaigns}</span>
          <span>Total iterations: {stats.total_iterations}</span>
          <span>Total crashes: {stats.total_crashes}</span>
          <span>Rules added to KB: {stats.rules_added_to_kb}</span>
        </div>
      {/if}

      <h3>Sandboxes ({sandboxes.length})</h3>
      {#each sandboxes as s (s.sandbox_id)}
        <div class="sns-row">
          <span class="badge status-{s.status.toLowerCase()}">{s.status}</span>
          <span>{s.component}</span>
          <span class="muted">{s.tier}</span>
          <span class="muted">crashes: {s.crashes} · violations: {s.violations}</span>
        </div>
      {/each}

      <h3>Capability violations ({violations.length})</h3>
      {#each violations as v, i (i)}
        <div class="sns-row">
          <span class="badge" class:danger={v.blocked}>{v.violation_type}</span>
          <span>{v.component}</span>
          <span class="muted">{v.attempted_action}</span>
        </div>
      {/each}

      <h3>Fuzzing failures ({failures.length})</h3>
      {#each failures as f (f.id)}
        <div class="sns-row">
          <span class="badge">{f.target}</span>
          <span>{f.error_pattern}</span>
          <span class="muted">{f.strategy}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .survival-panel { padding: 16px; overflow-y: auto; height: 100%; color: var(--text, #e4e4e7); font-size: 13px; }
  header h2 { margin: 0 0 6px; font-size: 1.1rem; }
  .lead { color: var(--text-dim, #a1a1aa); font-size: 0.82rem; line-height: 1.5; max-width: 680px; margin-bottom: 14px; }
  .muted { color: var(--text-dim, #71717a); font-size: 0.8rem; }
  .danger { color: #f87171 !important; }

  .tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border, #3f3f46); margin-bottom: 14px; }
  .tabs button {
    background: none; border: none; color: var(--text-dim, #a1a1aa); padding: 8px 14px; cursor: pointer;
    font-size: 0.85rem; border-bottom: 2px solid transparent;
  }
  .tabs button.active { color: var(--text); border-bottom-color: var(--accent, #16a34a); font-weight: 600; }

  .toggle-row { display: flex; flex-direction: column; align-items: flex-start; gap: 6px; margin-bottom: 16px; }
  .toggle { display: flex; align-items: center; gap: 6px; cursor: pointer; }

  h3 { font-size: 0.9rem; margin: 12px 0 8px; }

  .target-list, .kb-entry, .sns-row { margin-bottom: 8px; }
  .target-row, .sns-row {
    display: flex; gap: 12px; align-items: center; padding: 6px 10px; border-radius: 8px;
    background: var(--bg2, #1c1c1f); border: 1px solid var(--border, #3f3f46); font-size: 0.8rem;
  }
  .target-name { font-weight: 600; min-width: 160px; }
  .target-kind { color: var(--accent, #16a34a); }
  .target-path { color: var(--text-dim, #a1a1aa); font-family: monospace; }

  .propose-row { display: flex; gap: 8px; margin-bottom: 14px; }
  .propose-row input {
    flex: 1; background: var(--bg, #18181b); border: 1px solid var(--border, #3f3f46);
    border-radius: 8px; color: var(--text); padding: 8px; font-size: 0.82rem;
  }
  .propose-row button {
    border: none; border-radius: 8px; padding: 8px 14px; cursor: pointer; font-size: 0.82rem; font-weight: 600;
    background: var(--accent, #16a34a); color: #fff;
  }
  .propose-row button:disabled { opacity: 0.5; cursor: default; }

  .kb-entry {
    border: 1px solid var(--border, #3f3f46); border-radius: 10px; padding: 10px;
    background: var(--bg2, #1c1c1f);
  }
  .kb-header { display: flex; gap: 8px; align-items: center; margin-bottom: 6px; }
  .pattern { font-weight: 600; font-family: monospace; flex: 1; }
  .badge {
    font-size: 0.72rem; padding: 2px 8px; border-radius: 999px; background: var(--bg, #18181b);
    color: var(--text-dim, #a1a1aa); white-space: nowrap;
  }
  .badge.verified { background: rgba(34,197,94,0.18); color: #4ade80; }
  .script {
    background: var(--bg, #18181b); border-radius: 6px; padding: 6px 8px; margin: 0 0 6px;
    font-size: 0.75rem; white-space: pre-wrap; word-break: break-all;
  }
  .meta-row { display: flex; gap: 12px; font-size: 0.75rem; color: var(--text-dim, #a1a1aa); }

  .stats-row { display: flex; gap: 16px; flex-wrap: wrap; font-size: 0.8rem; margin-bottom: 12px; }
</style>
