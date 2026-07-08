<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { addToast } from '$lib/stores/toast';

  interface BugRecord {
    id: number;
    fingerprint: string;
    source: 'compile' | 'test' | 'lint' | 'runtime';
    severity: 'error' | 'warning';
    title: string;
    message: string;
    file_path: string | null;
    line_number: number | null;
    status: 'open' | 'fix_proposed' | 'fixed' | 'ignored' | 'wontfix';
    occurrence_count: number;
    fix_attempts: number;
    self_upgrade_proposal_id: string | null;
    first_seen_ms: number;
    last_seen_ms: number;
  }

  let survivalScanEnabled = false;
  let selfUpgradeEnabled = false;
  let flagsLoading = true;
  let bugs: BugRecord[] = [];
  let scanning = false;
  let expandedBugs = new Set<number>();
  let unlistenFns: Array<() => void> = [];

  const sourceLabel: Record<BugRecord['source'], string> = {
    compile: 'Compile', test: 'Test', lint: 'Lint', runtime: 'Runtime',
  };
  const statusLabel: Record<BugRecord['status'], string> = {
    open: 'Open',
    fix_proposed: 'Fix proposed',
    fixed: 'Fixed',
    ignored: 'Ignored',
    wontfix: "Won't fix",
  };

  async function loadFlags() {
    flagsLoading = true;
    try {
      const flags = await invoke<Record<string, unknown>>('get_feature_flags');
      survivalScanEnabled = Boolean(flags['survival_scan_enabled']);
      selfUpgradeEnabled = Boolean(flags['self_upgrade_enabled']);
    } catch (e) {
      addToast(`Failed to load feature flags: ${e}`, 'error');
    } finally {
      flagsLoading = false;
    }
  }

  async function toggleSurvivalScan() {
    try {
      const flags = await invoke<Record<string, unknown>>('get_feature_flags');
      flags['survival_scan_enabled'] = survivalScanEnabled;
      await invoke('set_feature_flags', { flags });
      addToast(survivalScanEnabled ? 'Background bug scanning enabled.' : 'Background bug scanning disabled.', 'info');
    } catch (e) {
      addToast(`Failed to save: ${e}`, 'error');
      survivalScanEnabled = !survivalScanEnabled;
    }
  }

  async function refreshBugs() {
    try {
      bugs = await invoke<BugRecord[]>('list_bugs');
    } catch (e) {
      addToast(`Failed to load bugs: ${e}`, 'error');
    }
  }

  async function scanNow() {
    scanning = true;
    try {
      const touched = await invoke<number>('scan_now');
      addToast(`Scan complete — ${touched} bug${touched === 1 ? '' : 's'} touched.`, 'info');
      await refreshBugs();
    } catch (e) {
      addToast(`Scan failed: ${e}`, 'error');
    } finally {
      scanning = false;
    }
  }

  async function resolveBug(bugId: number, action: 'ignore' | 'wontfix' | 'reopen') {
    try {
      await invoke('resolve_bug', { bugId, action });
    } catch (e) {
      addToast(`${e}`, 'error');
    } finally {
      await refreshBugs();
    }
  }

  async function attemptFix(bugId: number) {
    try {
      const result = await invoke<string>('attempt_fix_now', { bugId });
      addToast(result, 'info', 8000);
    } catch (e) {
      addToast(`${e}`, 'error', 10000);
    } finally {
      await refreshBugs();
    }
  }

  function toggleExpand(id: number) {
    if (expandedBugs.has(id)) expandedBugs.delete(id);
    else expandedBugs.add(id);
    expandedBugs = expandedBugs;
  }

  onMount(async () => {
    await loadFlags();
    await refreshBugs();
    const u1 = await listen('bug-discovered', () => void refreshBugs());
    const u2 = await listen('bug-updated', () => void refreshBugs());
    unlistenFns = [u1, u2];
  });
  onDestroy(() => unlistenFns.forEach((fn) => fn()));
</script>

