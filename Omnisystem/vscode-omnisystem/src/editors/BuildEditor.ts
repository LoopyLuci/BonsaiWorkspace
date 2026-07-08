import * as vscode from 'vscode';

// ── Types ────────────────────────────────────────────────────────────────────

interface BuildConfig {
  name: string;
  version: string;
  authors: string[];
  sources: string[];
  dependencies: string[];      // "pkg@version"
  targets: string[];
  optimizationLevel: 'debug' | 'release' | 'size';
  parallelJobs: number;
  testFiles: string;
  testRunner: string;
}

type WebviewMessage =
  | { type: 'save'; config: BuildConfig }
  | { type: 'ready' };

// ── Provider ─────────────────────────────────────────────────────────────────

export class BuildEditorProvider implements vscode.CustomTextEditorProvider {

  public static readonly viewType = 'omnisystem.buildEditor';

  public static register(context: vscode.ExtensionContext): vscode.Disposable {
    const provider = new BuildEditorProvider(context);
    return vscode.window.registerCustomEditorProvider(
      BuildEditorProvider.viewType,
      provider,
      {
        webviewOptions: { retainContextWhenHidden: true },
        supportsMultipleEditorsPerDocument: false
      }
    );
  }

  constructor(private readonly _context: vscode.ExtensionContext) {}

  public async resolveCustomTextEditor(
    document: vscode.TextDocument,
    webviewPanel: vscode.WebviewPanel,
    _token: vscode.CancellationToken
  ): Promise<void> {
    webviewPanel.webview.options = { enableScripts: true };
    webviewPanel.webview.html = this._buildHtml(webviewPanel.webview);

    // Push parsed config to webview whenever the file changes
    const pushUpdate = () => {
      const config = this._parse(document.getText());
      webviewPanel.webview.postMessage({ type: 'update', config });
    };

    const changeSubscription = vscode.workspace.onDidChangeTextDocument(e => {
      if (e.document.uri.toString() === document.uri.toString()) {
        pushUpdate();
      }
    });

    // Handle messages from the webview
    webviewPanel.webview.onDidReceiveMessage(async (msg: WebviewMessage) => {
      if (msg.type === 'ready') {
        pushUpdate();
      } else if (msg.type === 'save') {
        await this._applyEdits(document, msg.config);
        await document.save();
      }
    });

    webviewPanel.onDidDispose(() => changeSubscription.dispose());
  }

  // ── Parsing ────────────────────────────────────────────────────────────────

