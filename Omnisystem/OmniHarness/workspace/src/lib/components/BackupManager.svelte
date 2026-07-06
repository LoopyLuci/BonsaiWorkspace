<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let onClose: () => void = () => {};

  interface BackupEntry {
    id: string; filename: string; file_path: string;
    size_bytes: number; includes: string[]; checksum: string | null;
    encrypted: boolean; created_at: number; valid: boolean | null;
  }

  interface ImportSummary {
    profiles: number; avatars: number; sessions: number;
    errors: string[]; rollback_snapshot: string | null;
  }

  // Export state
  let exportIncludeSessions = true;
  let exportIncludeAvatars  = true;
  let exportEncrypt         = false;
  let exportPassphrase      = '';
  let exporting             = false;
  let exportResult          = '';
  let exportError           = '';

  // Import state
  let importPath     = '';
  let importMode     = 'Merge';
  let importProfileId = '';
  let importPassphrase = '';
  let importDryRun   = true;
  let importing      = false;
  let importSummary: ImportSummary | null = null;
  let importError    = '';

  // Backup list
  let backups: BackupEntry[] = [];
  let loadingList = false;
  let verifying: Record<string, boolean | null> = {};

  async function loadBackups() {
    loadingList = true;
    try { backups = await invoke<BackupEntry[]>('list_assistant_backups'); }
    catch (e) { console.error(e); }
    finally { loadingList = false; }
  }

  async function doExport() {
    exporting = true; exportResult = ''; exportError = '';
    try {
      const path = await invoke<string>('export_assistant_backup', {
        includeSessions: exportIncludeSessions,
        includeAvatars:  exportIncludeAvatars,
        encrypt:         exportEncrypt,
        passphrase:      exportEncrypt ? exportPassphrase : null,
      });
      exportResult = path;
      await loadBackups();
    } catch (e) { exportError = String(e); }
    finally { exporting = false; }
  }

  async function doImport(real: boolean) {
    if (!importPath) { importError = 'No file selected'; return; }
    importing = true; importSummary = null; importError = '';
    try {
      const modeArg = importMode === 'ReplaceProfile'
        ? { mode: 'ReplaceProfile', id: importProfileId }
        : { mode: importMode };

      const summary = await invoke<ImportSummary>('import_assistant_backup', {
        zipPath:    importPath,
        mode:       modeArg,
        passphrase: importPassphrase || null,
        dryRun:     !real,
      });
      importSummary = summary;
      if (real) await loadBackups();
    } catch (e) { importError = String(e); }
    finally { importing = false; }
  }

  async function pickImportFile() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const result = await open({ filters: [{ name: 'Backup', extensions: ['zip'] }] });
      if (typeof result === 'string') importPath = result;
    } catch {
      // Fallback to text input if dialog not available
    }
  }

  async function verify(b: BackupEntry) {
    verifying[b.id] = null;
    verifying = { ...verifying };
    try {
      const ok = await invoke<boolean>('verify_backup_integrity', {
        zipPath: b.file_path, passphrase: null,
      });
      verifying[b.id] = ok;
    } catch { verifying[b.id] = false; }
    verifying = { ...verifying };
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  }

  function fmtDate(ts: number): string {
    return new Date(ts * 1000).toLocaleString(undefined, {
      month: 'short', day: 'numeric', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }

  onMount(loadBackups);
</script>

<div class="bm">
  <div class="bm-header">
    <span>Backup & Restore</span>
    <button class="close-btn" on:click={onClose}>✕</button>
  </div>

  <!-- ── Export ── -->
  <section>
    <h3>Export Backup</h3>
    <div class="row-check">
      <label><input type="checkbox" bind:checked={exportIncludeSessions} /> Conversations</label>
      <label><input type="checkbox" bind:checked={exportIncludeAvatars}  /> Avatars</label>
      <label><input type="checkbox" bind:checked={exportEncrypt} /> Encrypt</label>
    </div>
    {#if exportEncrypt}
      <input class="pass-input" type="password" bind:value={exportPassphrase}
        placeholder="Passphrase" autocomplete="new-password" />
    {/if}
    <button class="primary-btn" on:click={doExport} disabled={exporting}>
      {exporting ? 'Exporting…' : '↓ Export'}
    </button>
    {#if exportResult}<div class="success">Saved: {exportResult}</div>{/if}
    {#if exportError}<div class="err">{exportError}</div>{/if}
  </section>

  <!-- ── Import ── -->
  <section>
    <h3>Import Backup</h3>
    <div class="file-row">
      <input class="path-input" type="text" bind:value={importPath}
        placeholder="Path to .zip backup…" readonly />
      <button class="secondary-btn" on:click={pickImportFile}>Browse</button>
    </div>

    <label class="mode-label">
      <span>Mode</span>
      <select bind:value={importMode}>
        <option value="Merge">Merge (add, rename on conflict)</option>
        <option value="FullReplace">Full Replace (auto-snapshot first)</option>
        <option value="ReplaceProfile">Replace Profile</option>
      </select>
    </label>

    {#if importMode === 'ReplaceProfile'}
      <input class="pass-input" type="text" bind:value={importProfileId}
        placeholder="Profile ID to replace" />
    {/if}

    <input class="pass-input" type="password" bind:value={importPassphrase}
      placeholder="Passphrase (if encrypted)" autocomplete="current-password" />

    <div class="import-btns">
      <button class="secondary-btn" on:click={() => doImport(false)} disabled={importing}>
        {importing && importDryRun ? 'Checking…' : '🔍 Dry Run'}
      </button>
      <button class="primary-btn" on:click={() => { importDryRun = false; doImport(true); }}
        disabled={importing || !importSummary || importSummary.errors.length > 0}>
        {importing && !importDryRun ? 'Importing…' : '↑ Apply Import'}
      </button>
    </div>

    {#if importSummary}
      <div class="summary" class:has-errors={importSummary.errors.length > 0}>
        <div>Profiles: {importSummary.profiles} · Avatars: {importSummary.avatars} · Sessions: {importSummary.sessions}</div>
        {#if importSummary.rollback_snapshot}
          <div class="dim">Rollback snapshot: {importSummary.rollback_snapshot}</div>
        {/if}
        {#if importSummary.errors.length > 0}
          <div class="err-list">
            {#each importSummary.errors as e}<div>✗ {e}</div>{/each}
          </div>
        {:else}
          <div class="ok">✓ All checks passed</div>
        {/if}
      </div>
    {/if}
    {#if importError}<div class="err">{importError}</div>{/if}
  </section>

  <!-- ── Backup List ── -->
  <section class="list-section">
    <div class="list-header">
      <h3>Saved Backups</h3>
      <button class="refresh-btn" on:click={loadBackups}>↻</button>
    </div>

    {#if loadingList}
      <div class="dim notice">Loading…</div>
    {:else if backups.length === 0}
      <div class="dim notice">No backups yet</div>
    {:else}
      <div class="backup-list">
        {#each backups as b (b.id)}
          <div class="backup-row">
            <div class="b-info">
              <span class="b-name">{b.filename}</span>
              <span class="b-meta">
                {fmtDate(b.created_at)} · {fmtSize(b.size_bytes)}
                {#if b.encrypted}<span class="tag enc">🔒</span>{/if}
                {#if !b.valid}<span class="tag missing">file missing</span>{/if}
              </span>
              <span class="b-includes">{b.includes.join(' · ')}</span>
            </div>
            <div class="b-actions">
              <button class="verify-btn"
                on:click={() => verify(b)}
                title="Verify checksums"
              >
                {#if verifying[b.id] === undefined}✓{:else if verifying[b.id] === null}…{:else if verifying[b.id]}✓ OK{:else}✗ Fail{/if}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .bm { display: flex; flex-direction: column; height: 100%; background: var(--bg); color: var(--fg); font-size: 0.82rem; overflow-y: auto; }
  .bm-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-bottom: 1px solid var(--border); font-weight: 600; flex-shrink: 0;
  }
  .close-btn { background: none; border: none; color: var(--fg-dim); cursor: pointer; font-size: 1rem; }

  section {
    padding: 10px 12px; border-bottom: 1px solid var(--border);
    display: flex; flex-direction: column; gap: 8px;
  }
  h3 { margin: 0; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--fg-dim); }
  .list-section { flex: 1; }

  .row-check { display: flex; gap: 14px; flex-wrap: wrap; }
  .row-check label { display: flex; align-items: center; gap: 5px; cursor: pointer; }

  .pass-input, .path-input {
    background: var(--bg2); border: 1px solid var(--border); border-radius: 6px;
    color: var(--fg); padding: 6px 8px; font-size: 0.82rem; width: 100%; box-sizing: border-box;
  }
  .file-row { display: flex; gap: 6px; }
  .path-input { flex: 1; }

  .mode-label { display: flex; align-items: center; gap: 8px; }
  .mode-label span { color: var(--fg-dim); white-space: nowrap; font-size: 0.78rem; }
  select {
    flex: 1; background: var(--bg2); border: 1px solid var(--border); border-radius: 6px;
    color: var(--fg); padding: 5px 8px; font-size: 0.82rem;
  }

  .import-btns { display: flex; gap: 8px; }
  .primary-btn {
    flex: 1; background: var(--accent); border: none; color: #fff;
    padding: 6px 14px; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 0.82rem;
  }
  .primary-btn:disabled { opacity: 0.5; cursor: default; }
  .primary-btn:not(:disabled):hover { background: var(--accent-hover, #4a93d9); }
  .secondary-btn {
    background: var(--bg2); border: 1px solid var(--border); color: var(--fg);
    padding: 6px 12px; border-radius: 6px; cursor: pointer; font-size: 0.82rem;
  }
  .secondary-btn:hover { border-color: var(--accent); }

  .summary {
    background: var(--bg2); border-radius: 6px; padding: 8px 10px;
    display: flex; flex-direction: column; gap: 4px; border: 1px solid var(--border);
  }
  .summary.has-errors { border-color: var(--danger); }
  .err-list { color: var(--danger); }
  .ok { color: #4ec94e; }
  .success { color: #4ec94e; font-size: 0.78rem; word-break: break-all; }
  .err { color: var(--danger); font-size: 0.78rem; }

  .list-header { display: flex; justify-content: space-between; align-items: center; }
  .refresh-btn { background: none; border: none; color: var(--fg-dim); cursor: pointer; font-size: 1rem; }
  .notice { padding: 12px 0; text-align: center; }
  .dim { color: var(--fg-dim); }

  .backup-list { display: flex; flex-direction: column; gap: 0; }
  .backup-row {
    display: flex; align-items: center; gap: 8px; padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  .b-info { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .b-name { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .b-meta { font-size: 0.72rem; color: var(--fg-dim); display: flex; align-items: center; gap: 6px; }
  .b-includes { font-size: 0.7rem; color: var(--fg-dim); }
  .tag { font-size: 0.68rem; border-radius: 4px; padding: 1px 5px; background: var(--border); }
  .enc { color: #ecc94b; }
  .missing { color: var(--danger); }
  .b-actions { flex-shrink: 0; }
  .verify-btn {
    background: var(--bg2); border: 1px solid var(--border); border-radius: 4px;
    color: var(--fg-dim); cursor: pointer; font-size: 0.75rem; padding: 3px 8px;
  }
  .verify-btn:hover { border-color: var(--accent); color: var(--fg); }
</style>