<div class="bug-database">
  <header>
    <h2>🐞 Bug Database</h2>
    <p class="lead">
      Watches for compile errors, test failures, lints, and runtime crashes across the Workspace IDE,
      catalogs every one (deduplicated by fingerprint, with an occurrence count), and — if Self-Build is
      also enabled — routes fixable bugs through the same sandboxed, risk-gated self-upgrade pipeline.
    </p>
  </header>

  <section class="toggle-row">
    {#if flagsLoading}
      <span class="muted">Loading settings…</span>
    {:else}
      <label class="toggle">
        <input type="checkbox" bind:checked={survivalScanEnabled} on:change={toggleSurvivalScan} />
        <span>Background scanning enabled</span>
      </label>
      <span class="muted">
        {selfUpgradeEnabled
          ? 'Self-Build is on — open bugs may be auto-submitted for fixing.'
          : 'Self-Build is off — bugs will only be cataloged, not auto-fixed. Enable it in the Self-Build panel.'}
      </span>
    {/if}
  </section>

  <section class="propose-row">
    <button on:click={scanNow} disabled={scanning}>{scanning ? 'Scanning…' : 'Scan Now'}</button>
  </section>

  <section class="queue">
    <h3>Bugs ({bugs.length})</h3>
    {#if bugs.length === 0}
      <p class="muted">No bugs found yet.</p>
    {/if}
    {#each bugs as b (b.id)}
      <div class="bug" class:resolved={b.status === 'fixed' || b.status === 'ignored' || b.status === 'wontfix'}>
        <div class="bug-header">
          <span class="source-badge source-{b.source}">{sourceLabel[b.source]}</span>
          <span class="severity-badge severity-{b.severity}">{b.severity}</span>
          <span class="title">{b.title}</span>
          <span class="status-badge status-{b.status}">{statusLabel[b.status]}</span>
        </div>
        <div class="meta-row">
          {#if b.file_path}<span>{b.file_path}{b.line_number ? `:${b.line_number}` : ''}</span>{/if}
          <span>Seen {b.occurrence_count}×</span>
          {#if b.fix_attempts > 0}<span>{b.fix_attempts} fix attempt{b.fix_attempts === 1 ? '' : 's'}</span>{/if}
        </div>

        <button class="expand-toggle" on:click={() => toggleExpand(b.id)}>
          {expandedBugs.has(b.id) ? '▾ Hide message' : '▸ Show message'}
        </button>
        {#if expandedBugs.has(b.id)}
          <pre class="message">{b.message}</pre>
        {/if}

        <div class="actions">
          {#if b.status === 'open'}
            <button class="fix" on:click={() => attemptFix(b.id)} disabled={!selfUpgradeEnabled}
              title={selfUpgradeEnabled ? '' : 'Enable Self-Build to attempt automatic fixes'}>
              Attempt Fix
            </button>
            <button class="ignore" on:click={() => resolveBug(b.id, 'ignore')}>Ignore</button>
            <button class="wontfix" on:click={() => resolveBug(b.id, 'wontfix')}>Won't Fix</button>
          {:else if b.status === 'ignored' || b.status === 'wontfix'}
            <button class="reopen" on:click={() => resolveBug(b.id, 'reopen')}>Reopen</button>
          {/if}
        </div>
      </div>
    {/each}
  </section>
</div>

<style>
  .bug-database { padding: 16px; overflow-y: auto; height: 100%; color: var(--text, #e4e4e7); font-size: 13px; }
  header h2 { margin: 0 0 6px; font-size: 1.1rem; }
  .lead { color: var(--text-dim, #a1a1aa); font-size: 0.82rem; line-height: 1.5; max-width: 640px; }
  .muted { color: var(--text-dim, #71717a); font-size: 0.8rem; }

  .toggle-row { display: flex; flex-direction: column; align-items: flex-start; gap: 6px; margin: 14px 0; }
  .toggle { display: flex; align-items: center; gap: 6px; cursor: pointer; }

  .propose-row { margin-bottom: 18px; }
  .propose-row button, .actions button {
    border: none; border-radius: 8px; padding: 8px 14px; cursor: pointer; font-size: 0.82rem; font-weight: 600;
  }
  .propose-row button { background: var(--accent, #16a34a); color: #fff; }
  .propose-row button:disabled { opacity: 0.5; cursor: default; }

  .queue h3 { font-size: 0.9rem; margin-bottom: 8px; }
  .bug {
    border: 1px solid var(--border, #3f3f46); border-radius: 10px; padding: 12px; margin-bottom: 12px;
    background: var(--bg2, #1c1c1f);
  }
  .bug.resolved { opacity: 0.6; }
  .bug-header { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 6px; }
  .title { font-weight: 600; flex: 1; min-width: 180px; }
  .source-badge, .severity-badge, .status-badge { font-size: 0.72rem; padding: 2px 8px; border-radius: 999px; white-space: nowrap; }
  .source-badge { background: rgba(96,165,250,0.18); color: #60a5fa; }
  .severity-error { background: rgba(248,113,113,0.18); color: #f87171; }
  .severity-warning { background: rgba(251,191,36,0.18); color: #fbbf24; }
  .status-badge { background: var(--bg, #18181b); color: var(--text-dim, #a1a1aa); }

  .meta-row { display: flex; flex-wrap: wrap; gap: 12px; font-size: 0.78rem; color: var(--text-dim, #a1a1aa); margin-bottom: 6px; }

  .expand-toggle {
    background: none; border: none; color: var(--text); cursor: pointer; font-size: 0.8rem; padding: 4px 0;
  }
  .message {
    background: var(--bg, #18181b); border-radius: 6px; padding: 8px; margin: 4px 0 8px;
    font-size: 0.78rem; white-space: pre-wrap; word-break: break-word;
  }

  .actions { display: flex; gap: 8px; margin-top: 8px; }
  .actions .fix { background: var(--accent, #16a34a); color: #fff; }
  .actions .fix:disabled { opacity: 0.5; cursor: default; }
  .actions .ignore, .actions .wontfix, .actions .reopen {
    background: transparent; border: 1px solid var(--border, #3f3f46) !important; color: var(--text);
  }
</style>
