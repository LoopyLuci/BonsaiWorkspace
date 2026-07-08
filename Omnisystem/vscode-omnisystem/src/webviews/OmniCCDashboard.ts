import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { OmniCCConversionEngine, quickConvert } from '../omnicc/ConversionEngine';
import { detectLanguage } from '../omnicc/LanguageDetector';
import { allLanguages, searchLanguages, getLang } from '../omnicc/LanguageRegistry';

export class OmniCCDashboardPanel {
    static readonly viewType = 'omnisystem.omniCC';
    static currentPanel: OmniCCDashboardPanel | undefined;

    private readonly _panel: vscode.WebviewPanel;
    private readonly _engine: OmniCCConversionEngine;
    private _disposables: vscode.Disposable[] = [];
    private _history: Array<{ src: string; tgt: string; output: string; ts: number }> = [];
    private _projectFolder: string | undefined;

    static createOrShow(extensionUri: vscode.Uri): void {
        const column = vscode.window.activeTextEditor?.viewColumn ?? vscode.ViewColumn.One;
        if (OmniCCDashboardPanel.currentPanel) {
            OmniCCDashboardPanel.currentPanel._panel.reveal(column);
            return;
        }
        const panel = vscode.window.createWebviewPanel(
            OmniCCDashboardPanel.viewType,
            'OmniCC — Universal Language Converter',
            column,
            {
                enableScripts: true,
                retainContextWhenHidden: true,
                localResourceRoots: [vscode.Uri.joinPath(extensionUri, 'media')],
            }
        );
        OmniCCDashboardPanel.currentPanel = new OmniCCDashboardPanel(panel, extensionUri);
    }

    static postMessage(msg: unknown): void {
        OmniCCDashboardPanel.currentPanel?._panel.webview.postMessage(msg);
    }

