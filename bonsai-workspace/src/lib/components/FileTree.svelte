<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { currentWorkspace, fileTreeRefresh, setWorkspace } from '$lib/stores/workspace';
  import { requestOpenFile } from '$lib/stores/openFile';

  interface FileEntry {
    path:   string;
    rel:    string;
    name:   string;
    is_dir: boolean;
  }

  let allFiles:     FileEntry[] = [];
  let searchTerm    = '';
  let loading       = false;
  let error         = '';
  let expandedDirs  = new Set<string>();
  let selectedPath  = '';

  // ── Load files whenever workspace or refresh signal changes ──────────────
  $: $currentWorkspace, $fileTreeRefresh, loadFiles();

  async function loadFiles() {
    if (!$currentWorkspace) { allFiles = []; return; }
    loading = true; error = '';
    try {
      const raw = await invoke<FileEntry[]>('list_project_files', {
        workspacePath: $currentWorkspace.path,
      });
      // Sort in tree order: parent always before children, dirs before files
      // at the same level. Sort key: treat each dir as "rel/" so it naturally
      // precedes its children in a simple string comparison.
      allFiles = raw.sort((a, b) => {
        const aKey = a.rel.toLowerCase() + (a.is_dir ? '/' : '');
        const bKey = b.rel.toLowerCase() + (b.is_dir ? '/' : '');
        return aKey < bKey ? -1 : aKey > bKey ? 1 : 0;
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function openWorkspace() {
    try {
      const selected = await invoke<string>('open_workspace');
      if (!selected) return;
      let branch = 'main';
      try { branch = await invoke<string>('get_git_branch', { workspacePath: selected }); } catch { /**/ }
      setWorkspace(selected, branch);
      expandedDirs = new Set();
      selectedPath = '';
    } catch (e) { error = String(e); }
  }

  function refreshFiles() { fileTreeRefresh.set(Date.now()); }

  function depth(rel: string) { return (rel.match(/\//g) ?? []).length; }

  // ── Visibility ──────────────────────────────────────────────────────────
  // Without search: show only items whose every parent directory is expanded.
  // With search:    show items (files or dirs) whose rel path contains the term,
  //                 plus all ancestor directories so the path is reachable.
  function computeDisplayed(files: FileEntry[], search: string, expanded: Set<string>): FileEntry[] {
    const needle = search.trim().toLowerCase();

    if (!needle) {
      // Hierarchical mode: item visible iff all ancestor dirs are expanded
      return files.filter(f => {
        const parts = f.rel.split('/');
        for (let i = 1; i < parts.length; i++) {
          if (!expanded.has(parts.slice(0, i).join('/'))) return false;
        }
        return true;
      });
    }

    // Search mode: collect matching entries and their ancestor dirs
    const visible = new Set<string>();
    for (const f of files) {
      if (f.rel.toLowerCase().includes(needle)) {
        visible.add(f.rel);
        // Add every ancestor directory
        const parts = f.rel.split('/');
        for (let i = 1; i < parts.length; i++) {
          visible.add(parts.slice(0, i).join('/'));
        }
      }
    }
    return files.filter(f => visible.has(f.rel));
  }

  $: displayed = computeDisplayed(allFiles, searchTerm, expandedDirs);

  // ── Interaction ─────────────────────────────────────────────────────────
  function handleClick(file: FileEntry) {
    selectedPath = file.path;
    if (file.is_dir) {
      if (expandedDirs.has(file.rel)) {
        // Collapse: also collapse all children
        expandedDirs = new Set([...expandedDirs].filter(d => !d.startsWith(file.rel)));
      } else {
        expandedDirs = new Set([...expandedDirs, file.rel]);
      }
    } else {
      requestOpenFile(file.path);
    }
  }

  // ── Icons ───────────────────────────────────────────────────────────────
  const EXT_ICON: Record<string, string> = {
    rs: '🦀', ts: '🔷', tsx: '🔷', js: '🟨', jsx: '🟨',
    py: '🐍', md: '📝', json: '📋', toml: '⚙', yaml: '⚙', yml: '⚙',
    html: '🌐', css: '🎨', sh: '💲', sql: '🗄', go: '🐹',
    cpp: '⚡', c: '⚡', cs: '🔵', rb: '💎', kt: '🎯', swift: '🍎',
    lock: '🔒', env: '🔑', gitignore: '🚫',
  };

  function fileIcon(name: string): string {
    const ext = name.split('.').pop()?.toLowerCase() ?? '';
    if (name.startsWith('.')) return EXT_ICON[name.slice(1)] ?? '🔧';
    return EXT_ICON[ext] ?? '📄';
  }
</script>

<div class="filetree">
  <!-- Header -->
  <div class="header">
    {#if $currentWorkspace}
      <div class="ws-row">
        <span class="ws-name" title={$currentWorkspace.path}>
          🌿 {$currentWorkspace.name}
        </span>
        <span class="ws-branch">{$currentWorkspace.branch}</span>
      </div>
    {/if}
    <div class="header-actions">
      <button class="btn-open" on:click={openWorkspace}>Open Folder</button>
      {#if $currentWorkspace}
        <button class="btn-refresh" on:click={refreshFiles} title="Refresh">↻</button>
      {/if}
    </div>
    {#if $currentWorkspace}
      <input
        class="search"
        bind:value={searchTerm}
        placeholder="Filter files…"
        aria-label="Filter files"
        spellcheck="false"
      />
    {/if}
  </div>

  <!-- File list -->
  <div class="file-list" role="tree" aria-label="Project files">
    {#if loading}
      <div class="notice">Loading…</div>
    {:else if error}
      <div class="notice error">{error}</div>
    {:else if !$currentWorkspace}
      <div class="notice muted">No workspace open</div>
    {:else if displayed.length === 0 && searchTerm}
      <div class="notice muted">No matches for "{searchTerm}"</div>
    {:else if displayed.length === 0}
      <div class="notice muted">Empty folder</div>
    {:else}
      {#each displayed as file (file.path)}
        {@const d = depth(file.rel)}
        {@const isExpanded = expandedDirs.has(file.rel)}
        <div
          class="row"
          class:dir={file.is_dir}
          class:selected={file.path === selectedPath}
          style="padding-left: {d * 14 + 8}px"
          role="button"
          aria-pressed={file.path === selectedPath}
          aria-expanded={file.is_dir ? isExpanded : undefined}
          tabindex="0"
          on:click={() => handleClick(file)}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleClick(file)}
          title={file.rel}
        >
          {#if file.is_dir}
            <span class="chevron" class:open={isExpanded}>›</span>
            <span class="icon dir-icon">📁</span>
          {:else}
            <span class="chevron invis">›</span>
            <span class="icon">{fileIcon(file.name)}</span>
          {/if}
          <span class="name">{file.name}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .filetree {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg2);
    border-right: 1px solid var(--border);
    overflow: hidden;
    font-size: 13px;
  }

  /* ── Header ── */
  .header {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex-shrink: 0;
  }

  .ws-row {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }

  .ws-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .ws-branch {
    font-size: 10px;
    color: var(--accent-hl);
    background: rgba(34, 197, 94, 0.12);
    border: 1px solid rgba(34, 197, 94, 0.25);
    border-radius: 10px;
    padding: 1px 7px;
    flex-shrink: 0;
  }

  .header-actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .btn-open {
    flex: 1;
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 5px 10px;
    font-size: 12px;
    cursor: pointer;
    transition: opacity 0.15s;
    font-weight: 500;
  }
  .btn-open:hover { opacity: 0.85; }

  .btn-refresh {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 14px;
    cursor: pointer;
    line-height: 1;
    transition: color 0.15s, border-color 0.15s;
  }
  .btn-refresh:hover { color: var(--text); border-color: var(--text-dim); }

  .search {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 8px;
    font-size: 12px;
    color: var(--text);
    width: 100%;
    outline: none;
    transition: border-color 0.15s;
  }
  .search:focus { border-color: var(--accent); }
  .search::placeholder { color: var(--text-dim); }

  /* ── File list ── */
  .file-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 0;
  }

  .notice {
    padding: 10px 12px;
    font-size: 12px;
    color: var(--text);
  }
  .notice.muted  { color: var(--text-dim); }
  .notice.error  { color: var(--red); }

  /* ── Row ── */
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding-right: 8px;
    cursor: pointer;
    border-radius: 4px;
    margin: 1px 4px;
    user-select: none;
    color: var(--text);
    transition: background 0.08s;
    min-width: 0;
  }
  .row:hover    { background: var(--bg-hover); }
  .row:focus    { outline: 1px solid var(--accent); outline-offset: -1px; }
  .row.selected { background: rgba(34, 197, 94, 0.18); }
  .row.dir      { color: var(--text); font-weight: 500; }

  /* Chevron: rotates to indicate open/closed state */
  .chevron {
    font-size: 13px;
    color: var(--text-dim);
    transition: transform 0.15s;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
    display: inline-block;
    transform: rotate(0deg);
  }
  .chevron.open    { transform: rotate(90deg); color: var(--accent-hl); }
  .chevron.invis   { opacity: 0; pointer-events: none; }

  .icon {
    font-size: 13px;
    flex-shrink: 0;
    width: 18px;
    text-align: center;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    flex: 1;
  }
</style>
