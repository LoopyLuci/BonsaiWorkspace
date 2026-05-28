<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  // ── State ──────────────────────────────────────────────────────────────────
  let identity: { fingerprint: string; public_key_hex: string } | null = null;
  let hasStored = false;
  let busy = false;
  let error = '';
  let successMsg = '';

  // Form state
  let tab: 'status' | 'create' | 'unlock' = 'status';
  let phrase = '';
  let passphrase = '';
  let generatedPhrase = '';
  let copied = false;

  // ── Init ───────────────────────────────────────────────────────────────────
  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    error = '';
    try {
      hasStored = await invoke<boolean>('transfer_has_stored_identity');
      identity = await invoke('transfer_get_identity');
      if (hasStored && !identity) tab = 'unlock';
      else if (!hasStored) tab = 'create';
    } catch (e: any) {
      error = String(e);
    }
  }

  // ── Actions ────────────────────────────────────────────────────────────────
  async function generatePhrase() {
    busy = true; error = ''; generatedPhrase = '';
    try {
      generatedPhrase = await invoke<string>('transfer_generate_phrase');
      phrase = generatedPhrase;
    } catch (e: any) {
      error = String(e);
    } finally { busy = false; }
  }

  async function createIdentity() {
    if (!phrase) { error = 'Enter or generate a recovery phrase first.'; return; }
    busy = true; error = ''; successMsg = '';
    try {
      identity = await invoke('transfer_create_identity', { phrase, passphrase });
      successMsg = 'Identity created. Write down your recovery phrase — it cannot be recovered.';
      tab = 'status';
    } catch (e: any) {
      error = String(e);
    } finally { busy = false; }
  }

  async function unlockIdentity() {
    if (!passphrase) { error = 'Enter your passphrase.'; return; }
    busy = true; error = ''; successMsg = '';
    try {
      identity = await invoke('transfer_unlock_identity', { passphrase });
      successMsg = 'Identity unlocked.';
      tab = 'status';
    } catch (e: any) {
      error = 'Wrong passphrase or corrupted store.';
    } finally { busy = false; }
  }

  async function copyPhrase() {
    await navigator.clipboard.writeText(generatedPhrase);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<div class="transfer-identity-panel">
  <h2 class="panel-title">🔑 Bonsai Identity</h2>

  {#if error}
    <div class="msg error">{error}</div>
  {/if}
  {#if successMsg}
    <div class="msg success">{successMsg}</div>
  {/if}

  <!-- Tab bar -->
  <div class="tabs">
    <button class:active={tab === 'status'} on:click={() => tab = 'status'}>Status</button>
    {#if !identity}
      <button class:active={tab === 'create'} on:click={() => tab = 'create'}>Create</button>
      {#if hasStored}
        <button class:active={tab === 'unlock'} on:click={() => tab = 'unlock'}>Unlock</button>
      {/if}
    {/if}
  </div>

  <!-- Status tab -->
  {#if tab === 'status'}
    {#if identity}
      <div class="identity-card">
        <div class="field">
          <span class="label">Fingerprint</span>
          <code class="value mono">{identity.fingerprint}</code>
        </div>
        <div class="field">
          <span class="label">Public Key</span>
          <code class="value mono small">{identity.public_key_hex.slice(0, 32)}…</code>
        </div>
        <div class="status-badge online">● Identity active</div>
      </div>
    {:else}
      <p class="hint">No identity loaded. {hasStored ? 'Unlock your stored identity.' : 'Create a new identity to use peer transfers and mailbox messaging.'}</p>
    {/if}
  {/if}

  <!-- Create tab -->
  {#if tab === 'create'}
    <div class="form">
      <p class="hint">A 12-word recovery phrase is the cryptographic root of your identity. Store it securely — it cannot be recovered from the app.</p>

      <button class="btn secondary" on:click={generatePhrase} disabled={busy}>
        {busy ? 'Generating…' : '⚡ Generate Recovery Phrase'}
      </button>

      {#if generatedPhrase}
        <div class="phrase-box">
          <code>{generatedPhrase}</code>
          <button class="copy-btn" on:click={copyPhrase}>{copied ? '✓ Copied' : 'Copy'}</button>
        </div>
      {/if}

      <label class="field-row">
        <span>Phrase</span>
        <textarea bind:value={phrase} rows="2" placeholder="word1 word2 word3 …" />
      </label>

      <label class="field-row">
        <span>Passphrase (optional)</span>
        <input type="password" bind:value={passphrase} placeholder="Optional extra protection" />
      </label>

      <button class="btn primary" on:click={createIdentity} disabled={busy || !phrase}>
        {busy ? 'Creating…' : 'Create Identity'}
      </button>
    </div>
  {/if}

  <!-- Unlock tab -->
  {#if tab === 'unlock'}
    <div class="form">
      <p class="hint">Enter the passphrase you used when creating your identity.</p>
      <label class="field-row">
        <span>Passphrase</span>
        <input type="password" bind:value={passphrase} placeholder="Your passphrase" on:keydown={e => e.key === 'Enter' && unlockIdentity()} />
      </label>
      <button class="btn primary" on:click={unlockIdentity} disabled={busy || !passphrase}>
        {busy ? 'Unlocking…' : 'Unlock Identity'}
      </button>
    </div>
  {/if}
</div>

<style>
  .transfer-identity-panel { padding: 16px; max-width: 480px; }
  .panel-title { font-size: 1.1rem; margin-bottom: 12px; }
  .tabs { display: flex; gap: 4px; margin-bottom: 14px; }
  .tabs button { padding: 4px 12px; border-radius: 4px; border: 1px solid var(--border, #444); background: none; color: inherit; cursor: pointer; font-size: 0.85rem; }
  .tabs button.active { background: var(--accent, #7c3aed); color: #fff; border-color: transparent; }
  .msg { padding: 8px 12px; border-radius: 4px; margin-bottom: 10px; font-size: 0.85rem; }
  .msg.error { background: #3b0a0a; color: #f87171; }
  .msg.success { background: #052e16; color: #4ade80; }
  .identity-card { background: var(--surface, #1e1e2e); border-radius: 6px; padding: 14px; }
  .field { margin-bottom: 8px; }
  .field .label { font-size: 0.75rem; color: var(--text-muted, #888); display: block; margin-bottom: 2px; }
  .field .value { font-size: 0.8rem; word-break: break-all; }
  .mono { font-family: monospace; }
  .small { font-size: 0.72rem; }
  .status-badge { margin-top: 10px; font-size: 0.8rem; color: #4ade80; }
  .hint { font-size: 0.83rem; color: var(--text-muted, #888); margin-bottom: 12px; }
  .form { display: flex; flex-direction: column; gap: 10px; }
  .field-row { display: flex; flex-direction: column; gap: 4px; font-size: 0.85rem; }
  .field-row input, .field-row textarea { padding: 6px 8px; border-radius: 4px; border: 1px solid var(--border, #444); background: var(--input-bg, #12121a); color: inherit; font-size: 0.85rem; resize: vertical; }
  .phrase-box { background: var(--surface, #1e1e2e); border-radius: 4px; padding: 10px; display: flex; gap: 8px; align-items: flex-start; }
  .phrase-box code { flex: 1; font-size: 0.78rem; word-break: break-all; font-family: monospace; }
  .copy-btn { font-size: 0.75rem; padding: 3px 8px; border-radius: 3px; border: 1px solid var(--border, #444); background: none; color: inherit; cursor: pointer; white-space: nowrap; }
  .btn { padding: 7px 16px; border-radius: 5px; border: none; cursor: pointer; font-size: 0.85rem; transition: opacity 0.15s; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn.primary { background: var(--accent, #7c3aed); color: #fff; }
  .btn.secondary { background: var(--surface, #1e1e2e); border: 1px solid var(--border, #444); color: inherit; }
</style>
