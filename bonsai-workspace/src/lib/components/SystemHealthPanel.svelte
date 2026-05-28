<script lang="ts">
  /**
   * SystemHealthPanel — live system health overlay.
   *
   * Shows:
   * - Sidecar status dots (llama-server, whisper)
   * - IPC failure log (last 10 from ipcHealth store)
   * - WebSocket connection state
   * - Memory pressure bar
   * - Crash recovery notice (if applicable)
   */
  import { onMount, onDestroy } from 'svelte';
  import { ipcHealth, permanentFailureCount } from '$lib/stores/ipcHealth';
  import { resilientInvoke } from '$lib/utils/ipc';

  export let onClose: () => void = () => {};

  // ── Sidecar health (from Tauri assistant-health events) ─────────────────
  interface SidecarHealth {
    name:   string;
    healthy: boolean;
  }
  let sidecars: SidecarHealth[] = [
    { name: 'llama-server', healthy: false },
    { name: 'whisper',      healthy: false },
  ];

  // ── Memory ────────────────────────────────────────────────────────────────
  let memoryPressure = 0;   // 0–100
  let memoryPollTimer: ReturnType<typeof setInterval> | null = null;

  // ── WebSocket state ───────────────────────────────────────────────────────
  let wsState = 'unknown';

  // ── Recovery ─────────────────────────────────────────────────────────────
  let crashRecovered = false;

  // ── Tauri event listener ──────────────────────────────────────────────────
  let unlisten: (() => void) | null = null;
  let unlistenRecovery: (() => void) | null = null;

  onMount(async () => {
    // Listen for sidecar health updates emitted by lib.rs watchdog.
    try {
      const { listen } = await import('@tauri-apps/api/event');
      unlisten = await listen<{ sidecars: SidecarHealth[] }>('assistant-health', (ev) => {
        sidecars = ev.payload.sidecars ?? sidecars;
      });
      unlistenRecovery = await listen<{ crashed: boolean }>('recovery-state', (ev) => {
        crashRecovered = ev.payload.crashed;
      });
    } catch { /* non-Tauri environment */ }

    // Poll memory pressure every 5 s.
    const fetchMemory = async () => {
      try {
        const pct = await resilientInvoke<number>('get_memory_pressure');
        memoryPressure = Math.round((pct ?? 0) * 100);
      } catch { /* best-effort */ }
    };
    fetchMemory();
    memoryPollTimer = setInterval(fetchMemory, 5000);
  });

  onDestroy(() => {
    unlisten?.();
    unlistenRecovery?.();
    if (memoryPollTimer) clearInterval(memoryPollTimer);
  });

  function dotClass(healthy: boolean) {
    return healthy ? 'dot green' : 'dot red';
  }

  function formatTime(ts: number) {
    return new Date(ts).toLocaleTimeString();
  }
</script>

