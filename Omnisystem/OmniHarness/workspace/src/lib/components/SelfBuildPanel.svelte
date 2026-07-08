<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { parseUnifiedDiff, type DiffHunk } from '$lib/stores/diff';
  import { addToast } from '$lib/stores/toast';

  interface FileChange {
    path: string;
    unified_diff: string;
    new_content: string;
  }
  interface RiskAssessment {
    score: number;
    violations: number;
    touched_safety_critical: string[];
    wide_surface: boolean;
  }
  type AutonomyTier = 'auto_merge' | 'staged_approval' | 'blocked';
  type ProposalStatus =
    | 'pending_review' | 'pending_approval' | 'auto_merged' | 'approved' | 'rejected' | 'failed';
  interface SandboxOutcome {
    build_ok: boolean;
    test_ok: boolean;
    build_output: string;
    test_output: string;
  }
  interface SelfUpgradeProposal {
    id: string;
    goal: string;
    files: FileChange[];
    risk: RiskAssessment;
    tier: AutonomyTier;
    sandbox: SandboxOutcome | null;
    status: ProposalStatus;
    created_at_ms: number;
  }

  let selfUpgradeEnabled = false;
  let flagsLoading = true;
  let proposals: SelfUpgradeProposal[] = [];
  let goalInput = '';
  let submitting = false;
  let expandedFiles = new Set<string>();
  let unlistenFns: Array<() => void> = [];

  function tierLabel(t: AutonomyTier): string {
    return t === 'auto_merge' ? 'Auto-merge eligible' : t === 'staged_approval' ? 'Staged approval' : 'Blocked (safety-critical)';
  }
  function statusLabel(s: ProposalStatus): string {
    return {
      pending_review: 'Awaiting review (never sandboxed yet)',
      pending_approval: 'Sandboxed — awaiting approval',
      auto_merged: 'Auto-merged',
      approved: 'Approved & merged',
      rejected: 'Rejected',
      failed: 'Sandbox failed',
    }[s];
  }

  async function loadFlags() {
    flagsLoading = true;
    try {
      const flags = await invoke<Record<string, unknown>>('get_feature_flags');
      selfUpgradeEnabled = Boolean(flags['self_upgrade_enabled']);
    } catch (e) {
      addToast(`Failed to load feature flags: ${e}`, 'error');
    } finally {
      flagsLoading = false;
    }
  }

  async function toggleSelfUpgrade() {
    try {
      const flags = await invoke<Record<string, unknown>>('get_feature_flags');
      flags['self_upgrade_enabled'] = selfUpgradeEnabled;
      await invoke('set_feature_flags', { flags });
      addToast(selfUpgradeEnabled ? 'Self-Build enabled.' : 'Self-Build disabled.', 'info');
    } catch (e) {
      addToast(`Failed to save: ${e}`, 'error');
      selfUpgradeEnabled = !selfUpgradeEnabled;
    }
  }

  async function refreshProposals() {
    try {
      proposals = await invoke<SelfUpgradeProposal[]>('list_self_upgrade_proposals');
    } catch (e) {
      addToast(`Failed to load proposals: ${e}`, 'error');
    }
  }

  async function submitGoal() {
    if (!goalInput.trim()) return;
    submitting = true;
    try {
      const output = await invoke<{ content: string }>('send_agent_message', {
        agentId: 'self-upgrader',
        message: { content: goalInput.trim(), role: null, metadata: null },
      });
      addToast(output.content, 'info', 8000);
      goalInput = '';
      await refreshProposals();
    } catch (e) {
      addToast(`Self-upgrade proposal failed: ${e}`, 'error', 8000);
    } finally {
      submitting = false;
    }
  }

  async function resolve(id: string, approved: boolean) {
    try {
      const msg = await invoke<string>('resolve_self_upgrade_proposal', { proposalId: id, approved });
      addToast(msg, approved ? 'success' : 'info', 8000);
    } catch (e) {
      addToast(`${e}`, 'error', 10000);
    } finally {
      await refreshProposals();
    }
  }

  function toggleExpand(path: string) {
    if (expandedFiles.has(path)) expandedFiles.delete(path);
    else expandedFiles.add(path);
    expandedFiles = expandedFiles;
  }

  function hunksFor(fc: FileChange): DiffHunk[] {
    return parseUnifiedDiff(fc.unified_diff);
  }

  onMount(async () => {
    await loadFlags();
    await refreshProposals();
    const u1 = await listen('self-upgrade-proposal', () => void refreshProposals());
    const u2 = await listen('self-upgrade-proposal-resolved', () => void refreshProposals());
    unlistenFns = [u1, u2];
  });
  onDestroy(() => unlistenFns.forEach((fn) => fn()));
