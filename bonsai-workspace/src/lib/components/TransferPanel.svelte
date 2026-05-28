<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  // ── Types ──────────────────────────────────────────────────────────────────
  interface TransferStatusDto {
    id: string;
    direction: string;
    total_bytes: number;
    transferred_bytes: number;
    chunk_count: number;
    chunks_done: number;
    state: string;
    bytes_per_sec: number;
    progress_pct: number;
  }

  // ── State ──────────────────────────────────────────────────────────────────
  let transfers: TransferStatusDto[] = [];
  let error = '';
  let filePath = '';
  let chunkSizeKib = 256;
  let busy = false;
  let pollInterval: ReturnType<typeof setInterval>;

  const STATE_COLORS: Record<string, string> = {
    complete: '#4ade80',
    active: '#60a5fa',
    pending: '#facc15',
    failed: '#f87171',
    cancelled: '#888',
  };

  // ── Helpers ────────────────────────────────────────────────────────────────
  function fmt(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1_048_576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1_048_576).toFixed(1)} MB`;
  }

  function stateColor(state: string): string {
    return STATE_COLORS[state] ?? '#aaa';
  }

  // ── Data loading ───────────────────────────────────────────────────────────
  async function loadTransfers() {
    try {
      transfers = await invoke<TransferStatusDto[]>('transfer_list_transfers');
    } catch (e: any) {
      error = String(e);
    }
  }

  // ── Send ───────────────────────────────────────────────────────────────────
  async function sendLoopback() {
    if (!filePath) { error = 'Enter a file path.'; return; }
    busy = true; error = '';
    try {
      const status = await invoke<TransferStatusDto>('transfer_send_file_loopback', {
        filePath,
        chunkSize: chunkSizeKib * 1024,
      });
      transfers = [status, ...transfers];
      filePath = '';
    } catch (e: any) {
      error = String(e);
    } finally { busy = false; }
  }

  onMount(() => {
    loadTransfers();
    pollInterval = setInterval(loadTransfers, 2000);
  });

  onDestroy(() => clearInterval(pollInterval));
</script>

<div class="transfer-panel">
  <h2 class="panel-title">📦 File Transfers</h2>

  {#if error}
    <div class="msg error">{error}</div>
  {/if}

  <!-- Send form -->
  <div class="send-form">
    <h3>Self-test Loopback Send</h3>
    <p class="hint">Sends a file through the full encrypt→chunk→schedule→send pipeline using an in-process lane.</p>

    <div class="row">
      <input
        class="path-input"
        bind:value={filePath}
        placeholder="Absolute file path…"
        on:keydown={e => e.key === 'Enter' && sendLoopback()}
      />
      <label class="chunk-label">
        Chunk
        <select bind:value={chunkSizeKib}>
          <option value={64}>64 KB</option>
          <option value={256}>256 KB</option>
          <option value={1024}>1 MB</option>
          <option value={4096}>4 MB</option>
        </select>
      </label>
      <button class="btn primary" on:click={sendLoopback} disabled={busy || !filePath}>
        {busy ? 'Sending…' : '▶ Send'}
      </button>
    </div>
  </div>

  <!-- Transfer list -->
  <div class="list-header">
    <span>Transfers ({transfers.length})</span>
    <button class="btn ghost" on:click={loadTransfers}>↻ Refresh</button>
  </div>

  {#if transfers.length === 0}
    <p class="hint empty">No transfers yet.</p>
  {:else}
    <div class="transfer-list">
      {#each transfers as t (t.id)}
        <div class="transfer-item">
          <div class="t-head">
            <span class="t-id mono">{t.id.slice(0, 8)}…</span>
            <span class="t-dir">{t.direction === 'send' ? '↑' : '↓'}</span>
            <span class="t-state" style="color:{stateColor(t.state)}">{t.state}</span>
            <span class="t-size">{fmt(t.transferred_bytes)} / {fmt(t.total_bytes)}</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" style="width:{t.progress_pct}%; background:{stateColor(t.state)}"></div>
          </div>
          <div class="t-detail">
            {t.chunks_done}/{t.chunk_count} chunks
            {#if t.bytes_per_sec > 0}
              · {fmt(t.bytes_per_sec)}/s
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .transfer-panel { padding: 16px; }
  .panel-title { font-size: 1.1rem; margin-bottom: 12px; }
  .msg { padding: 8px 12px; border-radius: 4px; margin-bottom: 10px; font-size: 0.85rem; }
  .msg.error { background: #3b0a0a; color: #f87171; }
  .send-form { background: var(--surface, #1e1e2e); border-radius: 6px; padding: 14px; margin-bottom: 16px; }
  .send-form h3 { font-size: 0.9rem; margin-bottom: 6px; }
  .hint { font-size: 0.8rem; color: var(--text-muted, #888); margin-bottom: 10px; }
  .hint.empty { text-align: center; padding: 20px 0; }
  .row { display: flex; gap: 8px; align-items: center; }
  .path-input { flex: 1; padding: 6px 8px; border-radius: 4px; border: 1px solid var(--border, #444); background: var(--input-bg, #12121a); color: inherit; font-size: 0.85rem; }
  .chunk-label { font-size: 0.8rem; display: flex; align-items: center; gap: 4px; white-space: nowrap; }
  .chunk-label select { padding: 4px 6px; border-radius: 4px; border: 1px solid var(--border, #444); background: var(--input-bg, #12121a); color: inherit; font-size: 0.8rem; }
  .list-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; font-size: 0.85rem; }
  .transfer-list { display: flex; flex-direction: column; gap: 8px; }
  .transfer-item { background: var(--surface, #1e1e2e); border-radius: 5px; padding: 10px 12px; }
  .t-head { display: flex; gap: 10px; align-items: center; margin-bottom: 6px; font-size: 0.82rem; }
  .t-id { flex: 1; }
  .t-dir { font-size: 1rem; }
  .t-size { margin-left: auto; color: var(--text-muted, #888); }
  .progress-bar { height: 4px; background: var(--border, #333); border-radius: 2px; overflow: hidden; margin-bottom: 4px; }
  .progress-fill { height: 100%; border-radius: 2px; transition: width 0.3s; }
  .t-detail { font-size: 0.75rem; color: var(--text-muted, #888); }
  .mono { font-family: monospace; }
  .btn { padding: 6px 14px; border-radius: 4px; border: none; cursor: pointer; font-size: 0.82rem; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn.primary { background: var(--accent, #7c3aed); color: #fff; }
  .btn.ghost { background: none; border: 1px solid var(--border, #444); color: inherit; }
</style>