<div class="health-panel" role="dialog" aria-label="System Health">
  <header class="panel-header">
    <span>System Health</span>
    <button class="close-btn" on:click={onClose} aria-label="Close">✕</button>
  </header>

  <!-- ── Crash recovery notice ──────────────────────────────────────────── -->
  {#if crashRecovered}
    <div class="recovery-notice">
      Recovered from an unexpected shutdown. All data is intact.
    </div>
  {/if}

  <!-- ── Sidecar dots ───────────────────────────────────────────────────── -->
  <section class="section">
    <h3 class="section-title">Sidecars</h3>
    <ul class="sidecar-list">
      {#each sidecars as s}
        <li class="sidecar-row">
          <span class={dotClass(s.healthy)}></span>
          <span class="sidecar-name">{s.name}</span>
          <span class="sidecar-status">{s.healthy ? 'Running' : 'Down'}</span>
        </li>
      {/each}
    </ul>
  </section>

  <!-- ── Memory pressure ───────────────────────────────────────────────── -->
  <section class="section">
    <h3 class="section-title">Memory</h3>
    <div class="bar-track">
      <div
        class="bar-fill"
        style="width: {memoryPressure}%; background: {memoryPressure > 85 ? '#ef4444' : memoryPressure > 65 ? '#f59e0b' : '#22c55e'}"
      ></div>
    </div>
    <span class="bar-label">{memoryPressure}% used</span>
  </section>

  <!-- ── IPC failure log ────────────────────────────────────────────────── -->
  <section class="section">
    <h3 class="section-title">
      IPC Log
      {#if $permanentFailureCount > 0}
        <span class="badge red">{$permanentFailureCount} permanent</span>
      {/if}
    </h3>
    {#if $ipcHealth.length === 0}
      <p class="empty-msg">No failures recorded.</p>
    {:else}
      <ul class="ipc-list">
        {#each $ipcHealth.slice(0, 10) as f}
          <li class="ipc-row" class:permanent={f.permanent}>
            <span class="ipc-cmd">{f.command}</span>
            <span class="ipc-err" title={f.error}>{f.error.slice(0, 60)}{f.error.length > 60 ? '…' : ''}</span>
            <span class="ipc-time">{formatTime(f.timestamp)}</span>
            <span class="ipc-badge" class:perm={f.permanent}>{f.permanent ? 'failed' : `retry ${f.attempt}`}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .health-panel {
    position: fixed;
    bottom: 56px;
    right: 20px;
    width: 380px;
    max-height: 520px;
    overflow-y: auto;
    background: var(--bg-surface, #1e1e2e);
    border: 1px solid var(--border, #313244);
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
    z-index: var(--z-overlay, 2000);
    font-size: 12px;
    color: var(--text, #cdd6f4);
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border, #313244);
    font-weight: 600;
    font-size: 13px;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted, #6c7086);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
  }
  .close-btn:hover { color: var(--text, #cdd6f4); }

  .recovery-notice {
    margin: 8px 14px;
    padding: 6px 10px;
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.3);
    border-radius: 6px;
    color: #22c55e;
    font-size: 11px;
  }

  .section { padding: 10px 14px; border-bottom: 1px solid var(--border, #313244); }
  .section:last-child { border-bottom: none; }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #6c7086);
    margin: 0 0 8px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .badge {
    padding: 1px 6px;
    border-radius: 10px;
    font-size: 10px;
    font-weight: 700;
    text-transform: none;
    letter-spacing: 0;
  }
  .badge.red { background: rgba(239,68,68,0.15); color: #ef4444; }

  /* Sidecar dots */
  .sidecar-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 5px; }
  .sidecar-row  { display: flex; align-items: center; gap: 8px; }
  .dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.green { background: #22c55e; box-shadow: 0 0 4px #22c55e88; }
  .dot.red   { background: #ef4444; box-shadow: 0 0 4px #ef444488; }
  .sidecar-name   { flex: 1; color: var(--text, #cdd6f4); }
  .sidecar-status { color: var(--text-muted, #6c7086); font-size: 11px; }

  /* Memory bar */
  .bar-track { height: 6px; background: var(--bg-base, #181825); border-radius: 3px; overflow: hidden; margin-bottom: 4px; }
  .bar-fill  { height: 100%; border-radius: 3px; transition: width 0.5s ease, background 0.5s ease; }
  .bar-label { font-size: 11px; color: var(--text-muted, #6c7086); }

  /* IPC log */
  .ipc-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .ipc-row {
    display: grid;
    grid-template-columns: 1fr 2fr auto auto;
    gap: 6px;
    align-items: center;
    padding: 4px 0;
    border-bottom: 1px solid rgba(255,255,255,0.04);
    font-size: 11px;
  }
  .ipc-row.permanent { background: rgba(239,68,68,0.05); }
  .ipc-cmd  { color: var(--accent, #89b4fa); font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ipc-err  { color: var(--text-muted, #6c7086); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ipc-time { color: var(--text-muted, #6c7086); font-size: 10px; white-space: nowrap; }
  .ipc-badge {
    padding: 1px 5px;
    border-radius: 8px;
    font-size: 10px;
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
    white-space: nowrap;
  }
  .ipc-badge.perm { background: rgba(239,68,68,0.15); color: #ef4444; }

  .empty-msg { color: var(--text-muted, #6c7086); font-size: 11px; margin: 0; }
</style>
