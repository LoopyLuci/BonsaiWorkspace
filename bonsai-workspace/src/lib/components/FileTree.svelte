<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { currentWorkspace, fileTreeRefresh, setWorkspace } from '$lib/stores/workspace';
  import { requestOpenFile } from '$lib/stores/openFile';
  import { detectFileType } from '$lib/utils/filetypes';

  interface FileEntry {
    path:   string;
    rel:    string;
    name:   string;
    is_dir: boolean;
  }

  type GitStatus = 'modified' | 'added' | 'deleted' | 'conflict' | 'unknown';
  type GitMap    = Map<string, GitStatus>;

  let allFiles:     FileEntry[] = [];
  let gitStatus:    GitMap      = new Map();
  let searchTerm    = '';
  let loading       = false;
  let error         = '';
  let expandedDirs  = new Set<string>();
  let selectedPath  = '';
  let showContextMenu = false;
  let contextX = 0;
  let contextY = 0;
  let contextDir = '';
  let compact   = false;

  // ── Pinned files (persisted via localStorage) ─────────────────────────────
  const PINS_KEY = 'bonsai:pinned-files';
  function loadPins(): Set<string> {
    try { return new Set(JSON.parse(localStorage.getItem(PINS_KEY) ?? '[]')); } catch { return new Set(); }
  }
  function savePins(p: Set<string>) {
    localStorage.setItem(PINS_KEY, JSON.stringify([...p]));
  }
  let pinned: Set<string> = loadPins();

  function togglePin(path: string) {
    pinned = new Set(pinned);
    if (pinned.has(path)) pinned.delete(path);
    else pinned.add(path);
    savePins(pinned);
  }

  // ── Load files whenever workspace or refresh signal changes ──────────────
  $: $currentWorkspace, $fileTreeRefresh, loadFiles();

  async function loadFiles() {
    if (!$currentWorkspace) { allFiles = []; gitStatus = new Map(); return; }
    loading = true; error = '';
    try {
      const [raw, statuses] = await Promise.all([
        invoke<FileEntry[]>('list_project_files', { workspacePath: $currentWorkspace.path }),
        invoke<{ path: string; status: string }[]>('get_git_status', { workspacePath: $currentWorkspace.path })
          .catch(() => [] as { path: string; status: string }[]),
      ]);

      allFiles = raw.sort((a, b) => {
        const aKey = a.rel.toLowerCase() + (a.is_dir ? '/' : '');
        const bKey = b.rel.toLowerCase() + (b.is_dir ? '/' : '');
        return aKey < bKey ? -1 : aKey > bKey ? 1 : 0;
      });

      const gm: GitMap = new Map();
      for (const s of statuses) {
        if (s.status !== 'clean') gm.set(s.path.replace(/\\/g, '/'), s.status as GitStatus);
      }
      gitStatus = gm;
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

  function joinPath(base: string, leaf: string): string {
    if (!base) return leaf;
    if (base.endsWith('/') || base.endsWith('\\')) return `${base}${leaf}`;
    return `${base}${base.includes('\\') ? '\\' : '/'}${leaf}`;
  }

  function dirname(path: string): string {
    const normalized = path.replace(/[\\/]+$/, '');
    const idx = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'));
    return idx >= 0 ? normalized.slice(0, idx) : normalized;
  }

  function defaultTargetDir(): string {
    if (!$currentWorkspace) return '';
    const selected = allFiles.find((f) => f.path === selectedPath);
    if (!selected) return $currentWorkspace.path;
    return selected.is_dir ? selected.path : dirname(selected.path);
  }

  function openContextMenu(event: MouseEvent, dir: string) {
    event.preventDefault();
    contextX = event.clientX;
    contextY = event.clientY;
    contextDir = dir || defaultTargetDir();
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
  }

  async function createNewFile(dir?: string) {
    if (!$currentWorkspace) return;
    const targetDir = dir || defaultTargetDir();
    const name = prompt('New file name:', 'new-file.txt');
    if (!name) return;
    const trimmed = name.trim();
    if (!trimmed) return;
    try {
      const path = joinPath(targetDir, trimmed);
      await invoke('write_file', { path, content: '' });
      refreshFiles();
      requestOpenFile(path);
    } catch (e) { error = String(e); }
  }

  async function createNewFolder(dir?: string) {
    if (!$currentWorkspace) return;
    const targetDir = dir || defaultTargetDir();
    const name = prompt('New folder name:', 'new-folder');
    if (!name) return;
    const trimmed = name.trim();
    if (!trimmed) return;
    try {
      const path = joinPath(targetDir, trimmed);
      await invoke('create_directory', { path });
      refreshFiles();
    } catch (e) { error = String(e); }
  }

  function depth(rel: string) { return (rel.match(/\//g) ?? []).length; }

  // ── Git status for a row ─────────────────────────────────────────────────
  // Normalize rel path and look up in git map.
  // For directories, check if any child has a status.
  function rowGitStatus(file: FileEntry): GitStatus | null {
    const relNorm = file.rel.replace(/\\/g, '/');
    if (!file.is_dir) {
      return gitStatus.get(relNorm) ?? null;
    }
    // Directory: return most urgent status of any child
    for (const [k, v] of gitStatus) {
      if (k.startsWith(relNorm + '/')) {
        if (v === 'conflict') return 'conflict';
      }
    }
    for (const [k, v] of gitStatus) {
      if (k.startsWith(relNorm + '/') && v === 'modified') return 'modified';
    }
    for (const [k, v] of gitStatus) {
      if (k.startsWith(relNorm + '/') && (v === 'added' || v === 'deleted')) return v;
    }
    return null;
  }

  const GIT_BADGE: Record<string, { label: string; cls: string }> = {
    modified: { label: 'M', cls: 'git-m' },
    added:    { label: 'A', cls: 'git-a' },
    deleted:  { label: 'D', cls: 'git-d' },
    conflict: { label: '!', cls: 'git-c' },
    unknown:  { label: '?', cls: 'git-u' },
  };

  // ── Visibility ──────────────────────────────────────────────────────────
  function computeDisplayed(files: FileEntry[], search: string, expanded: Set<string>): FileEntry[] {
    const needle = search.trim().toLowerCase();
    if (!needle) {
      return files.filter(f => {
        const parts = f.rel.split('/');
        for (let i = 1; i < parts.length; i++) {
          if (!expanded.has(parts.slice(0, i).join('/'))) return false;
        }
        return true;
      });
    }
    const visible = new Set<string>();
    for (const f of files) {
      if (f.rel.toLowerCase().includes(needle)) {
        visible.add(f.rel);
        const parts = f.rel.split('/');
        for (let i = 1; i < parts.length; i++) {
          visible.add(parts.slice(0, i).join('/'));
        }
      }
    }
    return files.filter(f => visible.has(f.rel));
  }

  $: displayed = computeDisplayed(allFiles, searchTerm, expandedDirs);
  $: fileCount  = displayed.filter(f => !f.is_dir).length;

  // Pinned files that exist in current workspace
  $: pinnedEntries = allFiles.filter(f => !f.is_dir && pinned.has(f.path));

  // ── Interaction ─────────────────────────────────────────────────────────
  function handleClick(file: FileEntry) {
    selectedPath = file.path;
    if (file.is_dir) {
      if (expandedDirs.has(file.rel)) {
        expandedDirs = new Set([...expandedDirs].filter(d => !d.startsWith(file.rel)));
      } else {
        expandedDirs = new Set([...expandedDirs, file.rel]);
      }
    } else {
      requestOpenFile(file.path);
    }
  }

  function fileTypeInfo(name: string) {
    return detectFileType(name);
  }
</script>

<svelte:window on:click={closeContextMenu} on:keydown={(e) => e.key === 'Escape' && closeContextMenu()} />

<div class="filetree" class:compact>
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
        <button class="btn-create" on:click={() => createNewFile()} title="New File">+ File</button>
        <button class="btn-create" on:click={() => createNewFolder()} title="New Folder">+ Dir</button>
        <button class="btn-refresh" on:click={refreshFiles} title="Refresh">↻</button>
        <button class="btn-icon" class:active={compact} on:click={() => compact = !compact} title="Compact mode">≡</button>
      {/if}
    </div>
    {#if $currentWorkspace}
      <!-- Sticky filter bar with count + clear -->
      <div class="filter-bar">
        <input
          class="search"
          bind:value={searchTerm}
          placeholder="Filter files…"
          aria-label="Filter files"
          spellcheck="false"
        />
        {#if searchTerm}
          <span class="filter-count">{fileCount}</span>
          <button class="filter-clear" on:click={() => searchTerm = ''} title="Clear filter">×</button>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Pinned files section -->
  {#if pinnedEntries.length > 0}
    <div class="pins-section">
      <div class="pins-label">Pinned</div>
      {#each pinnedEntries as file (file.path)}
        {@const ft = fileTypeInfo(file.name)}
        {@const gs = rowGitStatus(file)}
        <div
          class="row pin-row"
          class:selected={file.path === selectedPath}
          role="button"
          aria-pressed={file.path === selectedPath}
          tabindex="0"
          on:click={() => handleClick(file)}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleClick(file)}
          title={file.rel}
        >
          <span class="icon" style="color:{ft.iconColor}">{ft.icon}</span>
          <span class="name">{file.name}</span>
          {#if gs && GIT_BADGE[gs]}
            <span class="git-badge {GIT_BADGE[gs].cls}">{GIT_BADGE[gs].label}</span>
          {/if}
          <button class="pin-btn active" on:click|stopPropagation={() => togglePin(file.path)} title="Unpin">📌</button>
        </div>
      {/each}
      <div class="pins-divider"></div>
    </div>
  {/if}

  <!-- File list -->
  <div class="file-list" role="tree" tabindex="0" aria-label="Project files" on:contextmenu={(e) => openContextMenu(e, defaultTargetDir())}>
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
        {@const gs = rowGitStatus(file)}
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
          on:contextmenu={(e) => openContextMenu(e, file.is_dir ? file.path : dirname(file.path))}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleClick(file)}
          title={file.rel}
        >
          {#if file.is_dir}
            <span class="chevron" class:open={isExpanded}>›</span>
            <span class="icon dir-icon">📁</span>
          {:else}
            {@const ft = fileTypeInfo(file.name)}
            <span class="chevron invis">›</span>
            <span class="icon" style="color:{ft.iconColor}">{ft.icon}</span>
          {/if}
          <span class="name" class:git-name-m={gs === 'modified'} class:git-name-a={gs === 'added'} class:git-name-d={gs === 'deleted'}>{file.name}</span>
          {#if gs && GIT_BADGE[gs]}
            <span class="git-badge {GIT_BADGE[gs].cls}">{GIT_BADGE[gs].label}</span>
          {/if}
          {#if !file.is_dir}
            <button class="pin-btn" class:active={pinned.has(file.path)} on:click|stopPropagation={() => togglePin(file.path)} title={pinned.has(file.path) ? 'Unpin' : 'Pin'}>
              {pinned.has(file.path) ? '📌' : '⬡'}
            </button>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  {#if showContextMenu && $currentWorkspace}
    <div class="context-menu" style="left: {contextX}px; top: {contextY}px">
      <button class="context-item" on:click={() => { createNewFile(contextDir); closeContextMenu(); }}>New File</button>
      <button class="context-item" on:click={() => { createNewFolder(contextDir); closeContextMenu(); }}>New Folder</button>
    </div>
  {/if}
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

  .btn-refresh, .btn-icon {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 14px;
    cursor: pointer;
    line-height: 1;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .btn-refresh:hover, .btn-icon:hover { color: var(--text); border-color: var(--text-dim); }
  .btn-icon.active { color: var(--accent-hl); border-color: var(--accent-hl); background: rgba(34, 197, 94, 0.08); }

  .btn-create {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 12px;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
    line-height: 1;
  }
  .btn-create:hover { color: var(--text); border-color: var(--text-dim); }

  /* ── Filter bar ── */
  .filter-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    position: relative;
  }

  .search {
    flex: 1;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 8px;
    font-size: 12px;
    color: var(--text);
    outline: none;
    transition: border-color 0.15s;
    min-width: 0;
  }
  .search:focus { border-color: var(--accent); }
  .search::placeholder { color: var(--text-dim); }

  .filter-count {
    font-size: 10px;
    color: var(--text-dim);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1px 6px;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .filter-clear {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 16px;
    cursor: pointer;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
  }
  .filter-clear:hover { color: var(--text); }

  /* ── Pinned section ── */
  .pins-section {
    flex-shrink: 0;
    padding: 4px 0 0;
  }
  .pins-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 2px 10px;
  }
  .pin-row {
    padding-left: 8px !important;
  }
  .pins-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 6px;
  }

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
    padding-right: 6px;
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

  /* Compact mode: tighter rows */
  .compact .row { height: 20px; margin: 0 4px; border-radius: 3px; }

  /* Chevron */
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

  /* Git-colored filenames */
  .git-name-m { color: #e2c08d; }
  .git-name-a { color: #81c784; }
  .git-name-d { color: #e06c75; text-decoration: line-through; }

  /* Git badges */
  .git-badge {
    font-size: 10px;
    font-weight: 700;
    border-radius: 3px;
    padding: 0 3px;
    flex-shrink: 0;
    line-height: 15px;
  }
  .git-m { color: #e2c08d; background: rgba(226, 192, 141, 0.12); }
  .git-a { color: #81c784; background: rgba(129, 199, 132, 0.12); }
  .git-d { color: #e06c75; background: rgba(224, 108, 117, 0.12); }
  .git-c { color: #f44747; background: rgba(244, 71, 71, 0.15); }
  .git-u { color: var(--text-dim); background: transparent; }

  /* Pin button — hidden until row hover */
  .pin-btn {
    background: transparent;
    border: none;
    font-size: 11px;
    cursor: pointer;
    padding: 0 2px;
    opacity: 0;
    flex-shrink: 0;
    line-height: 1;
    color: var(--text-dim);
    transition: opacity 0.1s;
  }
  .pin-btn.active { opacity: 1; }
  .row:hover .pin-btn { opacity: 0.7; }
  .row:hover .pin-btn:hover { opacity: 1; }

  .context-menu {
    position: fixed;
    min-width: 140px;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    padding: 6px;
    z-index: var(--z-context, 1000);
  }

  .context-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text);
    border-radius: 6px;
    padding: 7px 9px;
    font-size: 12px;
    cursor: pointer;
  }
  .context-item:hover {
    background: var(--bg-hover);
    border-color: var(--border);
  }
</style>
