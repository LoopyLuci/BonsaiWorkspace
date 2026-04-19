<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  interface PendingConfirm {
    token: string;
    tool: string;
    prompt: string;
    expires_at: number;
  }

  let pending: PendingConfirm[] = [];
  let unlisten: (() => void) | null = null;
  let timers: number[] = [];

  onMount(async () => {
    unlisten = await listen<PendingConfirm>('assistant-confirm-required', (evt) => {
      pending = [...pending, evt.payload];
      // Auto-expire display after TTL
      const ms = (evt.payload.expires_at - Math.floor(Date.now() / 1000)) * 1000;
      const t = window.setTimeout(() => {
        pending = pending.filter(p => p.token !== evt.payload.token);
      }, Math.max(ms, 0));
      timers.push(t);
    });
  });

  onDestroy(() => {
    unlisten?.();
    timers.forEach(clearTimeout);
  });

  async function approve(p: PendingConfirm) {
    try {
      await invoke('confirm_tool_action', { token: p.token });
    } catch { /* already expired or consumed */ }
    pending = pending.filter(x => x.token !== p.token);
  }

  async function deny(p: PendingConfirm) {
    try {
      await invoke('cancel_tool_action', { token: p.token });
    } catch { /* no-op */ }
    pending = pending.filter(x => x.token !== p.token);
  }

  function secondsLeft(expiresAt: number): number {
    return Math.max(0, expiresAt - Math.floor(Date.now() / 1000));
  }
</script>

{#each pending as p (p.token)}
  <div class="confirm-card">
    <div class="icon">⚠️</div>
    <div class="body">
      <div class="prompt">{p.prompt}</div>
      <div class="tool-name">Tool: <code>{p.tool}</code></div>
    </div>
    <div class="actions">
      <button class="approve" on:click={() => approve(p)}>
        Approve <span class="ttl">({secondsLeft(p.expires_at)}s)</span>
      </button>
      <button class="deny" on:click={() => deny(p)}>Cancel</button>
    </div>
  </div>
{/each}

<style>
  .confirm-card {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 4px 8px;
    padding: 8px 12px;
    background: var(--bg2, #252526);
    border: 1px solid #f5a623;
    border-radius: 10px;
    font-size: 0.84rem;
  }
  .icon { font-size: 1.2rem; flex-shrink: 0; }
  .body { flex: 1; }
  .prompt { font-weight: 600; color: var(--fg, #ccc); }
  .tool-name { color: var(--fg-dim, #888); margin-top: 2px; }
  code { font-family: monospace; background: var(--bg, #1e1e1e); padding: 1px 4px; border-radius: 3px; }
  .actions { display: flex; gap: 6px; align-items: center; flex-shrink: 0; }
  .approve {
    background: var(--accent, #5ca4ea);
    color: #fff;
    border: none; border-radius: 6px;
    padding: 4px 10px;
    cursor: pointer;
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .approve:hover { background: var(--accent-hover, #4a93d9); }
  .deny {
    background: transparent;
    border: 1px solid var(--border, #3e3e42);
    color: var(--fg-dim, #888);
    border-radius: 6px;
    padding: 4px 8px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .deny:hover { border-color: var(--danger, #e05260); color: var(--danger, #e05260); }
  .ttl { opacity: 0.7; font-size: 0.75rem; }
</style>
