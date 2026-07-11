<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { addToast } from '$lib/stores/toast';

  // ── Types ─────────────────────────────────────────────────────────────────

  interface FindingSummary {
    critical: number;
    high: number;
    medium: number;
    low: number;
  }

  interface InstalledExtension {
    extension_id: string;
    name: string;
    version: string;
    description: string;
    author_name: string;
    repository: string;
    category: string;
    status: string;
    verdict: string;
    risk_score: number;
    installed_at: string;
    update_available: boolean;
    latest_version: string | null;
    config: Record<string, unknown>;
    config_schema: Record<string, unknown>;
    install_path: string;
    has_source: boolean;
    finding_summary: FindingSummary;
  }

  interface ExtensionCard {
    extension_id: string;
    name: string;
    description: string;
    author_name: string;
    repository: string;
    category: string;
    tags: string[];
    version: string;
    verdict: string;
    risk_score: number;
    install_count: number;
    rating: number;
    icon: string | null;
    is_installed: boolean;
  }

  // ── State ─────────────────────────────────────────────────────────────────

  let activeTab: 'browse' | 'installed' | 'import' | 'submit' = 'browse';
  function setTab(t: string) { activeTab = t as typeof activeTab; }
  function fieldSchema(f: unknown): { type?: string; description?: string; default?: unknown } {
    return (f as { type?: string; description?: string; default?: unknown }) ?? {};
  }
  function inputValue(e: Event): string {
    return (e.target as HTMLInputElement)?.value ?? '';
  }
  let browseCards: ExtensionCard[] = [];
  let installedExts: InstalledExtension[] = [];
  let selectedExt: InstalledExtension | null = null;
  let loading = false;
  let error = '';

  // Browse
  let searchQuery = '';
  let selectedCategory = 'All';

  // Install
  let githubUrl = '';
  let localPath = '';
  let installStatus = '';
  let installing = false;

  // Import from other IDE
  let importSource: 'vscode' | 'jetbrains' = 'vscode';
  let importUrl = '';
  let importStatus = '';
  let importing = false;

  // Submit
  let submitUrl = '';
  let submitStatus = '';

  const categories = ['All', 'Tool', 'UI', 'Theme', 'Language', 'Agent', 'Data', 'Security', 'Training', 'DevOps', 'Analytics', 'Utility'];

  // ── Computed ──────────────────────────────────────────────────────────────

  $: filteredCards = browseCards.filter(c => {
    const matchesSearch = !searchQuery ||
      c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.tags.some(t => t.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesCat = selectedCategory === 'All' || c.category === selectedCategory;
    return matchesSearch && matchesCat;
  });

  // ── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(async () => {
    await loadAll();
  });

  async function loadAll() {
    loading = true;
    error = '';
    try {
      [browseCards, installedExts] = await Promise.all([
        invoke<ExtensionCard[]>('ext_list_all'),
        invoke<InstalledExtension[]>('ext_list_installed'),
      ]);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // ── Actions ───────────────────────────────────────────────────────────────

  async function installFromGithub() {
    if (!githubUrl.trim()) return;
    installing = true;
    installStatus = 'Installing…';
    try {
      const result = await invoke<{ extension_id: string; verdict_label: string; risk_score: number; finding_count: number }>(
        'ext_install_from_github', { githubUrl: githubUrl.trim() }
      );
      installStatus = `Installed ${result.extension_id} — verdict: ${result.verdict_label}, risk: ${result.risk_score}, findings: ${result.finding_count}`;
      githubUrl = '';
      await loadAll();
    } catch (e) {
      installStatus = `Error: ${e}`;
    } finally {
      installing = false;
    }
  }

  async function uninstall(id: string) {
    try {
      await invoke('ext_uninstall', { extensionId: id });
      selectedExt = null;
      await loadAll();
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleEnabled(ext: InstalledExtension) {
    const newEnabled = ext.status === 'Disabled';
    try {
      await invoke('ext_set_enabled', { extensionId: ext.extension_id, enabled: newEnabled });
      await loadAll();
      if (selectedExt?.extension_id === ext.extension_id) {
        selectedExt = installedExts.find(e => e.extension_id === ext.extension_id) ?? null;
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function rescan(id: string) {
    try {
      await invoke('ext_rescan', { extensionId: id });
      await loadAll();
    } catch (e) {
      error = String(e);
    }
  }

  async function submitGithub() {
    if (!submitUrl.trim()) return;
    submitStatus = 'Submitting…';
    try {
      await invoke('ext_install_from_github', { githubUrl: submitUrl.trim() });
      submitStatus = 'Extension submitted and indexed successfully.';
      submitUrl = '';
      await loadAll();
    } catch (e) {
      submitStatus = `Error: ${e}`;
    }
  }

  async function startImport() {
    if (!importUrl.trim()) return;
    importing = true;
    importStatus = 'Parsing extension package…';
    try {
      // Phase 1: preview scan first
      const report = await invoke<{ risk_score: number; verdict: string; files_scanned: number; summary: string; user_message: string }>(
        'ext_preview_scan', { githubUrl: importUrl.trim() }
      );
      importStatus = `Scanned: ${report.summary} — Verdict: ${report.verdict}. ${report.user_message}`;
    } catch (e) {
      importStatus = `Import preview error: ${e}`;
    } finally {
      importing = false;
    }
  }

  // ── Helpers ───────────────────────────────────────────────────────────────

  function verdictColor(verdict: string): string {
    switch (verdict) {
      case 'Safe': return '#4caf50';
      case 'Caution': return '#ff9800';
      case 'Risky': case 'Blocked': return '#f44336';
      default: return '#888';
    }
  }

  function verdictIcon(verdict: string): string {
    switch (verdict) {
      case 'Safe': return '✅';
      case 'Caution': return '⚠️';
      case 'Risky': return '🔴';
      case 'Blocked': return '🚫';
      default: return '⬜';
    }
  }

  function riskBadge(score: number): string {
    if (score === 0) return 'risk-none';
    if (score < 20) return 'risk-low';
    if (score < 50) return 'risk-medium';
    if (score < 80) return 'risk-high';
    return 'risk-critical';
  }
</script>

<!-- ── Shell ─────────────────────────────────────────────────────────────── -->
<div class="extensions-panel">
  <div class="panel-header">
    <h2>🧩 Extensions</h2>
    <div class="tab-bar">
      {#each [['browse','Browse'], ['installed','Installed'], ['import','Import'], ['submit','Submit']] as [tab, label]}
        <button
          class="tab-btn"
          class:active={activeTab === tab}
          on:click={() => setTab(tab)}
        >{label}{tab === 'installed' ? ` (${installedExts.length})` : ''}</button>
      {/each}
    </div>
  </div>

  {#if error}
    <div class="error-bar">⚠ {error} <button on:click={() => error = ''}>✕</button></div>
  {/if}

  <!-- ── Browse tab ──────────────────────────────────────────────────────── -->
  {#if activeTab === 'browse'}
    <div class="browse-controls">
      <input class="search-input" placeholder="Search extensions…" bind:value={searchQuery} />
      <select bind:value={selectedCategory}>
        {#each categories as cat}<option>{cat}</option>{/each}
      </select>
      <button class="btn-sm" on:click={loadAll}>↺ Refresh</button>
    </div>

    {#if loading}
      <div class="loading">Loading extensions…</div>
    {:else if filteredCards.length === 0}
      <div class="empty-state">No extensions found. Install one using the GitHub URL below or submit your own.</div>
    {:else}
      <div class="card-grid">
        {#each filteredCards as card (card.extension_id)}
          <div class="ext-card">
            <div class="card-header">
              {#if card.icon}
                <img class="card-icon" src={card.icon} alt={card.name} />
              {:else}
                <div class="card-icon-placeholder">🧩</div>
              {/if}
              <div class="card-title-block">
                <div class="card-name">{card.name}</div>
                <div class="card-meta">v{card.version} · {card.author_name}</div>
              </div>
              <span class="verdict-badge" style="color: {verdictColor(card.verdict)}">
                {verdictIcon(card.verdict)} {card.verdict}
              </span>
            </div>
            <div class="card-desc">{card.description}</div>
            <div class="card-tags">
              <span class="tag cat-tag">{card.category}</span>
              {#each card.tags.slice(0, 3) as tag}
                <span class="tag">{tag}</span>
              {/each}
            </div>
            <div class="card-footer">
              <span class="stat">⬇ {card.install_count}</span>
              <span class="stat">⭐ {card.rating.toFixed(1)}</span>
              <span class="stat {riskBadge(card.risk_score)}">Risk {card.risk_score}</span>
              {#if card.is_installed}
                <span class="installed-badge">Installed</span>
              {:else}
                <button class="btn-install" on:click={() => { githubUrl = card.repository; activeTab = 'browse'; installFromGithub(); }}>
                  Install
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Quick install bar at bottom of browse -->
    <div class="quick-install">
      <input
        class="search-input flex1"
        placeholder="GitHub URL (https://github.com/user/repo)"
        bind:value={githubUrl}
        on:keydown={e => e.key === 'Enter' && installFromGithub()}
      />
      <button class="btn-primary" disabled={installing} on:click={installFromGithub}>
        {installing ? 'Installing…' : 'Install from GitHub'}
      </button>
    </div>
    {#if installStatus}
      <div class="status-bar">{installStatus}</div>
    {/if}

  <!-- ── Installed tab ────────────────────────────────────────────────────── -->
  {:else if activeTab === 'installed'}
    {#if installedExts.length === 0}
      <div class="empty-state">No extensions installed yet.</div>
    {:else}
      <div class="installed-layout">
        <!-- List -->
        <div class="installed-list">
          {#each installedExts as ext (ext.extension_id)}
            <div
              class="installed-row"
              class:selected={selectedExt?.extension_id === ext.extension_id}
              role="button"
              tabindex="0"
              on:click={() => selectedExt = ext}
              on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') selectedExt = ext; }}
            >
              <span class="verdict-dot" style="background:{verdictColor(ext.verdict)}"></span>
              <div class="row-info">
                <div class="row-name">{ext.name}</div>
                <div class="row-meta">v{ext.version} · {ext.category}</div>
              </div>
              <span class="status-chip" class:enabled={ext.status === 'Enabled'}>
                {ext.status}
              </span>
            </div>
          {/each}
        </div>

        <!-- Detail -->
        {#if selectedExt}
          {@const ext = selectedExt}
          <div class="ext-detail">
            <div class="detail-header">
              <h3>{ext.name} <span class="detail-version">v{ext.version}</span></h3>
              <div class="detail-actions">
                <button class="btn-sm" on:click={() => toggleEnabled(ext)}>
                  {ext.status === 'Enabled' ? 'Disable' : 'Enable'}
                </button>
                <button class="btn-sm" on:click={() => rescan(ext.extension_id)}>Rescan</button>
                <button class="btn-sm btn-danger" on:click={() => uninstall(ext.extension_id)}>Uninstall</button>
              </div>
            </div>

            <p class="detail-desc">{ext.description}</p>

            <div class="detail-grid">
              <div class="detail-item"><span class="lbl">Author</span><span>{ext.author_name}</span></div>
              <div class="detail-item"><span class="lbl">Category</span><span>{ext.category}</span></div>
              <div class="detail-item"><span class="lbl">Installed</span><span>{new Date(ext.installed_at).toLocaleDateString()}</span></div>
              <div class="detail-item"><span class="lbl">Status</span><span>{ext.status}</span></div>
            </div>

            <!-- Security summary -->
            <div class="security-section">
              <h4>Security {verdictIcon(ext.verdict)} <span style="color:{verdictColor(ext.verdict)}">{ext.verdict}</span>
                <span class="risk-score {riskBadge(ext.risk_score)}">Risk {ext.risk_score}/100</span>
              </h4>
              <div class="finding-row">
                {#if ext.finding_summary.critical > 0}
                  <span class="finding-chip critical">{ext.finding_summary.critical} Critical</span>
                {/if}
                {#if ext.finding_summary.high > 0}
                  <span class="finding-chip high">{ext.finding_summary.high} High</span>
                {/if}
                {#if ext.finding_summary.medium > 0}
                  <span class="finding-chip medium">{ext.finding_summary.medium} Medium</span>
                {/if}
                {#if ext.finding_summary.low > 0}
                  <span class="finding-chip low">{ext.finding_summary.low} Low</span>
                {/if}
                {#if ext.finding_summary.critical + ext.finding_summary.high + ext.finding_summary.medium + ext.finding_summary.low === 0}
                  <span class="finding-chip ok">No findings</span>
                {/if}
              </div>
            </div>

            <!-- Config -->
            {#if Object.keys(ext.config_schema).length > 0}
              <div class="config-section">
                <h4>Configuration</h4>
                {#each Object.entries(ext.config_schema) as [key, field]}
                  {@const schema = fieldSchema(field)}
                  <div class="config-row">
                    <label class="config-label">
                      {key}
                      {#if schema.description}<span class="config-hint">{schema.description}</span>{/if}
                    </label>
                    <input
                      class="config-input"
                      value={ext.config[key] ?? schema.default ?? ''}
                      on:change={async (e) => {
                        try {
                          await invoke('ext_set_config', {
                            extensionId: ext.extension_id,
                            key,
                            value: inputValue(e)
                          });
                        } catch (err) {
                          addToast(typeof err === 'string' ? err : `Failed to update ${key}`, 'error');
                        }
                      }}
                    />
                  </div>
                {/each}
                <button class="btn-sm" on:click={() => invoke('ext_reset_config', { extensionId: ext.extension_id })
                  .catch((err) => addToast(typeof err === 'string' ? err : 'Failed to reset extension config', 'error'))}>
                  Reset to Defaults
                </button>
              </div>
            {/if}
          </div>
        {:else}
          <div class="detail-placeholder">Select an extension to view details.</div>
        {/if}
      </div>
    {/if}

  <!-- ── Import tab ───────────────────────────────────────────────────────── -->
  {:else if activeTab === 'import'}
    <div class="import-panel">
      <h3>Import from Another IDE</h3>
      <p class="import-desc">
        Convert extensions from other IDEs into Workspace extensions.
        Phase 1 supports VSCode (<code>.vsix</code>) and GitHub-hosted packages.
      </p>

      <div class="import-source-row">
        <label>Source Format:</label>
        <select bind:value={importSource}>
          <option value="vscode">VSCode (.vsix)</option>
          <option value="jetbrains">JetBrains (coming soon)</option>
        </select>
      </div>

      <div class="import-input-row">
        <input
          class="search-input flex1"
          placeholder={importSource === 'vscode'
            ? 'GitHub URL or VSCode Marketplace ID (e.g. ms-python.python)'
            : 'GitHub URL'}
          bind:value={importUrl}
        />
        <button class="btn-primary" disabled={importing} on:click={startImport}>
          {importing ? 'Analysing…' : 'Analyse & Preview'}
        </button>
      </div>

      {#if importStatus}
        <div class="import-report">{importStatus}</div>
      {/if}

      <div class="conversion-info">
        <h4>Conversion Tiers</h4>
        <div class="tier-list">
          <div class="tier">
            <span class="tier-badge t1">Tier 1</span>
            <div>
              <strong>Manifest-only</strong> — Themes, snippets, keybindings.
              No code translation required. Fast and automatic.
            </div>
          </div>
          <div class="tier">
            <span class="tier-badge t2">Tier 2</span>
            <div>
              <strong>Shim-based</strong> — Extensions using standard VSCode APIs.
              Compiled against the <code>workspace-vscode-shim</code> library.
            </div>
          </div>
          <div class="tier">
            <span class="tier-badge t3">Tier 3</span>
            <div>
              <strong>AI-assisted</strong> — Complex extensions with custom UIs or
              non-standard APIs. The Conversion Agent rewrites them for Workspace.
            </div>
          </div>
        </div>
      </div>
    </div>

  <!-- ── Submit tab ───────────────────────────────────────────────────────── -->
  {:else if activeTab === 'submit'}
    <div class="submit-panel">
      <h3>Submit Your Extension</h3>
      <p class="import-desc">
        Share your Workspace extension with the community. Your repository will be
        indexed, security-scanned, and appear in the Browse tab.
      </p>

      <div class="import-input-row">
        <input
          class="search-input flex1"
          placeholder="GitHub repository URL (https://github.com/you/your-extension)"
          bind:value={submitUrl}
          on:keydown={e => e.key === 'Enter' && submitGithub()}
        />
        <button class="btn-primary" on:click={submitGithub}>Submit</button>
      </div>

      {#if submitStatus}
        <div class="import-report">{submitStatus}</div>
      {/if}

      <div class="submit-requirements">
        <h4>Requirements</h4>
        <ul>
          <li>Repository must contain a <code>workspace-extension.yaml</code> at the root.</li>
          <li>Extension must pass the security scan (verdict: Safe or Caution).</li>
          <li>Extension ID must be globally unique (format: <code>author.name</code>).</li>
          <li>A <code>README.md</code> describing what the extension does is strongly recommended.</li>
        </ul>

        <h4>Extension Manifest Template</h4>
        <pre class="code-block">extension_id: your-name.extension-name
name: "My Extension"
version: "1.0.0"
description: "What it does"
author:
  name: "Your Name"
  email: "you@example.com"
repository: "https://github.com/you/extension"
category: Tool
tags: [productivity, ai]
entry_points:
  main: "src/main.wasm"
permissions:
  network_access: none
  file_access: read_only</pre>
      </div>
    </div>
  {/if}
</div>

<style>
  .extensions-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1a1a2e;
    color: #e0e0e0;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 13px;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem 0;
    border-bottom: 1px solid #2a2a4a;
    flex-shrink: 0;
  }

  .panel-header h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: #c9b8ff;
    flex-shrink: 0;
  }

  .tab-bar {
    display: flex;
    gap: 2px;
  }

  .tab-btn {
    background: none;
    border: none;
    color: #888;
    padding: 0.5rem 0.9rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    font-size: 12px;
    transition: color 0.15s;
  }
  .tab-btn:hover { color: #ccc; }
  .tab-btn.active { color: #c9b8ff; border-bottom-color: #c9b8ff; }

  .error-bar {
    background: #5a1a1a;
    color: #ffaaaa;
    padding: 0.4rem 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    flex-shrink: 0;
  }
  .error-bar button { background: none; border: none; color: inherit; cursor: pointer; }

  .browse-controls {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    flex-shrink: 0;
    border-bottom: 1px solid #2a2a4a;
  }

  .search-input {
    background: #12122a;
    border: 1px solid #2a2a4a;
    color: #e0e0e0;
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    font-size: 13px;
    flex: 1;
  }
  .search-input:focus { outline: none; border-color: #7c5cbf; }
  .flex1 { flex: 1; }

  select {
    background: #12122a;
    border: 1px solid #2a2a4a;
    color: #e0e0e0;
    padding: 0.4rem 0.6rem;
    border-radius: 6px;
    font-size: 12px;
  }

  .btn-sm {
    background: #2a2a4a;
    border: 1px solid #3a3a6a;
    color: #ccc;
    padding: 0.35rem 0.75rem;
    border-radius: 5px;
    cursor: pointer;
    font-size: 12px;
  }
  .btn-sm:hover { background: #3a3a6a; color: #fff; }
  .btn-danger { border-color: #7a2222; color: #ffaaaa; }
  .btn-danger:hover { background: #5a1a1a; }

  .btn-primary {
    background: #5a4abf;
    border: none;
    color: #fff;
    padding: 0.4rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
  }
  .btn-primary:hover:not(:disabled) { background: #7c5cbf; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-install {
    background: #1e4a1e;
    border: 1px solid #2e6a2e;
    color: #7dff7d;
    padding: 0.25rem 0.6rem;
    border-radius: 5px;
    cursor: pointer;
    font-size: 11px;
  }
  .btn-install:hover { background: #2e6a2e; }

  .loading, .empty-state, .detail-placeholder {
    padding: 2rem;
    text-align: center;
    color: #888;
    flex: 1;
  }

  /* ── Browse grid ── */
  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.75rem;
    padding: 1rem;
    overflow-y: auto;
    flex: 1;
  }

  .ext-card {
    background: #12122a;
    border: 1px solid #2a2a4a;
    border-radius: 8px;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    transition: border-color 0.15s;
  }
  .ext-card:hover { border-color: #5a4abf; }

  .card-header {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
  }

  .card-icon { width: 32px; height: 32px; border-radius: 4px; object-fit: cover; flex-shrink: 0; }
  .card-icon-placeholder { width: 32px; height: 32px; border-radius: 4px; background: #2a2a4a; display: flex; align-items: center; justify-content: center; font-size: 16px; flex-shrink: 0; }

  .card-title-block { flex: 1; min-width: 0; }
  .card-name { font-weight: 600; font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card-meta { font-size: 11px; color: #888; }

  .verdict-badge { font-size: 11px; font-weight: 600; white-space: nowrap; flex-shrink: 0; }

  .card-desc { font-size: 12px; color: #aaa; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }

  .card-tags { display: flex; flex-wrap: wrap; gap: 4px; }
  .tag { background: #1e1e3e; border: 1px solid #3a3a6a; padding: 1px 6px; border-radius: 10px; font-size: 10px; color: #aaa; }
  .cat-tag { background: #1a1a4a; color: #9b8aff; border-color: #4a4a9a; }

  .card-footer {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: auto;
    flex-wrap: wrap;
  }
  .stat { font-size: 11px; color: #777; }
  .installed-badge { background: #1e4a1e; color: #7dff7d; padding: 1px 8px; border-radius: 10px; font-size: 11px; margin-left: auto; }

  /* Risk classes */
  .risk-none { color: #7dff7d; }
  .risk-low { color: #aaddff; }
  .risk-medium { color: #ffdd77; }
  .risk-high { color: #ff9944; }
  .risk-critical { color: #ff5555; }

  /* Quick install bar */
  .quick-install {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-top: 1px solid #2a2a4a;
    flex-shrink: 0;
  }
  .status-bar { padding: 0.3rem 1rem; font-size: 12px; color: #aaa; background: #0e0e22; border-top: 1px solid #2a2a4a; flex-shrink: 0; }

  /* ── Installed tab ── */
  .installed-layout {
    display: grid;
    grid-template-columns: 260px 1fr;
    flex: 1;
    overflow: hidden;
  }

  .installed-list {
    border-right: 1px solid #2a2a4a;
    overflow-y: auto;
  }

  .installed-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 0.85rem;
    cursor: pointer;
    border-bottom: 1px solid #1e1e3a;
    transition: background 0.1s;
  }
  .installed-row:hover { background: #1e1e3a; }
  .installed-row.selected { background: #1a1a4a; border-left: 2px solid #7c5cbf; }

  .verdict-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .row-info { flex: 1; min-width: 0; }
  .row-name { font-size: 13px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row-meta { font-size: 11px; color: #777; }

  .status-chip {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 8px;
    background: #2a1a1a;
    color: #ff8888;
    border: 1px solid #4a2222;
    flex-shrink: 0;
  }
  .status-chip.enabled { background: #1a2a1a; color: #7dff7d; border-color: #224422; }

  /* Detail panel */
  .ext-detail {
    padding: 1.25rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .detail-header h3 { margin: 0; font-size: 16px; }
  .detail-version { font-size: 12px; color: #888; font-weight: 400; }
  .detail-actions { display: flex; gap: 0.4rem; flex-wrap: wrap; }
  .detail-desc { margin: 0; color: #aaa; font-size: 13px; line-height: 1.5; }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.4rem;
  }
  .detail-item { display: flex; gap: 0.5rem; font-size: 12px; }
  .lbl { color: #777; min-width: 70px; }

  /* Security section */
  .security-section { background: #0e0e22; border: 1px solid #2a2a4a; border-radius: 6px; padding: 0.85rem; }
  .security-section h4 { margin: 0 0 0.5rem; font-size: 13px; }
  .risk-score { font-size: 11px; margin-left: 0.5rem; padding: 1px 8px; border-radius: 10px; background: #1e1e3e; }
  .finding-row { display: flex; flex-wrap: wrap; gap: 6px; }
  .finding-chip { font-size: 11px; padding: 2px 8px; border-radius: 10px; }
  .finding-chip.critical { background: #5a1a1a; color: #ff8888; }
  .finding-chip.high { background: #4a2a1a; color: #ffaa66; }
  .finding-chip.medium { background: #3a3a1a; color: #ffdd77; }
  .finding-chip.low { background: #1a2a3a; color: #88aaff; }
  .finding-chip.ok { background: #1a2a1a; color: #7dff7d; }

  /* Config section */
  .config-section { background: #0e0e22; border: 1px solid #2a2a4a; border-radius: 6px; padding: 0.85rem; display: flex; flex-direction: column; gap: 0.6rem; }
  .config-section h4 { margin: 0 0 0.4rem; font-size: 13px; }
  .config-row { display: flex; flex-direction: column; gap: 2px; }
  .config-label { font-size: 12px; color: #aaa; }
  .config-hint { font-size: 11px; color: #666; margin-left: 0.5rem; }
  .config-input {
    background: #12122a;
    border: 1px solid #2a2a4a;
    color: #e0e0e0;
    padding: 0.3rem 0.6rem;
    border-radius: 5px;
    font-size: 12px;
    width: 100%;
    box-sizing: border-box;
  }

  /* ── Import tab ── */
  .import-panel, .submit-panel {
    padding: 1.25rem;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .import-panel h3, .submit-panel h3 { margin: 0; font-size: 15px; color: #c9b8ff; }
  .import-desc { margin: 0; color: #aaa; font-size: 13px; line-height: 1.5; }

  .import-source-row, .import-input-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .import-source-row label { color: #aaa; font-size: 13px; white-space: nowrap; }

  .import-report {
    background: #0e0e22;
    border: 1px solid #2a2a4a;
    border-radius: 6px;
    padding: 0.85rem;
    font-size: 12px;
    color: #ccc;
    white-space: pre-wrap;
  }

  .conversion-info { background: #0e0e22; border: 1px solid #2a2a4a; border-radius: 6px; padding: 1rem; }
  .conversion-info h4 { margin: 0 0 0.75rem; font-size: 13px; color: #c9b8ff; }
  .tier-list { display: flex; flex-direction: column; gap: 0.75rem; }
  .tier { display: flex; align-items: flex-start; gap: 0.75rem; font-size: 12px; color: #aaa; line-height: 1.5; }
  .tier-badge { flex-shrink: 0; font-size: 10px; font-weight: 700; padding: 2px 8px; border-radius: 8px; }
  .t1 { background: #1a3a1a; color: #7dff7d; }
  .t2 { background: #1a2a3a; color: #88aaff; }
  .t3 { background: #3a2a1a; color: #ffaa66; }

  /* ── Submit tab ── */
  .submit-requirements { background: #0e0e22; border: 1px solid #2a2a4a; border-radius: 6px; padding: 1rem; }
  .submit-requirements h4 { margin: 0 0 0.5rem; font-size: 13px; color: #c9b8ff; }
  .submit-requirements ul { margin: 0 0 1rem; padding-left: 1.2rem; color: #aaa; font-size: 12px; line-height: 1.8; }
  .code-block {
    background: #12122a;
    border: 1px solid #2a2a4a;
    border-radius: 5px;
    padding: 0.85rem;
    font-family: 'Fira Code', monospace;
    font-size: 11px;
    color: #ccc;
    overflow-x: auto;
    white-space: pre;
    margin: 0;
  }
  code { background: #1e1e3e; padding: 1px 5px; border-radius: 3px; font-family: monospace; font-size: 11px; color: #c9b8ff; }
</style>
