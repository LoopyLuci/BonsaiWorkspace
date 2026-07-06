<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  interface PlanPendingPayload {
    plan_id:    string;
    tool:       string;
    args:       Record<string, unknown>;
    risk:       'High' | 'Medium' | 'Low';
    reason:     string;
    expires_at: number; // unix seconds
  }

  let pending: PlanPendingPayload[] = [];
  let unlisten: (() => void) | null = null;
  let timers: number[] = [];

  onMount(async () => {
    unlisten = await listen<PlanPendingPayload>('plan-pending', (evt) => {
      pending = [...pending, evt.payload];
      const ms = (evt.payload.expires_at - Math.floor(Date.now() / 1000)) * 1000;
      const t = window.setTimeout(() => {
        pending = pending.filter(p => p.plan_id !== evt.payload.plan_id);
      }, Math.max(ms, 1000));
      timers.push(t);
    });
  });

  onDestroy(() => {
    unlisten?.();
    timers.forEach(clearTimeout);
  });

  async function approve(p: PlanPendingPayload) {
    try {
      await invoke('resolve_plan', { planId: p.plan_id, approved: true });
    } catch { /* already expired */ }
    pending = pending.filter(x => x.plan_id !== p.plan_id);
  }

  async function reject(p: PlanPendingPayload) {
    try {
      await invoke('resolve_plan', { planId: p.plan_id, approved: false });
    } catch { /* already expired */ }
    pending = pending.filter(x => x.plan_id !== p.plan_id);
  }

  function secondsLeft(expiresAt: number): number {
    return Math.max(0, expiresAt - Math.floor(Date.now() / 1000));
  }

  function riskColor(risk: string): string {
    if (risk === 'High')   return '#e05260';
    if (risk === 'Medium') return '#f5a623';
    return '#6dbf67';
  }

  function argsPreview(args: Record<string, unknown>): string {
    try {
      return JSON.stringify(args, null, 2);
    } catch {
      return String(args);
    }
  }

  let expanded: Record<string, boolean> = {};
  function toggle(id: string) { expanded[id] = !expanded[id]; expanded = { ...expanded }; }
</script>

{#each pending as p (p.plan_id)}
  <div class="plan-card" role="dialog" aria-modal="true" aria-labelledby="plan-title-{p.plan_id}">
    <div class="header">
      <span class="risk-badge" style="background: {riskColor(p.risk)}20; color: {riskColor(p.risk)}; border-color: {riskColor(p.risk)}">
        {p.risk} Risk
      </span>
      <span class="tool-name" id="plan-title-{p.plan_id}">
        <code>{p.tool}</code>
      </span>
      <span class="ttl">{secondsLeft(p.expires_at)}s</span>
    </div>

    <div class="reason">{p.reason}</div>

    <button class="expand-toggle" on:click={() => toggle(p.plan_id)}>
      {expanded[p.plan_id] ? '▾ Hide args' : '▸ Show args'}
    </button>

    {#if expanded[p.plan_id]}
      <pre class="args-preview">{argsPreview(p.args)}</pre>
    {/if}

    <div class="actions">
      <button class="approve" on:click={() => approve(p)}>Approve</button>
      <button class="reject"  on:click={() => reject(p)}>Block</button>
    </div>
  </div>
{/each}

<style>
  .plan-card {
    margin: 4px 8px;
    padding: 10px 12px;
    background: var(--bg2, #252526);
    border: 1px solid #f5a623;
    border-radius: 10px;
    font-size: 0.84rem;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .risk-badge {
    font-size: 0.72rem;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 4px;
    border: 1px solid;
    white-space: nowrap;
  }

  .tool-name { flex: 1; color: var(--fg, #ccc); font-weight: 600; }
  .ttl { font-size: 0.72rem; color: var(--fg-dim, #888); }

  code {
    font-family: monospace;
    background: var(--bg, #1e1e1e);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .reason { color: var(--fg-dim, #aaa); font-size: 0.82rem; }

  .expand-toggle {
    background: none;
    border: none;
    color: var(--accent, #5ca4ea);
    cursor: pointer;
    font-size: 0.78rem;
    padding: 0;
    text-align: left;
  }

  .args-preview {
    background: var(--bg, #1e1e1e);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 6px;
    padding: 8px;
    font-size: 0.75rem;
    color: var(--fg-dim, #aaa);
    overflow-x: auto;
    max-height: 180px;
    overflow-y: auto;
    white-space: pre;
    margin: 0;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .approve {
    background: var(--accent, #5ca4ea);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 4px 14px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .approve:hover { background: var(--accent-hover, #4a93d9); }

  .reject {
    background: transparent;
    border: 1px solid var(--danger, #e05260);
    color: var(--danger, #e05260);
    border-radius: 6px;
    padding: 4px 12px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .reject:hover { background: color-mix(in srgb, var(--danger, #e05260) 15%, transparent); }
</style>
