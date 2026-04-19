<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  interface SidecarHealth { name: string; healthy: boolean; last_checked_ts: number; consecutive_failures: number; }
  interface AssistantHealth { sidecars: SidecarHealth[]; db_ok: boolean; last_error: string | null; checked_at: number; }
  interface MetricsSnapshot {
    turn_count: number; avg_turn_ms: number;
    tool_call_count: number; tool_error_rate_pct: number;
    tts_count: number; avg_tts_ms: number; tts_error_count: number;
    session_restore_count: number; sidecar_restart_count: number;
    last_errors: Array<{ ts: number; context: string; message: string }>;
  }

  export let onClose: () => void = () => {};

  let metrics: MetricsSnapshot | null = null;
  let health: AssistantHealth | null = null;
  let auditLines: string[] = [];
  let unlisten: UnlistenFn | null = null;

  async function refresh() {
    try {
      [metrics, health, auditLines] = await Promise.all([
        invoke<MetricsSnapshot>('get_assistant_metrics'),
        invoke<AssistantHealth>('get_assistant_health'),
        invoke<string[]>('get_assistant_audit_log'),
      ]);
    } catch (e) {
      console.error('diagnostics refresh', e);
    }
  }

  onMount(async () => {
    await refresh();
    unlisten = await listen<AssistantHealth>('assistant-health', e => { health = e.payload; });
  });
  onDestroy(() => unlisten?.());
</script>

<div class="diag">
  <div class="diag-header">
    <span>Diagnostics</span>
    <button class="close-btn" on:click={onClose}>✕</button>
  </div>

  <section>
    <h3>Sidecar Health</h3>
    {#if health}
      <div class="health-row">
        <span class="dot" class:ok={health.db_ok} class:fail={!health.db_ok}></span> SQLite DB
      </div>
      {#each health.sidecars as sc}
        <div class="health-row">
          <span class="dot" class:ok={sc.healthy} class:fail={!sc.healthy}></span>
          {sc.name}
          {#if !sc.healthy}<span class="fail-count">({sc.consecutive_failures} fail)</span>{/if}
        </div>
      {/each}
      {#if health.last_error}<div class="err-line">{health.last_error}</div>{/if}
    {:else}
      <p class="dim">Loading…</p>
    {/if}
  </section>

  <section>
    <h3>Metrics</h3>
    {#if metrics}
      <table>
        <tr><td>Turns</td><td>{metrics.turn_count} (avg {metrics.avg_turn_ms}ms)</td></tr>
        <tr><td>Tool calls</td><td>{metrics.tool_call_count} ({metrics.tool_error_rate_pct.toFixed(1)}% err)</td></tr>
        <tr><td>TTS synth</td><td>{metrics.tts_count} (avg {metrics.avg_tts_ms}ms, {metrics.tts_error_count} err)</td></tr>
        <tr><td>Session restores</td><td>{metrics.session_restore_count}</td></tr>
        <tr><td>Sidecar restarts</td><td>{metrics.sidecar_restart_count}</td></tr>
      </table>
      {#if metrics.last_errors.length > 0}
        <h4>Recent Errors</h4>
        <div class="err-scroll">
          {#each metrics.last_errors as e}
            <div class="err-entry"><span class="ctx">[{e.context}]</span> {e.message}</div>
          {/each}
        </div>
      {/if}
    {:else}
      <p class="dim">Loading…</p>
    {/if}
  </section>

  <section>
    <h3>Audit Log <span class="dim">(last 200)</span></h3>
    <div class="log-scroll">
      {#each auditLines as line}
        <div class="log-line">{line}</div>
      {/each}
      {#if auditLines.length === 0}<p class="dim">No entries yet.</p>{/if}
    </div>
  </section>

  <div class="diag-footer">
    <button on:click={refresh}>Refresh</button>
  </div>
</div>

<style>
  .diag {
    display: flex; flex-direction: column; height: 100%;
    background: var(--bg); color: var(--fg); font-size: 0.82rem;
    overflow: hidden;
  }
  .diag-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-bottom: 1px solid var(--border);
    font-weight: 600;
  }
  .close-btn { background: none; border: none; color: var(--fg-dim); cursor: pointer; font-size: 1rem; }
  section { padding: 8px 12px; border-bottom: 1px solid var(--border); }
  h3 { margin: 0 0 6px; font-size: 0.8rem; color: var(--fg-dim); text-transform: uppercase; letter-spacing: 0.05em; }
  h4 { margin: 8px 0 4px; font-size: 0.78rem; color: var(--fg-dim); }
  .health-row { display: flex; align-items: center; gap: 6px; margin-bottom: 4px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--fg-dim); flex-shrink: 0; }
  .dot.ok   { background: #4ec94e; }
  .dot.fail { background: var(--danger); }
  .fail-count { color: var(--danger); font-size: 0.75rem; }
  table { border-collapse: collapse; width: 100%; }
  td { padding: 2px 6px 2px 0; }
  td:first-child { color: var(--fg-dim); white-space: nowrap; }
  .err-scroll, .log-scroll {
    max-height: 110px; overflow-y: auto;
    background: var(--bg2); border-radius: 4px; padding: 4px 6px;
  }
  .log-scroll { max-height: 160px; font-family: monospace; font-size: 0.75rem; }
  .err-entry, .log-line { margin-bottom: 2px; word-break: break-all; }
  .err-line { color: var(--danger); margin-top: 4px; }
  .ctx { color: var(--accent); }
  .dim { color: var(--fg-dim); }
  .diag-footer {
    padding: 8px 12px; display: flex; justify-content: flex-end;
  }
  button { background: var(--bg2); border: 1px solid var(--border); color: var(--fg); padding: 4px 12px; border-radius: 4px; cursor: pointer; }
  button:hover { background: var(--border); }
</style>
