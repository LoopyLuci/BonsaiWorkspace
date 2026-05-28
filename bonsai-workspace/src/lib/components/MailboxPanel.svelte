<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  // ── State ──────────────────────────────────────────────────────────────────
  let identity: { fingerprint: string } | null = null;
  let agentCount = 0;
  let toFingerprint = '';
  let topic = '';
  let text = '';
  let busy = false;
  let error = '';
  let successMsg = '';
  let sentLog: Array<{ to: string; topic: string; text: string; ts: number }> = [];

  // ── Init ───────────────────────────────────────────────────────────────────
  onMount(async () => {
    try {
      identity = await invoke('transfer_get_identity');
      agentCount = await invoke<number>('transfer_mailbox_agent_count');
    } catch { /* no identity loaded */ }
  });

  // ── Actions ────────────────────────────────────────────────────────────────
  async function sendMessage() {
    if (!toFingerprint) { error = 'Enter recipient fingerprint.'; return; }
    if (!topic) { error = 'Enter a topic.'; return; }
    if (!text) { error = 'Message text is empty.'; return; }
    busy = true; error = ''; successMsg = '';
    try {
      await invoke('transfer_send_message', {
        toFingerprint,
        topic,
        text,
      });
      sentLog = [{ to: toFingerprint, topic, text, ts: Date.now() }, ...sentLog].slice(0, 50);
      successMsg = 'Message delivered to local mailbox.';
      text = '';
    } catch (e: any) {
      error = String(e);
    } finally { busy = false; }
  }

  function formatTs(ts: number): string {
    return new Date(ts).toLocaleTimeString();
  }
</script>

<div class="mailbox-panel">
  <h2 class="panel-title">✉ Agent Mailbox</h2>

  {#if !identity}
    <p class="hint warn">No identity loaded. Use the Identity panel to create or unlock your identity first.</p>
  {:else}
    <div class="id-row">
      <span class="label">Your fingerprint:</span>
      <code class="fp">{identity.fingerprint}</code>
      <span class="agent-count">{agentCount} agent{agentCount !== 1 ? 's' : ''} registered</span>
    </div>

    {#if error}
      <div class="msg error">{error}</div>
    {/if}
    {#if successMsg}
      <div class="msg success">{successMsg}</div>
    {/if}

    <!-- Compose form -->
    <div class="compose">
      <h3>Send Message</h3>
      <label class="field-row">
        <span>Recipient Fingerprint</span>
        <input bind:value={toFingerprint} placeholder="8-char hex fingerprint of recipient agent" />
      </label>
      <label class="field-row">
        <span>Topic</span>
        <input bind:value={topic} placeholder="e.g. ping, inference-request, model-shard" />
      </label>
      <label class="field-row">
        <span>Message</span>
        <textarea bind:value={text} rows="3" placeholder="Plain-text payload…" />
      </label>
      <button class="btn primary" on:click={sendMessage} disabled={busy}>
        {busy ? 'Sending…' : '▶ Send'}
      </button>
    </div>

    <!-- Sent log -->
    {#if sentLog.length}
      <div class="log-header">Sent this session ({sentLog.length})</div>
      <div class="log">
        {#each sentLog as m}
          <div class="log-item">
            <span class="log-ts">{formatTs(m.ts)}</span>
            <span class="log-to">→ <code>{m.to.slice(0, 8)}</code></span>
            <span class="log-topic">[{m.topic}]</span>
            <span class="log-text">{m.text.slice(0, 80)}{m.text.length > 80 ? '…' : ''}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .mailbox-panel { padding: 16px; }
  .panel-title { font-size: 1.1rem; margin-bottom: 12px; }
  .hint { font-size: 0.82rem; color: var(--text-muted, #888); }
  .hint.warn { color: #facc15; }
  .id-row { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; font-size: 0.82rem; flex-wrap: wrap; }
  .label { color: var(--text-muted, #888); }
  .fp { font-family: monospace; font-size: 0.8rem; }
  .agent-count { margin-left: auto; font-size: 0.75rem; color: var(--text-muted, #888); }
  .msg { padding: 8px 12px; border-radius: 4px; margin-bottom: 10px; font-size: 0.82rem; }
  .msg.error { background: #3b0a0a; color: #f87171; }
  .msg.success { background: #052e16; color: #4ade80; }
  .compose { background: var(--surface, #1e1e2e); border-radius: 6px; padding: 14px; margin-bottom: 16px; }
  .compose h3 { font-size: 0.9rem; margin-bottom: 10px; }
  .field-row { display: flex; flex-direction: column; gap: 4px; font-size: 0.83rem; margin-bottom: 8px; }
  .field-row input, .field-row textarea { padding: 6px 8px; border-radius: 4px; border: 1px solid var(--border, #444); background: var(--input-bg, #12121a); color: inherit; font-size: 0.83rem; resize: vertical; }
  .btn { padding: 7px 16px; border-radius: 5px; border: none; cursor: pointer; font-size: 0.83rem; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn.primary { background: var(--accent, #7c3aed); color: #fff; }
  .log-header { font-size: 0.8rem; color: var(--text-muted, #888); margin-bottom: 6px; }
  .log { display: flex; flex-direction: column; gap: 4px; }
  .log-item { display: flex; gap: 8px; font-size: 0.78rem; background: var(--surface, #1e1e2e); padding: 6px 10px; border-radius: 4px; flex-wrap: wrap; }
  .log-ts { color: var(--text-muted, #888); white-space: nowrap; }
  .log-to code { font-family: monospace; }
  .log-topic { color: #60a5fa; }
  .log-text { flex: 1; word-break: break-word; }
</style>