</script>

<div class="self-build">
  <header>
    <h2>🛠 Self-Build</h2>
    <p class="lead">
      Lets an agent propose — and, for low-risk sandboxed-and-tested changes, apply —
      changes to Omnisystem's own source. Every change is built and tested in an isolated git
      worktree first; anything touching a safety-critical file always requires your explicit
      approval before it's even attempted.
    </p>
  </header>

  <section class="toggle-row">
    {#if flagsLoading}
      <span class="muted">Loading settings…</span>
    {:else}
      <label class="toggle">
        <input type="checkbox" bind:checked={selfUpgradeEnabled} on:change={toggleSelfUpgrade} />
        <span>Self-Build enabled</span>
      </label>
      {#if !selfUpgradeEnabled}
        <span class="muted">Off by default — proposals will be rejected by the agent until you turn this on.</span>
      {/if}
    {/if}
  </section>

  <section class="propose-row">
    <textarea
      placeholder="Describe a goal for the self-upgrade agent (e.g. 'add a doc comment to X')…"
      bind:value={goalInput}
      disabled={!selfUpgradeEnabled || submitting}
      rows="2"
    ></textarea>
    <button on:click={submitGoal} disabled={!selfUpgradeEnabled || submitting || !goalInput.trim()}>
      {submitting ? 'Working…' : 'Propose'}
    </button>
  </section>

  <section class="queue">
    <h3>Proposals ({proposals.length})</h3>
    {#if proposals.length === 0}
      <p class="muted">No proposals yet.</p>
    {/if}
    {#each proposals.slice().reverse() as p (p.id)}
      <div class="proposal" class:blocked={p.tier === 'blocked'}>
        <div class="proposal-header">
          <span class="goal">{p.goal}</span>
          <span class="tier-badge tier-{p.tier}">{tierLabel(p.tier)}</span>
          <span class="status-badge status-{p.status}">{statusLabel(p.status)}</span>
        </div>
        <div class="risk-row">
          Trust score: {p.risk.score}/100
          {#if p.risk.touched_safety_critical.length > 0}
            <span class="danger"> — touches: {p.risk.touched_safety_critical.join(', ')}</span>
          {/if}
          {#if p.risk.wide_surface}<span class="warn"> — wide-surface change</span>{/if}
        </div>

        {#if p.sandbox}
          <div class="sandbox-row" class:pass={p.sandbox.build_ok && p.sandbox.test_ok} class:fail={!p.sandbox.build_ok || !p.sandbox.test_ok}>
            Build: {p.sandbox.build_ok ? '✓ passed' : '✗ failed'} ·
            Test: {p.sandbox.test_ok ? '✓ passed' : '✗ failed'}
          </div>
        {/if}

        <div class="files">
          {#each p.files as f (f.path)}
            <div class="file-block">
              <button class="file-toggle" on:click={() => toggleExpand(f.path)}>
                {expandedFiles.has(f.path) ? '▾' : '▸'} {f.path}
              </button>
              {#if expandedFiles.has(f.path)}
                <div class="diff-view">
                  {#each hunksFor(f) as h (h.hunkIndex)}
                    <div class="hunk">
                      {#if h.oldText}<pre class="line del">{h.oldText}</pre>{/if}
                      {#if h.newText}<pre class="line add">{h.newText}</pre>{/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>

        {#if p.status === 'pending_review' || p.status === 'pending_approval'}
          <div class="actions">
            <button class="approve" on:click={() => resolve(p.id, true)}>
              {p.tier === 'blocked' ? 'Run sandbox + Approve & Merge' : 'Approve & Merge'}
            </button>
            <button class="reject" on:click={() => resolve(p.id, false)}>Reject</button>
          </div>
        {/if}
      </div>
    {/each}
  </section>
</div>

<style>
  .self-build { padding: 16px; overflow-y: auto; height: 100%; color: var(--text, #e4e4e7); font-size: 13px; }
  header h2 { margin: 0 0 6px; font-size: 1.1rem; }
  .lead { color: var(--text-dim, #a1a1aa); font-size: 0.82rem; line-height: 1.5; max-width: 640px; }
  .muted { color: var(--text-dim, #71717a); font-size: 0.8rem; }
  .danger { color: #f87171; }
  .warn { color: #fbbf24; }

  .toggle-row { display: flex; align-items: center; gap: 10px; margin: 14px 0; }
  .toggle { display: flex; align-items: center; gap: 6px; cursor: pointer; }

  .propose-row { display: flex; gap: 8px; margin-bottom: 18px; }
  .propose-row textarea {
    flex: 1; resize: vertical; background: var(--bg, #18181b); border: 1px solid var(--border, #3f3f46);
    border-radius: 8px; color: var(--text); padding: 8px; font-size: 0.85rem;
  }
  .propose-row button, .actions button {
    border: none; border-radius: 8px; padding: 8px 14px; cursor: pointer; font-size: 0.82rem; font-weight: 600;
  }
  .propose-row button { background: var(--accent, #16a34a); color: #fff; align-self: flex-start; }
  .propose-row button:disabled { opacity: 0.5; cursor: default; }

  .queue h3 { font-size: 0.9rem; margin-bottom: 8px; }
  .proposal {
    border: 1px solid var(--border, #3f3f46); border-radius: 10px; padding: 12px; margin-bottom: 12px;
    background: var(--bg2, #1c1c1f);
  }
  .proposal.blocked { border-color: #f87171; }
  .proposal-header { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 6px; }
  .goal { font-weight: 600; flex: 1; min-width: 180px; }
  .tier-badge, .status-badge { font-size: 0.72rem; padding: 2px 8px; border-radius: 999px; white-space: nowrap; }
  .tier-auto_merge { background: rgba(34,197,94,0.18); color: #4ade80; }
  .tier-staged_approval { background: rgba(251,191,36,0.18); color: #fbbf24; }
  .tier-blocked { background: rgba(248,113,113,0.18); color: #f87171; }
  .status-badge { background: var(--bg, #18181b); color: var(--text-dim, #a1a1aa); }

  .risk-row { font-size: 0.78rem; color: var(--text-dim, #a1a1aa); margin-bottom: 6px; }
  .sandbox-row { font-size: 0.78rem; margin-bottom: 8px; padding: 4px 8px; border-radius: 6px; }
  .sandbox-row.pass { background: rgba(34,197,94,0.12); color: #4ade80; }
  .sandbox-row.fail { background: rgba(248,113,113,0.12); color: #f87171; }

  .file-block { margin-bottom: 4px; }
  .file-toggle {
    background: none; border: none; color: var(--text); cursor: pointer; font-size: 0.8rem;
    padding: 4px 0; font-family: monospace;
  }
  .diff-view { background: var(--bg, #18181b); border-radius: 6px; padding: 6px; margin: 4px 0 8px; }
  .hunk { margin-bottom: 4px; }
  .line { margin: 0; padding: 2px 6px; font-size: 0.75rem; white-space: pre-wrap; word-break: break-all; border-radius: 3px; }
  .line.add { background: rgba(34,197,94,0.12); color: #86efac; }
  .line.del { background: rgba(248,113,113,0.12); color: #fca5a5; }

  .actions { display: flex; gap: 8px; margin-top: 8px; }
  .actions .approve { background: var(--accent, #16a34a); color: #fff; }
  .actions .reject { background: transparent; border: 1px solid var(--border, #3f3f46) !important; color: var(--text); }
</style>