    private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
        this._panel = panel;
        this._engine = new OmniCCConversionEngine();
        this._panel.webview.html = this._buildHtml(extensionUri);
        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
        this._panel.webview.onDidReceiveMessage(msg => this._handleMessage(msg), null, this._disposables);
    }

    private async _handleMessage(msg: { command: string; [key: string]: unknown }): Promise<void> {
        switch (msg.command) {
            case 'convert': {
                const source = String(msg.source ?? '');
                const targetLang = String(msg.targetLang ?? 'javascript');
                const sourceLang = msg.sourceLang !== 'auto' ? String(msg.sourceLang) : undefined;
                const filename = msg.filename ? String(msg.filename) : undefined;
                if (!source.trim()) {
                    this._post({ command: 'convertResult', error: 'No source code provided', output: '' });
                    return;
                }
                const result = this._engine.convert({ source, targetLang, sourceLang, filename });
                this._addToHistory(result.sourceLanguage, result.targetLanguage, result.output);
                this._post({
                    command: 'convertResult',
                    output: result.output,
                    sourceLangId: result.sourceLangId,
                    targetLangId: result.targetLangId,
                    confidence: result.confidence,
                    signals: result.detectionSignals,
                    notes: result.notes,
                    unitCount: result.ir?.units?.length ?? 0,
                    linesConverted: result.linesConverted,
                    durationMs: result.durationMs,
                    widgetBridge: result.widgetBridge,
                    widgetPreview: result.widgetBridge?.previewHtml ?? '',
                });
                break;
            }
            case 'detect': {
                const source = String(msg.source ?? '');
                const filename = msg.filename ? String(msg.filename) : undefined;
                const detected = detectLanguage(source, filename);
                this._post({ command: 'detectResult', ...detected });
                break;
            }
            case 'searchLangs': {
                const query = String(msg.query ?? '');
                const results = query.length >= 2 ? searchLanguages(query).slice(0, 50) : allLanguages().slice(0, 100);
                this._post({ command: 'searchLangsResult', langs: results.map(l => ({ id: l.id, name: l.name, family: l.family, popularity: l.popularity, color: l.color, year: l.year, description: l.description })) });
                break;
            }
            case 'getLangInfo': {
                const lang = getLang(String(msg.langId ?? ''));
                this._post({ command: 'langInfoResult', lang });
                break;
            }
            case 'convertProject': {
                const files = (msg.files as Array<{ path: string; content: string }>) ?? [];
                const targetLang = String(msg.targetLang ?? 'javascript');
                if (files.length === 0) {
                    this._post({ command: 'projectResult', error: 'No files provided' });
                    return;
                }
                this._post({ command: 'projectProgress', total: files.length, done: 0 });
                const result = this._engine.convert({ source: '', targetLang, projectMode: true, projectFiles: files });
                this._post({
                    command: 'projectResult',
                    success: result.success,
                    fileCount: files.length,
                    successCount: result.projectResults?.filter(r => !r.error).length ?? 0,
                    totalLines: result.linesConverted,
                    durationMs: result.durationMs,
                    files: result.projectResults?.map(r => ({
                        path: r.path,
                        targetPath: r.targetPath,
                        sourceLang: r.sourceLangId,
                        linesIn: r.linesIn,
                        linesOut: r.linesOut,
                        confidence: r.confidence,
                        error: r.error,
                        output: (r.output ?? '').slice(0, 2000),
                    })),
                });
                break;
            }
            case 'openProjectFolder': {
                const uris = await vscode.window.showOpenDialog({
                    canSelectFolders: true,
                    canSelectFiles: false,
                    canSelectMany: false,
                    openLabel: 'Select Project Folder',
                    title: 'OmniCC — Select folder to convert',
                });
                if (!uris || uris.length === 0) { break; }
                this._projectFolder = uris[0].fsPath;
                const files = this._scanFolder(this._projectFolder);
                this._post({ command: 'projectFolderLoaded', folder: this._projectFolder, files });
                break;
            }
            case 'exportProject': {
                const results = (msg.results as Array<{ path: string; targetPath: string; output: string }>) ?? [];
                if (results.length === 0) { vscode.window.showWarningMessage('No converted files to export.'); break; }
                const saveUri = await vscode.window.showOpenDialog({
                    canSelectFolders: true, canSelectFiles: false, canSelectMany: false,
                    openLabel: 'Export to this folder', title: 'OmniCC — Export converted files',
                });
                if (!saveUri || saveUri.length === 0) { break; }
                const outDir = saveUri[0].fsPath;
                let saved = 0;
                for (const r of results) {
                    if (!r.output || r.output.trim() === '') { continue; }
                    const targetName = path.basename(r.targetPath ?? r.path ?? 'file.txt');
                    const outPath = path.join(outDir, targetName);
                    try {
                        fs.writeFileSync(outPath, r.output, 'utf-8');
                        saved++;
                    } catch { /* skip */ }
                }
                vscode.window.showInformationMessage(`OmniCC: Exported ${saved} file(s) to ${outDir}`);
                break;
            }
            case 'openFile': {
                const filePath = String(msg.path ?? '');
                if (filePath) {
                    const doc = await vscode.workspace.openTextDocument(filePath);
                    await vscode.window.showTextDocument(doc);
                }
                break;
            }
            case 'saveOutput': {
                const content = String(msg.content ?? '');
                const ext = String(msg.ext ?? '.txt');
                const uri = await vscode.window.showSaveDialog({ filters: { 'Converted': [ext.replace('.', '')] } });
                if (uri) {
                    await vscode.workspace.fs.writeFile(uri, Buffer.from(content, 'utf-8'));
                    vscode.window.showInformationMessage(`Saved to ${uri.fsPath}`);
                }
                break;
            }
            case 'copyOutput': {
                await vscode.env.clipboard.writeText(String(msg.content ?? ''));
                vscode.window.showInformationMessage('Copied to clipboard');
                break;
            }
            case 'openEditor': {
                const content = String(msg.content ?? '');
                const ext = String(msg.ext ?? '.txt');
                const tmpUri = vscode.Uri.parse(`untitled:converted${ext}`);
                const doc = await vscode.workspace.openTextDocument(tmpUri);
                const edit = new vscode.WorkspaceEdit();
                edit.insert(tmpUri, new vscode.Position(0, 0), content);
                await vscode.workspace.applyEdit(edit);
                await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
                break;
            }
            case 'owThemeChange': {
                const theme = String(msg.theme ?? '');
                if (theme) {
                    await vscode.commands.executeCommand('omnisystem._broadcastTheme', theme);
                }
                break;
            }
        }
    }

    private _post(msg: unknown): void {
        this._panel.webview.postMessage(msg);
    }

    private _scanFolder(folderPath: string): Array<{ path: string; content: string }> {
        const SOURCE_EXTS = new Set([
            '.js', '.ts', '.jsx', '.tsx', '.py', '.java', '.kt', '.cs', '.go',
            '.rs', '.swift', '.dart', '.rb', '.lua', '.php', '.r', '.jl', '.ex',
            '.erl', '.hs', '.ml', '.fs', '.elm', '.clj', '.sh', '.bash', '.zsh',
            '.ps1', '.fish', '.sql', '.graphql', '.proto', '.html', '.css', '.scss',
            '.vue', '.svelte', '.c', '.cpp', '.h', '.zig', '.odin', '.v', '.d',
            '.titan', '.vera', '.helix', '.aether', '.axiom', '.sylva', '.nexus',
            '.json', '.yaml', '.yml', '.toml', '.xml', '.tf',
        ]);
        const SKIP_DIRS = new Set(['node_modules', '.git', 'dist', 'build', 'out', '__pycache__', '.venv', 'venv', 'target', '.cache']);
        const files: Array<{ path: string; content: string }> = [];

        const walk = (dir: string, depth: number = 0): void => {
            if (depth > 8) { return; }
            let entries: fs.Dirent[];
            try { entries = fs.readdirSync(dir, { withFileTypes: true }); } catch { return; }
            for (const entry of entries) {
                if (entry.name.startsWith('.') && entry.name !== '.env') { continue; }
                const fullPath = path.join(dir, entry.name);
                if (entry.isDirectory()) {
                    if (!SKIP_DIRS.has(entry.name)) { walk(fullPath, depth + 1); }
                } else if (entry.isFile()) {
                    const ext = path.extname(entry.name).toLowerCase();
                    if (SOURCE_EXTS.has(ext) && files.length < 5000) {
                        try {
                            const stat = fs.statSync(fullPath);
                            if (stat.size < 2 * 1024 * 1024) { // skip files >2MB
                                const content = fs.readFileSync(fullPath, 'utf-8');
                                files.push({ path: fullPath, content });
                            }
                        } catch { /* skip unreadable */ }
                    }
                }
            }
        };

        walk(folderPath);
        return files;
    }

    private _addToHistory(src: string, tgt: string, output: string): void {
        this._history.unshift({ src, tgt, output, ts: Date.now() });
        if (this._history.length > 20) { this._history.pop(); }
    }

    dispose(): void {
        OmniCCDashboardPanel.currentPanel = undefined;
        this._panel.dispose();
        for (const d of this._disposables) { d.dispose(); }
        this._disposables = [];
    }

    // ─── HTML ─────────────────────────────────────────────────────────────────

    private _buildHtml(extensionUri: vscode.Uri): string {
        const owCss = this._panel.webview.asWebviewUri(vscode.Uri.joinPath(extensionUri, 'media', 'omni-widgets.css'));
        const owJs = this._panel.webview.asWebviewUri(vscode.Uri.joinPath(extensionUri, 'media', 'omni-widgets.js'));
        const csp = `default-src 'none'; style-src ${this._panel.webview.cspSource} 'unsafe-inline'; script-src ${this._panel.webview.cspSource} 'unsafe-inline'; font-src ${this._panel.webview.cspSource};`;

        return /* html */`<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy" content="${csp}">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>OmniCC — Universal Language Converter</title>
<link rel="stylesheet" href="${owCss}">
<style>
  *{box-sizing:border-box;margin:0;padding:0}
  body{font-family:var(--ow-font-mono);background:var(--ow-bg-primary);color:var(--ow-text-primary);height:100vh;display:flex;flex-direction:column;overflow:hidden}
  #app{display:flex;flex-direction:column;height:100%;overflow:hidden}

  /* ── Header ── */
  .cc-header{display:flex;align-items:center;gap:12px;padding:10px 16px;background:var(--ow-bg-secondary);border-bottom:1px solid var(--ow-border);flex-shrink:0}
  .cc-logo{font-size:18px;font-weight:700;color:var(--ow-accent);letter-spacing:-0.5px}
  .cc-logo span{color:var(--ow-text-secondary);font-weight:400}
  .cc-tabs{display:flex;gap:2px;margin-left:auto}
  .cc-tab{padding:5px 14px;border-radius:var(--ow-radius-sm);border:none;background:transparent;color:var(--ow-text-secondary);cursor:pointer;font-size:12px;transition:all .15s}
  .cc-tab:hover{background:var(--ow-bg-hover);color:var(--ow-text-primary)}
  .cc-tab.active{background:var(--ow-accent);color:#fff}
  .theme-btn{padding:5px 10px;border-radius:var(--ow-radius-sm);border:1px solid var(--ow-border);background:var(--ow-bg-secondary);color:var(--ow-text-secondary);cursor:pointer;font-size:11px}

  /* ── Mode panels ── */
  .cc-mode{display:none;flex:1;overflow:hidden}
  .cc-mode.active{display:flex;flex-direction:column}

  /* ── Quick Convert ── */
  #mode-quick{flex-direction:row}
  .cc-pane{flex:1;display:flex;flex-direction:column;min-width:0}
  .cc-pane-header{display:flex;align-items:center;gap:8px;padding:8px 12px;background:var(--ow-bg-tertiary);border-bottom:1px solid var(--ow-border);flex-shrink:0}
  .cc-pane-title{font-size:11px;font-weight:600;text-transform:uppercase;letter-spacing:1px;color:var(--ow-text-secondary)}
  .lang-select{padding:3px 8px;border-radius:var(--ow-radius-sm);border:1px solid var(--ow-border);background:var(--ow-bg-secondary);color:var(--ow-text-primary);font-size:12px;min-width:140px;cursor:pointer}
  .cc-pane textarea,.cc-pane pre{flex:1;width:100%;border:none;resize:none;font-family:var(--ow-font-mono);font-size:12px;line-height:1.6;padding:12px;background:var(--ow-bg-primary);color:var(--ow-text-primary);outline:none;overflow:auto;margin:0;white-space:pre-wrap;word-break:break-word}
  .cc-pane pre{background:var(--ow-bg-secondary)}
  .cc-divider{width:1px;background:var(--ow-border);flex-shrink:0;display:flex;align-items:center;justify-content:center;position:relative}
  .swap-btn{position:absolute;z-index:10;padding:6px 8px;border-radius:50%;border:1px solid var(--ow-border);background:var(--ow-bg-secondary);color:var(--ow-text-secondary);cursor:pointer;font-size:14px;transition:all .15s}
  .swap-btn:hover{background:var(--ow-accent);color:#fff;border-color:var(--ow-accent)}

  /* ── Status bar ── */
  .cc-status{display:flex;align-items:center;gap:12px;padding:5px 12px;background:var(--ow-bg-tertiary);border-top:1px solid var(--ow-border);flex-shrink:0;font-size:11px}
  .status-item{display:flex;align-items:center;gap:4px;color:var(--ow-text-secondary)}
  .status-item b{color:var(--ow-text-primary)}
  .cc-confidence{display:flex;align-items:center;gap:4px}
  .confidence-bar{width:60px;height:4px;background:var(--ow-bg-hover);border-radius:2px;overflow:hidden}
  .confidence-fill{height:100%;background:var(--ow-success);border-radius:2px;transition:width .3s}

  /* ── Action strip ── */
  .cc-actions{display:flex;gap:6px;padding:6px 12px;background:var(--ow-bg-secondary);border-top:1px solid var(--ow-border);flex-shrink:0}
  .cc-btn{padding:5px 12px;border-radius:var(--ow-radius-sm);border:1px solid var(--ow-border);background:var(--ow-bg-hover);color:var(--ow-text-primary);cursor:pointer;font-size:11px;transition:all .15s}
  .cc-btn:hover{background:var(--ow-accent);color:#fff;border-color:var(--ow-accent)}
  .cc-btn.primary{background:var(--ow-accent);color:#fff;border-color:var(--ow-accent)}
  .cc-btn.primary:hover{opacity:.85}

  /* ── Widget Bridge callout ── */
  .widget-bridge-callout{display:none;margin:8px 12px;padding:10px 14px;background:rgba(99,102,241,.08);border:1px solid var(--ow-accent);border-radius:var(--ow-radius);font-size:12px}
  .widget-bridge-callout.visible{display:block}
  .widget-bridge-callout h4{color:var(--ow-accent);margin-bottom:6px;font-size:12px}

  /* ── Project mode ── */
  #mode-project{flex-direction:column;gap:0}
  .project-header{display:flex;gap:8px;padding:12px;background:var(--ow-bg-secondary);border-bottom:1px solid var(--ow-border);flex-shrink:0}
  .project-body{display:flex;flex:1;overflow:hidden}
  .file-tree{width:240px;border-right:1px solid var(--ow-border);overflow-y:auto;padding:8px}
  .file-item{padding:4px 8px;border-radius:var(--ow-radius-sm);cursor:pointer;font-size:12px;display:flex;align-items:center;gap:6px;color:var(--ow-text-secondary)}
  .file-item:hover{background:var(--ow-bg-hover);color:var(--ow-text-primary)}
  .file-item.ok::before{content:"✓";color:var(--ow-success)}
  .file-item.err::before{content:"✗";color:var(--ow-error)}
  .file-preview{flex:1;overflow:auto;padding:12px}
  .project-stats{display:flex;gap:16px;padding:8px 12px;background:var(--ow-bg-tertiary);border-top:1px solid var(--ow-border);font-size:11px;color:var(--ow-text-secondary)}
  .project-stat b{color:var(--ow-text-primary)}
  .progress-bar{flex:1;height:4px;background:var(--ow-bg-hover);border-radius:2px;overflow:hidden}
  .progress-fill{height:100%;background:var(--ow-accent);border-radius:2px;transition:width .3s}

  /* ── Explorer mode ── */
  #mode-explorer{flex-direction:column}
  .explorer-search{padding:10px 12px;border-bottom:1px solid var(--ow-border);flex-shrink:0}
  .explorer-search input{width:100%;padding:6px 10px;border-radius:var(--ow-radius-sm);border:1px solid var(--ow-border);background:var(--ow-bg-secondary);color:var(--ow-text-primary);font-size:13px;outline:none}
  .explorer-search input:focus{border-color:var(--ow-accent)}
  .lang-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:8px;padding:12px;overflow-y:auto;flex:1}
  .lang-card{padding:10px 12px;border-radius:var(--ow-radius);border:1px solid var(--ow-border);background:var(--ow-bg-secondary);cursor:pointer;transition:all .15s}
  .lang-card:hover{border-color:var(--ow-accent);background:var(--ow-bg-hover)}
  .lang-card-header{display:flex;align-items:center;gap:8px;margin-bottom:4px}
  .lang-dot{width:10px;height:10px;border-radius:50%;flex-shrink:0}
  .lang-card-name{font-weight:600;font-size:13px}
  .lang-card-family{font-size:10px;color:var(--ow-text-secondary);text-transform:uppercase;letter-spacing:0.5px}
  .lang-card-desc{font-size:11px;color:var(--ow-text-secondary);margin-top:4px;overflow:hidden;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical}
  .lang-card-year{font-size:10px;color:var(--ow-text-secondary);margin-top:4px}

  /* ── Notes strip ── */
  .cc-notes{display:none;padding:6px 12px;background:var(--ow-bg-secondary);border-top:1px solid var(--ow-border);font-size:11px;color:var(--ow-text-secondary);flex-shrink:0;max-height:80px;overflow-y:auto}
  .cc-notes.visible{display:block}
  .cc-note{padding:1px 0}
  .cc-note.warn{color:var(--ow-warning)}
  .cc-note.info{color:var(--ow-text-secondary)}

  /* Scrollbar */
  ::-webkit-scrollbar{width:6px;height:6px}
  ::-webkit-scrollbar-track{background:transparent}
  ::-webkit-scrollbar-thumb{background:var(--ow-border);border-radius:3px}
  ::-webkit-scrollbar-thumb:hover{background:var(--ow-text-secondary)}
</style>
</head>
<body>
<div id="app">

  <!-- Header -->
  <div class="cc-header">
    <div class="cc-logo">OmniCC <span>Universal Language Converter</span></div>
    <div style="display:flex;align-items:center;gap:6px;font-size:11px;color:var(--ow-text-secondary)">
      <span id="lang-count">0</span> languages
    </div>
    <div class="cc-tabs">
      <button class="cc-tab active" data-mode="quick">⚡ Quick Convert</button>
      <button class="cc-tab" data-mode="project">📁 Project</button>
      <button class="cc-tab" data-mode="explorer">🌐 Explorer</button>
    </div>
    <select id="theme-select" class="theme-btn">
      <option value="omni-dark">Omni Dark</option>
      <option value="omni-light">Omni Light</option>
      <option value="synthwave">Synthwave</option>
      <option value="forest">Forest</option>
      <option value="ocean">Ocean</option>
      <option value="sunset">Sunset</option>
    </select>
  </div>

  <!-- Quick Convert Mode -->
  <div id="mode-quick" class="cc-mode active">
    <div style="display:flex;flex:1;min-height:0">

      <!-- Source pane -->
      <div class="cc-pane">
        <div class="cc-pane-header">
          <span class="cc-pane-title">Source</span>
          <select id="src-lang" class="lang-select">
            <option value="auto">Auto Detect</option>
          </select>
          <span id="detect-badge" style="font-size:10px;color:var(--ow-text-secondary);display:none"></span>
        </div>
        <textarea id="src-input" placeholder="Paste any code here — 1000+ languages supported…" spellcheck="false"></textarea>
      </div>

      <!-- Divider + swap -->
      <div class="cc-divider">
        <button class="swap-btn" id="swap-btn" title="Swap source / target">⇄</button>
      </div>

      <!-- Output pane -->
      <div class="cc-pane">
        <div class="cc-pane-header">
          <span class="cc-pane-title">Output</span>
          <select id="tgt-lang" class="lang-select">
            <option value="javascript">JavaScript</option>
          </select>
        </div>
        <pre id="out-output" aria-live="polite">// Output will appear here…</pre>
      </div>
    </div>

    <!-- Widget Bridge callout -->
    <div class="widget-bridge-callout" id="bridge-callout">
      <h4>⚡ Widget Bridge Active</h4>
      <div id="bridge-detail">UI patterns detected — converted via Widget Bridge for maximum fidelity</div>
    </div>

    <!-- Status -->
    <div class="cc-status">
      <div class="status-item cc-confidence">
        <span>Confidence:</span>
        <div class="confidence-bar"><div class="confidence-fill" id="conf-fill" style="width:0%"></div></div>
        <b id="conf-text">—</b>
      </div>
      <div class="status-item"><span>Detected:</span><b id="status-src">—</b></div>
      <div class="status-item"><span>Target:</span><b id="status-tgt">—</b></div>
      <div class="status-item"><span>Units:</span><b id="status-units">—</b></div>
      <div class="status-item"><span>Lines:</span><b id="status-lines">—</b></div>
      <div class="status-item"><span>Time:</span><b id="status-time">—</b></div>
    </div>

    <!-- Actions -->
    <div class="cc-actions">
      <button class="cc-btn primary" id="btn-convert">Convert</button>
      <button class="cc-btn" id="btn-copy">Copy Output</button>
      <button class="cc-btn" id="btn-save">Save File</button>
      <button class="cc-btn" id="btn-editor">Open in Editor</button>
      <button class="cc-btn" id="btn-clear">Clear</button>
    </div>

    <!-- Notes -->
    <div class="cc-notes" id="quick-notes"></div>
  </div>

  <!-- Project Mode -->
  <div id="mode-project" class="cc-mode">
    <div class="project-header">
      <select id="proj-tgt-lang" class="lang-select">
        <option value="javascript">JavaScript</option>
      </select>
      <button class="cc-btn primary" id="btn-proj-open">📂 Open Folder</button>
      <button class="cc-btn" id="btn-proj-convert">⚡ Convert All</button>
      <button class="cc-btn" id="btn-proj-export" style="display:none">💾 Export</button>
      <div class="progress-bar" style="flex:1"><div class="progress-fill" id="proj-progress" style="width:0%"></div></div>
      <span id="proj-status" style="font-size:11px;color:var(--ow-text-secondary)">No folder selected</span>
    </div>
    <div class="project-body">
      <div class="file-tree" id="file-tree"><div style="padding:8px;font-size:12px;color:var(--ow-text-secondary)">Open a folder to see files</div></div>
      <div class="file-preview" id="file-preview"><pre style="font-size:12px;color:var(--ow-text-secondary)">Select a file to preview converted output</pre></div>
    </div>
    <div class="project-stats">
      <span><b id="pstat-files">0</b> files</span>
      <span><b id="pstat-ok">0</b> succeeded</span>
      <span><b id="pstat-lines">0</b> lines</span>
      <span><b id="pstat-time">—</b> ms</span>
    </div>
  </div>

  <!-- Explorer Mode -->
  <div id="mode-explorer" class="cc-mode">
    <div class="explorer-search">
      <input type="search" id="explorer-search" placeholder="Search 1000+ languages — try 'rust', 'functional', 'ML', 'ml'…" autocomplete="off">
    </div>
    <div class="lang-grid" id="lang-grid"></div>
  </div>

</div>

<script src="${owJs}"></script>
<script>
const vscode = acquireVsCodeApi();
let _output = '';
let _tgtExt = '.js';
let _projectFiles = [];
let _projectResults = [];
let _debounceTimer = null;

// ── OW init ───────────────────────────────────────────────────────────────────
(function owInit() {
  if (typeof OW === 'undefined') return;
  OW.setVscodeApi(vscode);
  try { var s = vscode.getState(); if (s && s.owTheme) { OW.switchTheme(s.owTheme); return; } } catch(e) {}
  OW.loadTheme();
})();

// ── Language registry ─────────────────────────────────────────────────────────
let _langs = [];
let _langsLoaded = false;

function loadLangs() {
  vscode.postMessage({ command: 'searchLangs', query: '' });
}

function populateLangSelects(langs) {
  _langs = langs;
  _langsLoaded = true;
  document.getElementById('lang-count').textContent = langs.length.toString();

  const selects = [document.getElementById('src-lang'), document.getElementById('tgt-lang'), document.getElementById('proj-tgt-lang')];
  for (const sel of selects) {
    if (!sel) continue;
    const hasAuto = sel.id === 'src-lang';
    const val = sel.value;
    sel.innerHTML = hasAuto ? '<option value="auto">Auto Detect</option>' : '';
    // Group by family
    const families = {};
    for (const l of langs) {
      if (!families[l.family]) families[l.family] = [];
      families[l.family].push(l);
    }
    for (const [fam, fLangs] of Object.entries(families).sort()) {
      const grp = document.createElement('optgroup');
      grp.label = fam.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
      for (const l of fLangs.sort((a,b) => (b.popularity||0)-(a.popularity||0))) {
        const opt = document.createElement('option');
        opt.value = l.id;
        opt.textContent = l.name;
        grp.appendChild(opt);
      }
      sel.appendChild(grp);
    }
    // Restore selection
    if (val && sel.querySelector('[value="' + val + '"]')) sel.value = val;
    else if (!hasAuto) sel.value = 'javascript';
  }
}

// ── Tabs ──────────────────────────────────────────────────────────────────────
document.querySelectorAll('.cc-tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.cc-tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.cc-mode').forEach(m => m.classList.remove('active'));
    tab.classList.add('active');
    const mode = tab.dataset.mode;
    document.getElementById('mode-' + mode).classList.add('active');
    if (mode === 'explorer') renderExplorer();
  });
});

// ── Theme ─────────────────────────────────────────────────────────────────────
document.getElementById('theme-select').addEventListener('change', e => {
  const theme = e.target.value;
  if (typeof OW !== 'undefined') OW.switchTheme(theme);
  vscode.postMessage({ command: 'owThemeChange', theme });
});

// ── Quick Convert ─────────────────────────────────────────────────────────────
const srcInput = document.getElementById('src-input');
const outOutput = document.getElementById('out-output');

srcInput.addEventListener('input', () => {
  clearTimeout(_debounceTimer);
  _debounceTimer = setTimeout(doConvert, 380);
});

document.getElementById('src-lang').addEventListener('change', doConvert);
document.getElementById('tgt-lang').addEventListener('change', () => {
  const tgtId = document.getElementById('tgt-lang').value;
  updateTgtExt(tgtId);
  doConvert();
});

document.getElementById('swap-btn').addEventListener('click', () => {
  const src = srcInput.value;
  const out = _output;
  const srcLang = document.getElementById('src-lang').value;
  const tgtLang = document.getElementById('tgt-lang').value;
  srcInput.value = out;
  document.getElementById('src-lang').value = tgtLang !== 'auto' ? tgtLang : srcLang;
  document.getElementById('tgt-lang').value = srcLang !== 'auto' ? srcLang : tgtLang;
  doConvert();
});

document.getElementById('btn-convert').addEventListener('click', doConvert);
document.getElementById('btn-copy').addEventListener('click', () => vscode.postMessage({ command: 'copyOutput', content: _output }));
document.getElementById('btn-save').addEventListener('click', () => vscode.postMessage({ command: 'saveOutput', content: _output, ext: _tgtExt }));
document.getElementById('btn-editor').addEventListener('click', () => vscode.postMessage({ command: 'openEditor', content: _output, ext: _tgtExt }));
document.getElementById('btn-clear').addEventListener('click', () => { srcInput.value = ''; outOutput.textContent = '// Output will appear here…'; setStatus({}); });

function doConvert() {
  const source = srcInput.value;
  if (!source.trim()) return;
  const srcLang = document.getElementById('src-lang').value;
  const tgtLang = document.getElementById('tgt-lang').value;
  vscode.postMessage({ command: 'convert', source, sourceLang: srcLang, targetLang: tgtLang });
}

function updateTgtExt(tgtId) {
  const EXT = {javascript:'.js',typescript:'.ts',python:'.py',java:'.java',kotlin:'.kt',csharp:'.cs',go:'.go',rust:'.rs',swift:'.swift',ruby:'.rb',lua:'.lua',php:'.php',html:'.html',css:'.css',scss:'.scss',sql:'.sql',json:'.json',yaml:'.yml',toml:'.toml',xml:'.xml',graphql:'.graphql',bash:'.sh',powershell:'.ps1',titan:'.titan',vera:'.vera',nexus:'.nexus',helix:'.helix',aether:'.aether',axiom:'.axiom',sylva:'.sylva'};
  _tgtExt = EXT[tgtId] || '.txt';
}

function setStatus(r) {
  document.getElementById('status-src').textContent = r.sourceLangId || '—';
  document.getElementById('status-tgt').textContent = r.targetLangId || '—';
  document.getElementById('status-units').textContent = r.unitCount != null ? r.unitCount : '—';
  document.getElementById('status-lines').textContent = r.linesConverted != null ? r.linesConverted : '—';
  document.getElementById('status-time').textContent = r.durationMs != null ? r.durationMs + 'ms' : '—';
  const conf = r.confidence || 0;
  document.getElementById('conf-fill').style.width = conf + '%';
  document.getElementById('conf-text').textContent = conf ? conf + '%' : '—';
  const confFill = document.getElementById('conf-fill');
  confFill.style.background = conf >= 80 ? 'var(--ow-success)' : conf >= 50 ? 'var(--ow-warning)' : 'var(--ow-error)';
  // Widget bridge callout
  const callout = document.getElementById('bridge-callout');
  if (r.widgetBridge && r.widgetBridge.detected) {
    callout.classList.add('visible');
    document.getElementById('bridge-detail').textContent =
      r.widgetBridge.convertedCount + '/' + r.widgetBridge.widgetCount + ' UI units converted via Widget Bridge (confidence: ' + r.widgetBridge.confidence + '%)';
  } else {
    callout.classList.remove('visible');
  }
}

function setNotes(notes) {
  const el = document.getElementById('quick-notes');
  if (!notes || notes.length === 0) { el.classList.remove('visible'); return; }
  el.classList.add('visible');
  el.innerHTML = notes.map(n => '<div class="cc-note info">' + escHtml(n) + '</div>').join('');
}

// ── Project Mode ──────────────────────────────────────────────────────────────
document.getElementById('btn-proj-open').addEventListener('click', () => {
  vscode.postMessage({ command: 'openProjectFolder' });
});

document.getElementById('btn-proj-convert').addEventListener('click', () => {
  if (_projectFiles.length === 0) { vscode.postMessage({ command: 'openProjectFolder' }); return; }
  const tgtLang = document.getElementById('proj-tgt-lang').value;
  document.getElementById('proj-progress').style.width = '5%';
  document.getElementById('proj-status').textContent = 'Converting…';
  vscode.postMessage({ command: 'convertProject', files: _projectFiles, targetLang: tgtLang });
});

document.getElementById('btn-proj-export').addEventListener('click', () => {
  if (_projectResults.length === 0) return;
  vscode.postMessage({ command: 'exportProject', results: _projectResults.map(r => ({ path: r.path, targetPath: r.targetPath, output: r.output })) });
});

function renderFileTree(results) {
  _projectResults = results;
  const tree = document.getElementById('file-tree');
  tree.innerHTML = results.map((r, i) =>
    '<div class="file-item ' + (r.error ? 'err' : 'ok') + '" data-idx="' + i + '">' +
    r.path.split(/[\\/]/).pop() +
    '</div>'
  ).join('');
  tree.querySelectorAll('.file-item').forEach(el => {
    el.addEventListener('click', () => {
      const r = _projectResults[parseInt(el.dataset.idx)];
      document.getElementById('file-preview').innerHTML =
        '<div style="font-size:11px;color:var(--ow-text-secondary);margin-bottom:8px">' + r.path + ' → ' + r.targetPath + '</div>' +
        '<pre style="font-size:12px;white-space:pre-wrap">' + escHtml(r.output || '') + '</pre>';
    });
  });
}

// ── Explorer Mode ─────────────────────────────────────────────────────────────
let _explorerRendered = false;
function renderExplorer() {
  if (_explorerRendered) return;
  _explorerRendered = true;
  vscode.postMessage({ command: 'searchLangs', query: '' });
}

document.getElementById('explorer-search').addEventListener('input', e => {
  vscode.postMessage({ command: 'searchLangs', query: e.target.value });
});

function renderLangGrid(langs) {
  const grid = document.getElementById('lang-grid');
  if (!langs || langs.length === 0) {
    grid.innerHTML = '<div style="padding:16px;color:var(--ow-text-secondary)">No languages found</div>';
    return;
  }
  grid.innerHTML = langs.map(l =>
    '<div class="lang-card" data-id="' + l.id + '">' +
      '<div class="lang-card-header">' +
        '<div class="lang-dot" style="background:' + (l.color || '#888') + '"></div>' +
        '<span class="lang-card-name">' + escHtml(l.name) + '</span>' +
      '</div>' +
      '<div class="lang-card-family">' + escHtml(l.family) + '</div>' +
      '<div class="lang-card-desc">' + escHtml(l.description || '') + '</div>' +
      '<div class="lang-card-year">' + (l.year ? 'Since ' + l.year : '') + '</div>' +
    '</div>'
  ).join('');
  grid.querySelectorAll('.lang-card').forEach(card => {
    card.addEventListener('click', () => {
      // Switch to quick mode with this lang as target
      document.querySelector('[data-mode="quick"]').click();
      const tgtSel = document.getElementById('tgt-lang');
      if (tgtSel.querySelector('[value="' + card.dataset.id + '"]')) {
        tgtSel.value = card.dataset.id;
        updateTgtExt(card.dataset.id);
        doConvert();
      }
    });
  });
}

// ── Message handler ───────────────────────────────────────────────────────────
window.addEventListener('message', e => {
  const msg = e.data;
  switch (msg.command) {
    case 'convertResult':
      if (msg.error) {
        outOutput.textContent = '// Error: ' + msg.error;
        break;
      }
      _output = msg.output || '';
      outOutput.textContent = _output;
      setStatus(msg);
      setNotes(msg.notes);
      updateTgtExt(msg.targetLangId || 'js');
      break;
    case 'detectResult':
      document.getElementById('detect-badge').style.display = 'inline';
      document.getElementById('detect-badge').textContent = msg.name + ' (' + msg.confidence + '%)';
      break;
    case 'searchLangsResult':
      populateLangSelects(msg.langs);
      renderLangGrid(msg.langs);
      break;
    case 'projectProgress':
      document.getElementById('proj-progress').style.width = ((msg.done / msg.total) * 100) + '%';
      document.getElementById('proj-status').textContent = msg.done + '/' + msg.total + ' converted';
      break;
    case 'projectFolderLoaded': {
      _projectFiles = msg.files || [];
      const folderName = (msg.folder || '').replace(/\\/g, '/').split('/').pop() || msg.folder;
      document.getElementById('proj-status').textContent = _projectFiles.length + ' files in ' + folderName;
      document.getElementById('proj-progress').style.width = '0%';
      document.getElementById('btn-proj-export').style.display = 'none';
      const tree = document.getElementById('file-tree');
      tree.innerHTML = _projectFiles.map(f =>
        '<div class="file-item ok" style="cursor:default;padding:4px 8px;font-size:11px;color:var(--ow-text-secondary)">' +
        escHtml((f.path || f).replace(/\\/g,'/').split('/').slice(-2).join('/')) + '</div>'
      ).join('');
      break;
    }
    case 'projectResult':
      if (msg.files) {
        renderFileTree(msg.files);
        document.getElementById('btn-proj-export').style.display = '';
      }
      document.getElementById('pstat-files').textContent = msg.fileCount || 0;
      document.getElementById('pstat-ok').textContent = msg.successCount || 0;
      document.getElementById('pstat-lines').textContent = (msg.totalLines || 0).toLocaleString();
      document.getElementById('pstat-time').textContent = msg.durationMs || 0;
      document.getElementById('proj-progress').style.width = '100%';
      document.getElementById('proj-status').textContent = msg.successCount + '/' + msg.fileCount + ' files succeeded';
      break;
    case 'owThemeSync':
      if (typeof OW !== 'undefined') {
        OW._owSyncing = true;
        OW.switchTheme(msg.theme);
        OW._owSyncing = false;
      }
      break;
  }
});

function escHtml(s) {
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

// ── Init ──────────────────────────────────────────────────────────────────────
loadLangs();
</script>
</body>
</html>`;
    }
}