  private _parse(text: string): BuildConfig {
    const cfg: BuildConfig = {
      name: '',
      version: '0.1.0',
      authors: [],
      sources: [],
      dependencies: [],
      targets: [],
      optimizationLevel: 'debug',
      parallelJobs: 4,
      testFiles: '**/*_test.titan',
      testRunner: 'omnicc test'
    };

    const lines = text.split(/\r?\n/);
    let section = '';

    for (const raw of lines) {
      const line = raw.trim();

      if (line.startsWith('[') && line.endsWith(']')) {
        section = line.slice(1, -1).trim();
        continue;
      }

      if (!line || line.startsWith('#')) continue;

      const eqIdx = line.indexOf('=');
      if (eqIdx === -1) continue;
      const key = line.slice(0, eqIdx).trim();
      const val = line.slice(eqIdx + 1).trim().replace(/^["']|["']$/g, '');

      if (section === 'project') {
        if (key === 'name') cfg.name = val;
        if (key === 'version') cfg.version = val;
        if (key === 'authors') cfg.authors = this._parseList(val);
      } else if (section === 'build') {
        if (key === 'sources') cfg.sources = this._parseList(val);
        if (key === 'targets') cfg.targets = this._parseList(val);
        if (key === 'opt') cfg.optimizationLevel = (val as BuildConfig['optimizationLevel']) ?? 'debug';
        if (key === 'jobs') cfg.parallelJobs = parseInt(val, 10) || 4;
      } else if (section === 'dependencies') {
        cfg.dependencies.push(`${key}@${val}`);
      } else if (section === 'test') {
        if (key === 'files') cfg.testFiles = val;
        if (key === 'runner') cfg.testRunner = val;
      }
    }

    return cfg;
  }

  private _parseList(val: string): string[] {
    // Handles: ["a", "b"] or a, b
    return val
      .replace(/^\[|\]$/g, '')
      .split(',')
      .map(s => s.trim().replace(/^["']|["']$/g, ''))
      .filter(Boolean);
  }

  private _serialize(cfg: BuildConfig): string {
    const lines: string[] = [];

    lines.push('[project]');
    lines.push(`name = "${cfg.name}"`);
    lines.push(`version = "${cfg.version}"`);
    lines.push(`authors = [${cfg.authors.map(a => `"${a}"`).join(', ')}]`);
    lines.push('');

    lines.push('[build]');
    lines.push(`sources = [${cfg.sources.map(s => `"${s}"`).join(', ')}]`);
    lines.push(`targets = [${cfg.targets.map(t => `"${t}"`).join(', ')}]`);
    lines.push(`opt = "${cfg.optimizationLevel}"`);
    lines.push(`jobs = ${cfg.parallelJobs}`);
    lines.push('');

    if (cfg.dependencies.length > 0) {
      lines.push('[dependencies]');
      for (const dep of cfg.dependencies) {
        const atIdx = dep.lastIndexOf('@');
        if (atIdx !== -1) {
          lines.push(`${dep.slice(0, atIdx)} = "${dep.slice(atIdx + 1)}"`);
        } else {
          lines.push(`${dep} = "*"`);
        }
      }
      lines.push('');
    }

    lines.push('[test]');
    lines.push(`files = "${cfg.testFiles}"`);
    lines.push(`runner = "${cfg.testRunner}"`);
    lines.push('');

    return lines.join('\n');
  }

  private async _applyEdits(document: vscode.TextDocument, cfg: BuildConfig): Promise<void> {
    const edit = new vscode.WorkspaceEdit();
    const fullRange = new vscode.Range(
      document.positionAt(0),
      document.positionAt(document.getText().length)
    );
    edit.replace(document.uri, fullRange, this._serialize(cfg));
    await vscode.workspace.applyEdit(edit);
  }

  // ── HTML ───────────────────────────────────────────────────────────────────

  private _buildHtml(_webview: vscode.Webview): string {
    return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>BUILD.omnisystem</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: var(--vscode-font-family);
    font-size: var(--vscode-font-size);
    color: var(--vscode-foreground);
    background: var(--vscode-editor-background);
    padding: 16px;
    max-width: 860px;
  }
  h2 { margin: 0 0 12px; font-size: 1rem; font-weight: 600; color: var(--vscode-foreground); }
  .section {
    border: 1px solid var(--vscode-panel-border, #3c3c3c);
    border-radius: 4px;
    margin-bottom: 16px;
    overflow: hidden;
  }
  .section-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 12px;
    background: var(--vscode-sideBar-background, #252526);
    cursor: pointer;
    user-select: none;
  }
  .section-header:hover { background: var(--vscode-list-hoverBackground, #2a2d2e); }
  .section-body { padding: 12px; display: grid; gap: 8px; }
  .section-body.collapsed { display: none; }
  .row { display: grid; grid-template-columns: 140px 1fr; align-items: center; gap: 8px; }
  label { color: var(--vscode-descriptionForeground); font-size: 0.85rem; }
  input[type="text"], input[type="number"] {
    background: var(--vscode-input-background);
    color: var(--vscode-input-foreground);
    border: 1px solid var(--vscode-input-border, #3c3c3c);
    border-radius: 2px;
    padding: 4px 6px;
    width: 100%;
    font-family: inherit;
    font-size: inherit;
  }
  input[type="range"] { width: 100%; accent-color: var(--vscode-button-background); }
  select {
    background: var(--vscode-dropdown-background);
    color: var(--vscode-dropdown-foreground);
    border: 1px solid var(--vscode-dropdown-border, #3c3c3c);
    border-radius: 2px;
    padding: 4px 6px;
    width: 100%;
    font-family: inherit;
    font-size: inherit;
  }
  .list-table { width: 100%; border-collapse: collapse; }
  .list-table td { padding: 3px 6px; vertical-align: middle; }
  .list-table tr:nth-child(even) { background: var(--vscode-list-hoverBackground, #2a2d2e); }
  .remove-btn {
    background: none; border: none; color: var(--vscode-errorForeground, #f48771);
    cursor: pointer; font-size: 1rem; padding: 0 4px;
  }
  .add-row { display: flex; gap: 6px; margin-top: 4px; }
  .add-input {
    flex: 1;
    background: var(--vscode-input-background);
    color: var(--vscode-input-foreground);
    border: 1px solid var(--vscode-input-border, #3c3c3c);
    border-radius: 2px;
    padding: 4px 6px;
    font-family: inherit;
    font-size: inherit;
  }
  .btn {
    background: var(--vscode-button-background);
    color: var(--vscode-button-foreground);
    border: none; border-radius: 2px;
    padding: 4px 10px; cursor: pointer;
    font-family: inherit; font-size: inherit;
  }
  .btn:hover { background: var(--vscode-button-hoverBackground); }
  .btn-secondary {
    background: var(--vscode-button-secondaryBackground, #3c3c3c);
    color: var(--vscode-button-secondaryForeground, #cccccc);
  }
  .btn-secondary:hover { background: var(--vscode-button-secondaryHoverBackground, #4c4c4c); }
  .target-grid { display: flex; flex-wrap: wrap; gap: 10px; }
  .target-item { display: flex; align-items: center; gap: 6px; }
  .save-bar {
    position: sticky; bottom: 0;
    background: var(--vscode-editor-background);
    border-top: 1px solid var(--vscode-panel-border, #3c3c3c);
    padding: 10px 0;
    display: flex; gap: 8px;
  }
  .jobs-label { font-size: 0.85rem; color: var(--vscode-descriptionForeground); }
  .arrow { transition: transform 0.15s; display: inline-block; margin-right: 6px; }
  .collapsed-arrow { transform: rotate(-90deg); }
</style>
</head>
<body>

<!-- Project Info -->
<div class="section" id="sec-project">
  <div class="section-header" onclick="toggleSection('project')">
    <span><span class="arrow" id="arrow-project">▼</span><strong>Project Info</strong></span>
  </div>
  <div class="section-body" id="body-project">
    <div class="row"><label>Name</label><input type="text" id="proj-name"></div>
    <div class="row"><label>Version</label><input type="text" id="proj-version"></div>
    <div class="row"><label>Authors</label>
      <div>
        <table class="list-table" id="authors-table"></table>
        <div class="add-row">
          <input type="text" class="add-input" id="author-input" placeholder="Author Name">
          <button class="btn btn-secondary" onclick="addAuthor()">+ Add</button>
        </div>
      </div>
    </div>
  </div>
</div>

<!-- Source Files -->
<div class="section" id="sec-sources">
  <div class="section-header" onclick="toggleSection('sources')">
    <span><span class="arrow" id="arrow-sources">▼</span><strong>Source Files</strong></span>
  </div>
  <div class="section-body" id="body-sources">
    <table class="list-table" id="sources-table"></table>
    <div class="add-row">
      <input type="text" class="add-input" id="source-input" placeholder="src/main.titan">
      <button class="btn btn-secondary" onclick="addSource()">+ Add</button>
    </div>
  </div>
</div>

<!-- Dependencies -->
<div class="section" id="sec-deps">
  <div class="section-header" onclick="toggleSection('deps')">
    <span><span class="arrow" id="arrow-deps">▼</span><strong>Dependencies</strong></span>
  </div>
  <div class="section-body" id="body-deps">
    <table class="list-table" id="deps-table"></table>
    <div class="add-row">
      <input type="text" class="add-input" id="dep-input" placeholder="package@1.0.0">
      <button class="btn btn-secondary" onclick="addDep()">+ Add</button>
    </div>
  </div>
</div>

<!-- Build Targets -->
<div class="section" id="sec-targets">
  <div class="section-header" onclick="toggleSection('targets')">
    <span><span class="arrow" id="arrow-targets">▼</span><strong>Build Targets</strong></span>
  </div>
  <div class="section-body" id="body-targets">
    <div class="target-grid">
      <label class="target-item"><input type="checkbox" id="tgt-x86_64-linux"> x86_64-linux</label>
      <label class="target-item"><input type="checkbox" id="tgt-x86_64-windows"> x86_64-windows</label>
      <label class="target-item"><input type="checkbox" id="tgt-wasm32"> wasm32</label>
      <label class="target-item"><input type="checkbox" id="tgt-aarch64-linux"> aarch64-linux</label>
      <label class="target-item"><input type="checkbox" id="tgt-aarch64-macos"> aarch64-macos</label>
    </div>
  </div>
</div>

<!-- Build Options -->
<div class="section" id="sec-opts">
  <div class="section-header" onclick="toggleSection('opts')">
    <span><span class="arrow" id="arrow-opts">▼</span><strong>Build Options</strong></span>
  </div>
  <div class="section-body" id="body-opts">
    <div class="row">
      <label>Optimization</label>
      <select id="opt-level">
        <option value="debug">debug</option>
        <option value="release">release</option>
        <option value="size">size</option>
      </select>
    </div>
    <div class="row">
      <label>Parallel Jobs <span class="jobs-label" id="jobs-display">(4)</span></label>
      <input type="range" id="opt-jobs" min="1" max="32" value="4"
        oninput="document.getElementById('jobs-display').textContent='(' + this.value + ')'">
    </div>
  </div>
</div>

<!-- Test Config -->
<div class="section" id="sec-test">
  <div class="section-header" onclick="toggleSection('test')">
    <span><span class="arrow" id="arrow-test">▼</span><strong>Test Config</strong></span>
  </div>
  <div class="section-body" id="body-test">
    <div class="row"><label>Test Files Pattern</label><input type="text" id="test-files"></div>
    <div class="row"><label>Test Runner</label><input type="text" id="test-runner"></div>
  </div>
</div>

<div class="save-bar">
  <button class="btn" onclick="save()">Save</button>
  <button class="btn btn-secondary" onclick="requestUpdate()">Reload</button>
</div>

<script>
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const vscode = acquireVsCodeApi();

  const KNOWN_TARGETS = ['x86_64-linux', 'x86_64-windows', 'wasm32', 'aarch64-linux', 'aarch64-macos'];
  let _config = {};

  // ── Section toggle ──────────────────────────────────────────────────────────
  function toggleSection(id) {
    const body = document.getElementById('body-' + id);
    const arrow = document.getElementById('arrow-' + id);
    body.classList.toggle('collapsed');
    arrow.classList.toggle('collapsed-arrow');
  }

  // ── List helpers ────────────────────────────────────────────────────────────
  function renderList(tableId, items, onRemove) {
    const t = document.getElementById(tableId);
    t.innerHTML = '';
    items.forEach((item, i) => {
      const tr = document.createElement('tr');
      tr.innerHTML = \`<td>\${item}</td><td><button class="remove-btn" onclick="\${onRemove}(\${i})">✕</button></td>\`;
      t.appendChild(tr);
    });
  }

  function addAuthor() {
    const inp = document.getElementById('author-input');
    if (!inp.value.trim()) return;
    _config.authors = [...(_config.authors || []), inp.value.trim()];
    inp.value = '';
    renderAll();
  }
  function removeAuthor(i) { _config.authors.splice(i, 1); renderAll(); }

  function addSource() {
    const inp = document.getElementById('source-input');
    if (!inp.value.trim()) return;
    _config.sources = [...(_config.sources || []), inp.value.trim()];
    inp.value = '';
    renderAll();
  }
  function removeSource(i) { _config.sources.splice(i, 1); renderAll(); }

  function addDep() {
    const inp = document.getElementById('dep-input');
    if (!inp.value.trim()) return;
    _config.dependencies = [...(_config.dependencies || []), inp.value.trim()];
    inp.value = '';
    renderAll();
  }
  function removeDep(i) { _config.dependencies.splice(i, 1); renderAll(); }

  // ── Render ──────────────────────────────────────────────────────────────────
  function renderAll() {
    document.getElementById('proj-name').value = _config.name || '';
    document.getElementById('proj-version').value = _config.version || '';
    renderList('authors-table', _config.authors || [], 'removeAuthor');
    renderList('sources-table', _config.sources || [], 'removeSource');
    renderList('deps-table', _config.dependencies || [], 'removeDep');
    KNOWN_TARGETS.forEach(t => {
      const cb = document.getElementById('tgt-' + t);
      if (cb) cb.checked = (_config.targets || []).includes(t);
    });
    document.getElementById('opt-level').value = _config.optimizationLevel || 'debug';
    const jobs = _config.parallelJobs || 4;
    document.getElementById('opt-jobs').value = jobs;
    document.getElementById('jobs-display').textContent = '(' + jobs + ')';
    document.getElementById('test-files').value = _config.testFiles || '';
    document.getElementById('test-runner').value = _config.testRunner || '';
  }

  // ── Collect ─────────────────────────────────────────────────────────────────
  function collect() {
    return {
      name: document.getElementById('proj-name').value,
      version: document.getElementById('proj-version').value,
      authors: _config.authors || [],
      sources: _config.sources || [],
      dependencies: _config.dependencies || [],
      targets: KNOWN_TARGETS.filter(t => document.getElementById('tgt-' + t)?.checked),
      optimizationLevel: document.getElementById('opt-level').value,
      parallelJobs: parseInt(document.getElementById('opt-jobs').value, 10),
      testFiles: document.getElementById('test-files').value,
      testRunner: document.getElementById('test-runner').value
    };
  }

  function save() { vscode.postMessage({ type: 'save', config: collect() }); }
  function requestUpdate() { vscode.postMessage({ type: 'ready' }); }

  // ── Messages from extension ─────────────────────────────────────────────────
  window.addEventListener('message', e => {
    if (e.data.type === 'update') {
      _config = e.data.config;
      renderAll();
    }
  });

  // Ready
  vscode.postMessage({ type: 'ready' });
</script>
</body>
</html>`;
  }
}
