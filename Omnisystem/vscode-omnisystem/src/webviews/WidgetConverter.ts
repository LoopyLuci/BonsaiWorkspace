import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { convert, detectLanguage, renderOWPreview } from '../conversion/WidgetConversionEngine';
import { LANGUAGE_EXTENSIONS, LANGUAGE_LABELS } from '../conversion/WidgetIR';

// ─── Nonce helper ──────────────────────────────────────────────────────────────

function getNonce(): string {
    let t = '';
    const c = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) { t += c.charAt(Math.floor(Math.random() * c.length)); }
    return t;
}

// ─── WidgetConverterPanel ──────────────────────────────────────────────────────

export class WidgetConverterPanel {
    public static currentPanel: WidgetConverterPanel | undefined;
    public static readonly viewType = 'omnisystem.widgetConverter';

    public static postMessage(msg: object): void {
        WidgetConverterPanel.currentPanel?._panel.webview.postMessage(msg);
    }

    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];

    public static createOrShow(extensionUri: vscode.Uri): void {
        const column = vscode.window.activeTextEditor?.viewColumn ?? vscode.ViewColumn.One;
        if (WidgetConverterPanel.currentPanel) {
            WidgetConverterPanel.currentPanel._panel.reveal(column);
            return;
        }
        const panel = vscode.window.createWebviewPanel(
            WidgetConverterPanel.viewType,
            'Widget Converter',
            column,
            {
                enableScripts: true,
                localResourceRoots: [extensionUri],
                retainContextWhenHidden: true,
            }
        );
        WidgetConverterPanel.currentPanel = new WidgetConverterPanel(panel, extensionUri);
    }

    private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
        this._panel = panel;
        this._extensionUri = extensionUri;

        this._panel.iconPath = {
            light: vscode.Uri.joinPath(extensionUri, 'icons', 'titan-light.svg'),
            dark:  vscode.Uri.joinPath(extensionUri, 'icons', 'titan-dark.svg'),
        };

        this._update();
        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
        this._panel.onDidChangeViewState(
            () => { if (this._panel.visible) { this._update(); } },
            null, this._disposables
        );
        this._panel.webview.onDidReceiveMessage(
            (msg) => this._handleMessage(msg),
            null, this._disposables
        );
    }

    private async _handleMessage(msg: { command: string; [key: string]: unknown }): Promise<void> {
        switch (msg.command) {

            case 'convert': {
                const source = String(msg.source ?? '');
                const sourceLang = String(msg.sourceLang ?? 'auto');
                const targetLang = String(msg.targetLang ?? 'vera');
                const widgetName = String(msg.widgetName ?? '');

                const result = convert({ source, sourceLang, targetLang, widgetNameHint: widgetName });
                const previewHtml = renderOWPreview(result.widgetType, result.widgetName);

                this._panel.webview.postMessage({
                    type: 'conversionResult',
                    code:        result.code,
                    widgetType:  result.widgetType,
                    widgetName:  result.widgetName,
                    confidence:  result.confidence,
                    notes:       result.notes,
                    fileExt:     result.fileExtension,
                    previewHtml,
                });
                break;
            }

            case 'detectLang': {
                const source = String(msg.source ?? '');
                const detected = detectLanguage(source);
                this._panel.webview.postMessage({ type: 'langDetected', lang: detected });
                break;
            }

            case 'createFile': {
                const code    = String(msg.code ?? '');
                const langKey = String(msg.targetLang ?? 'vera');
                const wName   = String(msg.widgetName ?? 'Widget').replace(/[^A-Za-z0-9_-]/g, '');
                const ext     = LANGUAGE_EXTENSIONS[langKey as keyof typeof LANGUAGE_EXTENSIONS] ?? '.vera';
                const fileName = `${wName}${ext}`;

                const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri;
                const defaultUri = wsRoot
                    ? vscode.Uri.joinPath(wsRoot, fileName)
                    : vscode.Uri.file(path.join(require('os').homedir(), fileName));

                const saveUri = await vscode.window.showSaveDialog({
                    defaultUri,
                    filters: { 'All Files': ['*'] },
                    title: `Save ${fileName}`,
                    saveLabel: 'Create Widget File',
                });

                if (saveUri) {
                    try {
                        await vscode.workspace.fs.writeFile(saveUri, Buffer.from(code, 'utf8'));
                        const doc = await vscode.workspace.openTextDocument(saveUri);
                        await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
                        this._panel.webview.postMessage({ type: 'fileCreated', path: saveUri.fsPath });
                    } catch (err) {
                        vscode.window.showErrorMessage(`Widget Converter: Failed to create file: ${err}`);
                    }
                }
                break;
            }

            case 'copyCode': {
                const code = String(msg.code ?? '');
                await vscode.env.clipboard.writeText(code);
                this._panel.webview.postMessage({ type: 'codeCopied' });
                break;
            }

            case 'openInEditor': {
                const code    = String(msg.code ?? '');
                const langKey = String(msg.targetLang ?? 'vera');
                const ext     = LANGUAGE_EXTENSIONS[langKey as keyof typeof LANGUAGE_EXTENSIONS] ?? '.txt';
                const tmpPath = path.join(require('os').tmpdir(), `ow_widget_preview${ext}`);
                fs.writeFileSync(tmpPath, code, 'utf8');
                const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(tmpPath));
                await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
                break;
            }

            case 'owThemeChange': {
                if (typeof msg.theme === 'string') {
                    vscode.commands.executeCommand('omnisystem._broadcastTheme', msg.theme);
                }
                break;
            }
        }
    }

    private _update(): void {
        const webview = this._panel.webview;
        const cssUri = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.css'));
        const jsUri  = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.js'));
        this._panel.title = 'Widget Converter';
        this._panel.webview.html = this._getHtml(webview, cssUri.toString(), jsUri.toString());
    }

    private _getHtml(webview: vscode.Webview, cssUri: string, jsUri: string): string {
        const nonce = getNonce();
        const langOptions = Object.entries(LANGUAGE_LABELS)
            .map(([v, l]) => `<option value="${v}">${l}</option>`)
            .join('');

        const sourceLangOptions = `
          <option value="auto">Auto-detect</option>
          <option value="javascript">JavaScript</option>
          <option value="typescript">TypeScript</option>
          <option value="css">CSS</option>
          <option value="tauri">Tauri (HTML+JS)</option>
          <option value="python">Python GUI</option>
          <option value="vera">Vera (OW Component)</option>
          <option value="nexus">Nexus (OW Layout)</option>
          <option value="titan">Titan (OW Runtime)</option>`;

        const targetLangOptions = `
          <option value="vera">Vera (OW Component)</option>
          <option value="nexus">Nexus (OW Layout)</option>
          <option value="titan">Titan (OW Runtime)</option>
          <option value="javascript">JavaScript</option>
          <option value="typescript">TypeScript</option>
          <option value="css">CSS</option>
          <option value="tauri">Tauri (HTML+JS)</option>
          <option value="python">Python GUI</option>`;

        return /* html */ `<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src vscode-resource: 'unsafe-inline'; script-src vscode-resource: 'nonce-${nonce}';">
  <title>Widget Converter</title>
  <link rel="stylesheet" href="${cssUri}" />
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body {
      margin: 0; padding: 0;
      background: var(--ow-bg, #0A1628);
      color: var(--ow-text, #E0E0E0);
      font-family: 'Segoe UI', system-ui, sans-serif;
      font-size: 13px;
      display: flex;
      flex-direction: column;
      height: 100vh;
      overflow: hidden;
    }

    /* ── Header ── */
    .wc-header {
      display: flex;
      align-items: center;
      gap: 14px;
      padding: 12px 18px;
      background: var(--ow-bg-card, #0F1F3A);
      border-bottom: 1px solid var(--ow-border, #1E3A5F);
      flex-shrink: 0;
    }
    .wc-logo {
      width: 38px; height: 38px;
      background: linear-gradient(135deg, var(--ow-accent, #00D4FF), #0060FF);
      border-radius: 10px;
      display: flex; align-items: center; justify-content: center;
      font-size: 20px; flex-shrink: 0;
      box-shadow: 0 3px 14px rgba(0,212,255,0.3);
    }
    .wc-header-text h1 {
      color: var(--ow-accent, #00D4FF); font-size: 17px; font-weight: 700;
      margin: 0 0 2px; letter-spacing: -0.3px;
    }
    .wc-header-text p { color: var(--ow-text-dim, #5588AA); margin: 0; font-size: 11px; }
    .wc-header-right { margin-left: auto; display: flex; gap: 10px; align-items: center; }
    .wc-badge-group { display: flex; gap: 6px; }

    /* ── Toolbar ── */
    .wc-toolbar {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 18px;
      background: var(--ow-bg-raise, #0A1628);
      border-bottom: 1px solid var(--ow-border, #1E3A5F);
      flex-shrink: 0;
      flex-wrap: wrap;
    }
    .wc-lang-group { display: flex; align-items: center; gap: 8px; }
    .wc-lang-label { color: var(--ow-text-dim, #5588AA); font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.8px; white-space: nowrap; }
    .wc-lang-select {
      background: var(--ow-bg-card, #0F1F3A);
      color: var(--ow-text, #E0E0E0);
      border: 1px solid var(--ow-border, #1E3A5F);
      border-radius: 6px;
      padding: 6px 10px;
      font-size: 12px;
      cursor: pointer;
      outline: none;
      min-width: 160px;
    }
    .wc-lang-select:focus { border-color: var(--ow-border-focus, rgba(0,212,255,0.5)); }
    .wc-swap-btn {
      background: var(--ow-bg-card, #0F1F3A);
      border: 1px solid var(--ow-border, #1E3A5F);
      color: var(--ow-text-dim, #5588AA);
      border-radius: 6px;
      padding: 6px 12px;
      cursor: pointer;
      font-size: 16px;
      transition: all 0.15s;
    }
    .wc-swap-btn:hover { color: var(--ow-accent, #00D4FF); border-color: var(--ow-accent, #00D4FF); }
    .wc-toolbar-actions { margin-left: auto; display: flex; gap: 8px; }

    /* ── Widget name input ── */
    .wc-name-input {
      background: var(--ow-bg-card, #0F1F3A);
      color: var(--ow-text, #E0E0E0);
      border: 1px solid var(--ow-border, #1E3A5F);
      border-radius: 6px;
      padding: 6px 10px;
      font-size: 12px;
      width: 130px;
      outline: none;
    }
    .wc-name-input:focus { border-color: var(--ow-border-focus, rgba(0,212,255,0.5)); }
    .wc-name-input::placeholder { color: var(--ow-text-muted, #3A6A8F); }

    /* ── Split pane ── */
    .wc-split {
      display: flex;
      flex: 1;
      min-height: 0;
      gap: 0;
      overflow: hidden;
    }
    .wc-pane {
      display: flex;
      flex-direction: column;
      flex: 1;
      min-width: 0;
      overflow: hidden;
    }
    .wc-pane-divider {
      width: 3px;
      background: var(--ow-border, #1E3A5F);
      flex-shrink: 0;
      cursor: col-resize;
      transition: background 0.15s;
    }
    .wc-pane-divider:hover { background: var(--ow-accent, #00D4FF); }
    .wc-pane-header {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 8px 14px;
      background: var(--ow-bg-card, #0F1F3A);
      border-bottom: 1px solid var(--ow-border, #1E3A5F);
      flex-shrink: 0;
    }
    .wc-pane-title { color: var(--ow-text, #E0E0E0); font-weight: 600; font-size: 12px; }
    .wc-pane-stats { color: var(--ow-text-dim, #5588AA); font-size: 11px; margin-left: auto; }
    .wc-pane-actions { display: flex; gap: 6px; margin-left: auto; }
    .wc-editor {
      flex: 1;
      width: 100%;
      background: var(--ow-bg, #0A1628);
      color: var(--ow-text, #E0E0E0);
      border: none;
      outline: none;
      padding: 14px;
      font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
      font-size: 12px;
      line-height: 1.6;
      resize: none;
      tab-size: 2;
    }
    .wc-editor::placeholder { color: var(--ow-text-muted, #3A6A8F); }
    .wc-output-scroll {
      flex: 1;
      overflow: auto;
      background: var(--ow-bg, #0A1628);
    }
    .wc-code-display {
      margin: 0;
      padding: 14px;
      font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
      font-size: 12px;
      line-height: 1.6;
      white-space: pre-wrap;
      word-break: break-all;
      color: var(--ow-text, #E0E0E0);
    }
    .wc-code-display code { color: inherit; }

    /* ── Detected type badge ── */
    .wc-detected {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 6px 14px;
      background: var(--ow-bg-card, #0F1F3A);
      border-bottom: 1px solid var(--ow-border, #1E3A5F);
      flex-shrink: 0;
    }
    .wc-det-label { color: var(--ow-text-dim, #5588AA); font-size: 11px; }

    /* ── Notes strip ── */
    .wc-notes {
      padding: 6px 14px;
      background: var(--ow-bg-raise, #0D1926);
      border-top: 1px solid var(--ow-border, #1E3A5F);
      border-bottom: 1px solid var(--ow-border, #1E3A5F);
      font-size: 11px;
      color: var(--ow-text-dim, #5588AA);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      flex-shrink: 0;
      display: none;
    }
    .wc-notes.visible { display: block; }

    /* ── Bottom area (preview + log) ── */
    .wc-bottom {
      display: flex;
      border-top: 1px solid var(--ow-border, #1E3A5F);
      max-height: 200px;
      flex-shrink: 0;
    }
    .wc-preview {
      flex: 1;
      display: flex;
      flex-direction: column;
      min-width: 0;
      border-right: 1px solid var(--ow-border, #1E3A5F);
    }
    .wc-preview-header {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 6px 14px;
      background: var(--ow-bg-card, #0F1F3A);
      border-bottom: 1px solid var(--ow-border, #1E3A5F);
      flex-shrink: 0;
      font-size: 11px;
      color: var(--ow-text-dim, #5588AA);
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.8px;
    }
    .wc-preview-canvas {
      flex: 1;
      overflow: auto;
      padding: 16px;
      display: flex;
      align-items: flex-start;
      gap: 12px;
      flex-wrap: wrap;
      background: var(--ow-bg, #0A1628);
    }
    .wc-preview-empty { color: var(--ow-text-muted, #3A6A8F); font-size: 12px; font-style: italic; }

    .wc-log {
      width: 280px;
      flex-shrink: 0;
      display: flex;
      flex-direction: column;
    }
    .wc-log-header {
      padding: 6px 14px;
      background: var(--ow-bg-card, #0F1F3A);
      border-bottom: 1px solid var(--ow-border, #1E3A5F);
      font-size: 11px;
      color: var(--ow-text-dim, #5588AA);
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.8px;
      flex-shrink: 0;
    }
    .wc-log-scroll {
      flex: 1;
      overflow-y: auto;
      padding: 8px;
      background: var(--ow-bg, #0A1628);
      font-family: 'Cascadia Code', 'Courier New', monospace;
      font-size: 10.5px;
      color: var(--ow-text-dim, #5588AA);
      line-height: 1.5;
    }
    .wc-log-line { padding: 1px 0; }
    .wc-log-line.ok { color: var(--ow-success, #00FF88); }
    .wc-log-line.err { color: var(--ow-danger, #FF4444); }

    /* ── Converting spinner ── */
    .wc-converting {
      display: none;
      align-items: center;
      gap: 6px;
      color: var(--ow-success, #00FF88);
      font-size: 11px;
    }
    .wc-converting.active { display: flex; }
    .wc-spin {
      width: 10px; height: 10px;
      border: 2px solid var(--ow-border, #1E3A5F);
      border-top-color: var(--ow-success, #00FF88);
      border-radius: 50%;
      animation: spin 0.7s linear infinite;
    }
    @keyframes spin { to { transform: rotate(360deg); } }

    /* Scrollbar */
    ::-webkit-scrollbar { width: 5px; height: 5px; }
    ::-webkit-scrollbar-track { background: transparent; }
    ::-webkit-scrollbar-thumb { background: var(--ow-border, #1E3A5F); border-radius: 4px; }
    ::-webkit-scrollbar-thumb:hover { background: var(--ow-text-dim, #5588AA); }

    /* Confidence colors */
    .conf-high   { color: var(--ow-success, #00FF88); }
    .conf-medium { color: var(--ow-warning, #FFB800); }
    .conf-low    { color: var(--ow-danger, #FF4444); }
  </style>
</head>
<body>

  <!-- Header -->
  <div class="wc-header">
    <div class="wc-logo">⇄</div>
    <div class="wc-header-text">
      <h1>Widget Converter</h1>
      <p>Live conversion between Omnisystem Languages ↔ JavaScript · TypeScript · CSS · Tauri · Python GUI</p>
    </div>
    <div class="wc-header-right">
      <div class="wc-converting" id="converting-indicator">
        <div class="wc-spin"></div>
        <span>Converting...</span>
      </div>
      <span class="ow-badge" id="widget-type-badge" style="display:none">—</span>
      <span class="ow-badge" id="confidence-badge" style="display:none">—</span>
      <button class="ow-btn ow-btn-ghost" onclick="openThemePicker()" title="Switch Theme" style="padding:6px 10px">🎨</button>
    </div>
  </div>

  <!-- Toolbar -->
  <div class="wc-toolbar">
    <div class="wc-lang-group">
      <span class="wc-lang-label">From</span>
      <select id="source-lang" class="wc-lang-select" onchange="onLangChange()">
        ${sourceLangOptions}
      </select>
    </div>

    <button class="wc-swap-btn" onclick="swapLanguages()" title="Swap source ↔ target">⇄</button>

    <div class="wc-lang-group">
      <span class="wc-lang-label">To</span>
      <select id="target-lang" class="wc-lang-select" onchange="onLangChange()">
        ${targetLangOptions}
      </select>
    </div>

    <div class="wc-lang-group">
      <span class="wc-lang-label">Name</span>
      <input id="widget-name-input" class="wc-name-input" placeholder="Widget name (optional)" oninput="scheduleConvert()" />
    </div>

    <div class="wc-toolbar-actions">
      <button class="ow-btn ow-btn-primary" onclick="doConvert()" id="convert-btn">Convert ⇄</button>
      <button class="ow-btn ow-btn-ghost" onclick="clearAll()">Clear</button>
    </div>
  </div>

  <!-- Split pane -->
  <div class="wc-split">
    <!-- Input pane -->
    <div class="wc-pane">
      <div class="wc-pane-header">
        <span class="wc-pane-title">Source Code</span>
        <span class="wc-pane-stats" id="input-stats">0 lines · 0 chars</span>
      </div>
      <textarea
        id="source-input"
        class="wc-editor"
        placeholder="Paste widget code here — JavaScript, TypeScript, CSS, Tauri, Python GUI, Vera, Nexus, or Titan..."
        oninput="onSourceInput()"
        spellcheck="false"
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
      ></textarea>
    </div>

    <div class="wc-pane-divider"></div>

    <!-- Output pane -->
    <div class="wc-pane">
      <div class="wc-pane-header">
        <span class="wc-pane-title">Converted Output</span>
        <span class="wc-pane-stats" id="output-stats">—</span>
        <div class="wc-pane-actions">
          <button class="ow-btn ow-btn-secondary" id="copy-btn" onclick="copyOutput()" disabled style="font-size:11px;padding:4px 10px">Copy</button>
          <button class="ow-btn ow-btn-ghost" id="open-btn" onclick="openInEditor()" disabled style="font-size:11px;padding:4px 10px">Open</button>
          <button class="ow-btn ow-btn-primary" id="create-btn" onclick="createFile()" disabled style="font-size:11px;padding:4px 10px">Create File</button>
        </div>
      </div>
      <div class="wc-output-scroll" id="output-scroll">
        <pre class="wc-code-display" id="output-display"><code id="output-code">// Converted output appears here.
// Paste widget code on the left to begin.</code></pre>
      </div>
    </div>
  </div>

  <!-- Notes strip -->
  <div class="wc-notes" id="notes-strip"></div>

  <!-- Bottom: preview + log -->
  <div class="wc-bottom">
    <div class="wc-preview">
      <div class="wc-preview-header">
        ⬡ Live Widget Preview
        <span id="preview-label" style="margin-left:auto;color:var(--ow-text-muted);font-size:10px;text-transform:none;letter-spacing:0">waiting for conversion...</span>
      </div>
      <div class="wc-preview-canvas" id="preview-canvas">
        <span class="wc-preview-empty">Widget preview will render here after conversion</span>
      </div>
    </div>

    <div class="wc-log">
      <div class="wc-log-header">Conversion Log</div>
      <div class="wc-log-scroll" id="log-scroll">
        <div class="wc-log-line">Ready — paste widget code to convert</div>
      </div>
    </div>
  </div>

  <script src="${jsUri}"></script>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();

    // ── OW init (theme sync) ───────────────────────────────────────────────
    (function owInit() {
      if (typeof OW === 'undefined') return;
      OW.setVscodeApi(vscode);
      try {
        var s = vscode.getState();
        if (s && s.owTheme) { OW.switchTheme(s.owTheme); return; }
      } catch(e){}
      OW.loadTheme();
    })();

    function openThemePicker() {
      if (typeof OW === 'undefined') return;
      var body = OW.themePicker({ onchange: function(id) { OW.switchTheme(id); persistTheme(id); } });
      var m = OW.modal({ title: '🎨 Choose Theme', body: body, size: 'sm',
        buttons: [{ label: 'Close', variant: 'ghost', onclick: function() { m.close(); } }] });
    }

    function persistTheme(id) {
      try { vscode.setState(Object.assign(vscode.getState() || {}, { owTheme: id })); } catch(e){}
    }

    // ── State ─────────────────────────────────────────────────────────────
    let currentOutput = '';
    let currentWidgetName = 'Widget';
    let currentTargetLang = 'vera';
    let debounceTimer = null;
    const DEBOUNCE_MS = 350;

    // ── Source input ──────────────────────────────────────────────────────
    function onSourceInput() {
      const src = document.getElementById('source-input').value;
      const lines = src.split('\n').length;
      const chars = src.length;
      document.getElementById('input-stats').textContent = lines + ' lines · ' + chars + ' chars';
      scheduleConvert();

      // Auto-detect language if 'auto' selected
      const srcLangSel = document.getElementById('source-lang');
      if (srcLangSel.value === 'auto' && src.trim().length > 30) {
        vscode.postMessage({ command: 'detectLang', source: src });
      }
    }

    function scheduleConvert() {
      clearTimeout(debounceTimer);
      const src = document.getElementById('source-input').value;
      if (src.trim().length < 8) return;
      debounceTimer = setTimeout(() => doConvert(), DEBOUNCE_MS);
    }

    function onLangChange() {
      const src = document.getElementById('source-input').value;
      if (src.trim().length > 8) { doConvert(); }
    }

    function doConvert() {
      const source   = document.getElementById('source-input').value;
      const srcLang  = document.getElementById('source-lang').value;
      const tgtLang  = document.getElementById('target-lang').value;
      const wName    = document.getElementById('widget-name-input').value.trim();

      if (!source.trim()) { return; }

      currentTargetLang = tgtLang;
      document.getElementById('converting-indicator').classList.add('active');
      document.getElementById('convert-btn').disabled = true;

      vscode.postMessage({ command: 'convert', source, sourceLang: srcLang, targetLang: tgtLang, widgetName: wName });
    }

    function swapLanguages() {
      const srcSel = document.getElementById('source-lang');
      const tgtSel = document.getElementById('target-lang');
      const oldSrc = srcSel.value === 'auto' ? 'javascript' : srcSel.value;
      const oldTgt = tgtSel.value;

      // Find the option in source that matches old target (or vice versa)
      const matchSrc = Array.from(srcSel.options).find(o => o.value === oldTgt);
      const matchTgt = Array.from(tgtSel.options).find(o => o.value === oldSrc);

      if (matchSrc) srcSel.value = oldTgt;
      if (matchTgt) tgtSel.value = oldSrc;

      // Also swap content if output is not empty
      if (currentOutput) {
        const input = document.getElementById('source-input');
        input.value = currentOutput;
        onSourceInput();
      }
      doConvert();
    }

    function clearAll() {
      document.getElementById('source-input').value = '';
      document.getElementById('output-code').textContent = '// Cleared — paste new code to convert';
      document.getElementById('input-stats').textContent = '0 lines · 0 chars';
      document.getElementById('output-stats').textContent = '—';
      document.getElementById('preview-canvas').innerHTML = '<span class="wc-preview-empty">Widget preview will render here after conversion</span>';
      document.getElementById('preview-label').textContent = 'waiting for conversion...';
      document.getElementById('notes-strip').textContent = '';
      document.getElementById('notes-strip').classList.remove('visible');
      document.getElementById('widget-type-badge').style.display = 'none';
      document.getElementById('confidence-badge').style.display = 'none';
      document.getElementById('copy-btn').disabled = true;
      document.getElementById('open-btn').disabled = true;
      document.getElementById('create-btn').disabled = true;
      currentOutput = '';
      appendLog('Cleared.');
    }

    // ── Output actions ────────────────────────────────────────────────────
    function copyOutput() {
      if (!currentOutput) return;
      vscode.postMessage({ command: 'copyCode', code: currentOutput });
    }

    function openInEditor() {
      if (!currentOutput) return;
      vscode.postMessage({ command: 'openInEditor', code: currentOutput, targetLang: currentTargetLang });
    }

    function createFile() {
      if (!currentOutput) return;
      vscode.postMessage({
        command: 'createFile',
        code: currentOutput,
        targetLang: currentTargetLang,
        widgetName: currentWidgetName,
      });
    }

    // ── Log ───────────────────────────────────────────────────────────────
    function appendLog(text, cls) {
      const scroll = document.getElementById('log-scroll');
      const line = document.createElement('div');
      line.className = 'wc-log-line' + (cls ? ' ' + cls : '');
      line.textContent = text;
      scroll.appendChild(line);
      // Keep last 60 lines
      while (scroll.children.length > 60) { scroll.removeChild(scroll.firstChild); }
      scroll.scrollTop = scroll.scrollHeight;
    }

    // ── Message handler ───────────────────────────────────────────────────
    window.addEventListener('message', event => {
      const msg = event.data;

      if (msg.type === 'conversionResult') {
        document.getElementById('converting-indicator').classList.remove('active');
        document.getElementById('convert-btn').disabled = false;

        currentOutput = msg.code;
        currentWidgetName = msg.widgetName;

        // Output display
        document.getElementById('output-code').textContent = msg.code || '// No output generated';

        // Stats
        const outLines = (msg.code || '').split('\\n').length;
        const outChars = (msg.code || '').length;
        document.getElementById('output-stats').textContent = outLines + ' lines · ' + outChars + ' chars';

        // Badges
        const typeBadge = document.getElementById('widget-type-badge');
        typeBadge.textContent = msg.widgetType || 'unknown';
        typeBadge.style.display = 'inline';
        const confBadge = document.getElementById('confidence-badge');
        confBadge.textContent = (msg.confidence || 'low') + ' confidence';
        confBadge.className = 'ow-badge conf-' + (msg.confidence || 'low');
        confBadge.style.display = 'inline';

        // Notes strip
        const notesEl = document.getElementById('notes-strip');
        if (msg.notes && msg.notes.length > 0) {
          notesEl.textContent = 'ℹ ' + msg.notes.join('  ·  ');
          notesEl.classList.add('visible');
        } else {
          notesEl.classList.remove('visible');
        }

        // Preview
        const canvas = document.getElementById('preview-canvas');
        if (msg.previewHtml) {
          canvas.innerHTML = msg.previewHtml;
          document.getElementById('preview-label').textContent =
            (msg.widgetType || 'widget') + ' — ' + msg.widgetName;
        }

        // Enable buttons
        const hasOutput = !!msg.code;
        document.getElementById('copy-btn').disabled = !hasOutput;
        document.getElementById('open-btn').disabled = !hasOutput;
        document.getElementById('create-btn').disabled = !hasOutput;

        // Log
        const confEmoji = msg.confidence === 'high' ? '✓' : msg.confidence === 'medium' ? '~' : '?';
        appendLog(confEmoji + ' ' + (msg.widgetType || '?') + ' → ' + currentTargetLang + ' [' + msg.confidence + ']',
                  msg.confidence === 'high' ? 'ok' : '');
        if (msg.notes) {
          for (const note of msg.notes.slice(0, 3)) { appendLog('  · ' + note); }
        }
      }

      if (msg.type === 'langDetected') {
        const srcSel = document.getElementById('source-lang');
        if (srcSel.value === 'auto') {
          appendLog('Auto-detected: ' + msg.lang);
        }
      }

      if (msg.type === 'codeCopied') {
        appendLog('✓ Copied to clipboard', 'ok');
        const btn = document.getElementById('copy-btn');
        const prev = btn.textContent;
        btn.textContent = '✓ Copied!';
        setTimeout(() => { btn.textContent = prev; }, 1500);
      }

      if (msg.type === 'fileCreated') {
        appendLog('✓ File created: ' + msg.path, 'ok');
      }

      // OW theme sync
      if (msg.type === 'owThemeSync' && msg.theme && typeof OW !== 'undefined') {
        OW._owSyncing = true;
        OW.switchTheme(msg.theme);
        OW._owSyncing = false;
        persistTheme(msg.theme);
      }
    });
  </script>
</body>
</html>`;
    }

    public dispose(): void {
        WidgetConverterPanel.currentPanel = undefined;
        this._panel.dispose();
        while (this._disposables.length) {
            const d = this._disposables.pop();
            if (d) { d.dispose(); }
        }
    }
}
