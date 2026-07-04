"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.LayoutPreviewPanel = void 0;
const vscode = __importStar(require("vscode"));
// ─── Utilities ────────────────────────────────────────────────────────────────
function getNonce() {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}
function escapeHtml(str) {
    return str
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
}
function parseNexusLayout(source) {
    // Simple heuristic parser — extracts named containers and grids
    const root = { type: 'container', label: 'Root', children: [] };
    const gridMatch = source.match(/grid\s*\{[^}]*columns:\s*(\d+)/i);
    const flexMatch = source.match(/flex\s*\{[^}]*direction:\s*(\w+)/i);
    if (gridMatch) {
        root.type = 'grid';
        root.columns = parseInt(gridMatch[1]) || 3;
    }
    else if (flexMatch) {
        root.type = 'flex';
        root.direction = flexMatch[1] === 'column' ? 'column' : 'row';
    }
    // Find named blocks
    const blockRe = /(\w+)\s+(\w+)\s*\{([^}]*)\}/g;
    let m;
    const colors = ['#00D4FF', '#FF6B9D', '#FFB800', '#00FF88', '#DDA0DD', '#FF8C42', '#87CEEB', '#98FB98'];
    let colorIdx = 0;
    while ((m = blockRe.exec(source)) !== null) {
        const kind = m[1].toLowerCase();
        const name = m[2];
        const body = m[3];
        if (['layout', 'root', 'app'].includes(kind))
            continue;
        const spanMatch = body.match(/span:\s*(\d+)/);
        const child = {
            type: kind,
            id: name,
            label: name,
            color: colors[colorIdx++ % colors.length],
            children: [],
            span: spanMatch ? parseInt(spanMatch[1]) : 1,
        };
        root.children.push(child);
    }
    // If no children were parsed, add some demo ones
    if (root.children.length === 0) {
        const demos = [
            { label: 'Header', color: '#00D4FF', span: root.columns || 1 },
            { label: 'Sidebar', color: '#FFB800' },
            { label: 'Main Content', color: '#00FF88', span: (root.columns || 3) - 1 },
            { label: 'Footer', color: '#FF6B9D', span: root.columns || 1 },
        ];
        demos.forEach(d => {
            root.children.push({
                type: 'box', label: d.label, color: d.color,
                children: [], span: d.span || 1,
            });
        });
    }
    return root;
}
// ─── LayoutPreviewPanel ───────────────────────────────────────────────────────
class LayoutPreviewPanel {
    static postMessage(msg) {
        LayoutPreviewPanel.currentPanel?._panel.webview.postMessage(msg);
    }
    static createOrShow(extensionUri) {
        const column = vscode.window.activeTextEditor
            ? vscode.window.activeTextEditor.viewColumn
            : undefined;
        if (LayoutPreviewPanel.currentPanel) {
            LayoutPreviewPanel.currentPanel._panel.reveal(column);
            LayoutPreviewPanel.currentPanel._loadActiveLayout();
            return;
        }
        const panel = vscode.window.createWebviewPanel(LayoutPreviewPanel.viewType, 'NEXUS Layout Preview', column || vscode.ViewColumn.Two, {
            enableScripts: true,
            localResourceRoots: [extensionUri],
            retainContextWhenHidden: true,
        });
        LayoutPreviewPanel.currentPanel = new LayoutPreviewPanel(panel, extensionUri);
    }
    constructor(panel, extensionUri) {
        this._disposables = [];
        this._panel = panel;
        this._extensionUri = extensionUri;
        this._panel.iconPath = {
            light: vscode.Uri.joinPath(extensionUri, 'icons', 'nexus-light.svg'),
            dark: vscode.Uri.joinPath(extensionUri, 'icons', 'nexus-dark.svg'),
        };
        this._update();
        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
        this._panel.webview.onDidReceiveMessage((msg) => this._handleMessage(msg), null, this._disposables);
        const editorChange = vscode.window.onDidChangeActiveTextEditor(() => {
            this._loadActiveLayout();
        });
        this._disposables.push(editorChange);
        const saveDisposable = vscode.workspace.onDidSaveTextDocument((doc) => {
            if (doc.languageId === 'nexus' || doc.fileName.endsWith('.nexus')) {
                this._loadActiveLayout();
            }
        });
        this._disposables.push(saveDisposable);
        // Also track text changes for live preview
        const textChange = vscode.workspace.onDidChangeTextDocument((event) => {
            const doc = event.document;
            if (doc.languageId === 'nexus' || doc.fileName.endsWith('.nexus')) {
                const editor = vscode.window.activeTextEditor;
                if (editor && editor.document === doc) {
                    this._loadActiveLayout();
                }
            }
        });
        this._disposables.push(textChange);
        this._loadActiveLayout();
    }
    _post(msg) {
        this._panel.webview.postMessage(msg);
    }
    _loadActiveLayout() {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            this._post({ type: 'noLayout' });
            return;
        }
        const doc = editor.document;
        const isNexus = doc.languageId === 'nexus' || doc.fileName.endsWith('.nexus');
        if (!isNexus) {
            this._post({ type: 'noLayout' });
            return;
        }
        const source = doc.getText();
        const fileName = doc.fileName.replace(/\\/g, '/').split('/').pop() || 'unknown.nexus';
        const layout = parseNexusLayout(source);
        const analysis = this._analyzeLayout(source);
        this._post({ type: 'layoutLoaded', source, fileName, layout, analysis });
    }
    _analyzeLayout(source) {
        const gridCount = (source.match(/\bgrid\b/gi) || []).length;
        const flexCount = (source.match(/\bflex\b/gi) || []).length;
        const containerCount = (source.match(/\bcontainer\b/gi) || []).length;
        const responsiveRules = (source.match(/@media|breakpoint|responsive/gi) || []).length;
        const lines = source.split('\n').length;
        let complexity = 'Simple';
        if (gridCount + flexCount > 5 || responsiveRules > 3)
            complexity = 'Complex';
        else if (gridCount + flexCount > 2 || responsiveRules > 0)
            complexity = 'Moderate';
        return {
            gridCount, flexCount, containerCount, responsiveRules, lines, complexity,
        };
    }
    async _handleMessage(message) {
        switch (message.command) {
            case 'loadActive':
                this._loadActiveLayout();
                break;
            case 'openFile': {
                const uris = await vscode.window.showOpenDialog({
                    canSelectMany: false,
                    filters: { 'NEXUS Layouts': ['nexus'] },
                    title: 'Open NEXUS Layout',
                });
                if (uris && uris[0]) {
                    const doc = await vscode.workspace.openTextDocument(uris[0]);
                    await vscode.window.showTextDocument(doc, vscode.ViewColumn.One);
                    this._loadActiveLayout();
                }
                break;
            }
            case 'refresh':
                this._loadActiveLayout();
                break;
        }
    }
    _update() {
        const webview = this._panel.webview;
        const widgetStyleUri = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.css'));
        const widgetScriptUri = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.js'));
        this._panel.title = 'NEXUS Layout Preview';
        this._panel.webview.html = this._getHtmlForWebview(this._panel.webview, widgetStyleUri.toString(), widgetScriptUri.toString());
    }
    _getHtmlForWebview(webview, widgetStyleUri, widgetScriptUri) {
        const nonce = getNonce();
        return /* html */ `<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src vscode-resource: 'unsafe-inline'; script-src vscode-resource: 'nonce-${nonce}' https:;">
  <title>NEXUS Layout Preview</title>
  <link rel="stylesheet" href="${widgetStyleUri}"/>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body {
      background: #0A1628; color: #E0E0E0;
      font-family: 'Segoe UI', system-ui, sans-serif;
      margin: 0; padding: 20px; min-height: 100vh;
    }
    .header {
      display: flex; align-items: center; gap: 16px;
      margin-bottom: 20px; padding-bottom: 16px;
      border-bottom: 1px solid #1E3A5F;
    }
    .logo {
      width: 46px; height: 46px;
      background: linear-gradient(135deg, #98FB98, #2E8B57);
      border-radius: 12px;
      display: flex; align-items: center; justify-content: center;
      font-size: 22px; box-shadow: 0 4px 20px rgba(152,251,152,0.25);
    }
    h1 { color: #00D4FF; font-size: 22px; font-weight: 700; margin: 0 0 3px; }
    .subtitle { color: #5588AA; font-size: 12px; margin: 0; }
    .toolbar {
      display: flex; align-items: center; gap: 10px;
      margin-bottom: 16px; flex-wrap: wrap;
    }
    .bp-group {
      display: flex; border: 1px solid #1E3A5F; border-radius: 8px;
      overflow: hidden; flex-shrink: 0;
    }
    .bp-btn {
      background: transparent; color: #88AACC; border: none;
      padding: 7px 14px; cursor: pointer; font-size: 12px; font-weight: 600;
      transition: all 0.15s; white-space: nowrap;
      display: flex; align-items: center; gap: 6px;
    }
    .bp-btn:hover { background: #0F2A4A; color: #00D4FF; }
    .bp-btn.active { background: #0F2A4A; color: #00D4FF; border-bottom: 2px solid #00D4FF; }
    .btn {
      background: linear-gradient(135deg, #00D4FF, #0090CC);
      color: #0A1628; border: none; border-radius: 7px;
      padding: 7px 16px; cursor: pointer; font-weight: 700;
      font-size: 12px; transition: all 0.15s; white-space: nowrap;
    }
    .btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .btn-secondary {
      background: transparent; color: #00D4FF; border: 1px solid #1E5A7F;
    }
    .btn-secondary:hover { background: #0F2A4A; }
    .dimension-badge {
      background: #0F1F3A; border: 1px solid #1E3A5F; border-radius: 6px;
      padding: 5px 12px; font-family: monospace; font-size: 12px; color: #FFB800;
      margin-left: auto;
    }
    /* Main layout */
    .main-layout {
      display: grid;
      grid-template-columns: 340px 1fr;
      gap: 18px;
      height: calc(100vh - 160px);
      min-height: 500px;
    }
    @media (max-width: 900px) { .main-layout { grid-template-columns: 1fr; height: auto; } }
    .card {
      background: #0F1F3A; border: 1px solid #1E3A5F;
      border-radius: 12px; padding: 18px;
      display: flex; flex-direction: column; overflow: hidden;
    }
    .card h3 { color: #00D4FF; font-size: 14px; font-weight: 600; margin: 0 0 12px; flex-shrink: 0; }
    /* Source code */
    .source-code {
      background: #050D1A; border: 1px solid #0F1E30;
      border-radius: 8px; padding: 14px;
      font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
      font-size: 11px; line-height: 1.7; overflow: auto; flex: 1;
      color: #AAD4EE; white-space: pre; tab-size: 2;
    }
    .source-code::-webkit-scrollbar { width: 5px; height: 5px; }
    .source-code::-webkit-scrollbar-thumb { background: #1E3A5F; border-radius: 4px; }
    /* Syntax */
    .kw { color: #98FB98; font-weight: 600; }
    .ty { color: #00D4FF; font-weight: 600; }
    .id { color: #FFB800; }
    .num { color: #00FF88; }
    .cm { color: #3A6A8F; font-style: italic; }
    .str { color: #FF8C42; }
    .prop { color: #DDA0DD; }
    /* Preview viewport */
    .preview-wrapper {
      flex: 1; overflow: auto; display: flex;
      flex-direction: column; gap: 12px;
    }
    .viewport-frame {
      background: #050D1A; border: 1px solid #1E3A5F;
      border-radius: 8px; overflow: hidden;
      transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
      margin: 0 auto;
      width: 100%;
      max-width: 100%;
      position: relative;
    }
    .viewport-bar {
      background: #0A1628; border-bottom: 1px solid #1E3A5F;
      padding: 6px 12px; display: flex; align-items: center; gap: 8px;
      flex-shrink: 0;
    }
    .viewport-dots { display: flex; gap: 5px; }
    .vdot {
      width: 10px; height: 10px; border-radius: 50%;
    }
    .vdot-red { background: #FF5F57; }
    .vdot-yellow { background: #FEBC2E; }
    .vdot-green { background: #28C840; }
    .viewport-url {
      flex: 1; background: #131E30; border-radius: 4px;
      padding: 3px 10px; font-family: monospace; font-size: 11px; color: #5588AA;
    }
    .viewport-content {
      padding: 16px; min-height: 300px; overflow: auto;
      transition: all 0.3s;
    }
    /* Rendered layout boxes */
    .nexus-grid {
      display: grid;
      gap: 10px;
    }
    .nexus-flex {
      display: flex;
      gap: 10px;
      flex-wrap: wrap;
    }
    .nexus-flex.col { flex-direction: column; }
    .layout-box {
      border-radius: 8px; padding: 14px;
      min-height: 60px;
      display: flex; flex-direction: column;
      align-items: flex-start; justify-content: flex-start;
      position: relative; overflow: hidden;
    }
    .layout-box::before {
      content: '';
      position: absolute; inset: 0;
      background: currentColor; opacity: 0.08;
      pointer-events: none;
    }
    .layout-box-label {
      font-size: 11px; font-weight: 700; text-transform: uppercase;
      letter-spacing: 0.8px; opacity: 0.9; margin-bottom: 4px;
    }
    .layout-box-type {
      font-size: 10px; opacity: 0.5; font-family: monospace;
    }
    .layout-box-ruler {
      position: absolute; bottom: 6px; right: 8px;
      font-size: 9px; opacity: 0.4; font-family: monospace;
    }
    /* Analysis sidebar */
    .analysis-section { margin-bottom: 14px; }
    .section-title {
      font-size: 10px; font-weight: 700; letter-spacing: 1.2px;
      color: #3A6A8F; text-transform: uppercase; margin: 0 0 8px;
    }
    .metric-row {
      display: flex; justify-content: space-between; align-items: center;
      padding: 6px 0; border-bottom: 1px solid #0F1E30; font-size: 12px;
    }
    .metric-row:last-child { border-bottom: none; }
    .metric-label { color: #7799BB; }
    .metric-value { color: #FFB800; font-family: monospace; font-size: 11px; font-weight: 700; }
    .badge {
      display: inline-block; padding: 2px 8px; border-radius: 20px;
      font-size: 10px; font-weight: 700;
    }
    .badge-green { background: #003322; color: #00FF88; border: 1px solid #00FF8844; }
    .badge-yellow { background: #332200; color: #FFB800; border: 1px solid #FFB80044; }
    .badge-red { background: #330000; color: #FF4444; border: 1px solid #FF444444; }
    .badge-blue { background: #002244; color: #00D4FF; border: 1px solid #00D4FF44; }
    /* No layout state */
    .empty-state {
      display: flex; flex-direction: column; align-items: center;
      justify-content: center; gap: 14px; height: 100%;
      color: #3A5A7A; text-align: center;
    }
    .empty-state .icon { font-size: 52px; opacity: 0.4; }
    /* Grid/flex legend */
    .legend { display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 10px; flex-shrink: 0; }
    .legend-item { display: flex; align-items: center; gap: 5px; font-size: 11px; color: #7799BB; }
    .legend-dot { width: 10px; height: 10px; border-radius: 3px; }
    /* Responsive ruler */
    .ruler {
      display: flex; align-items: center; gap: 2px; margin-bottom: 10px; flex-shrink: 0;
      font-size: 10px; color: #3A5A7A; font-family: monospace; overflow: hidden;
    }
    .ruler-tick { background: #1E3A5F; width: 1px; height: 8px; flex-shrink: 0; }
    .ruler-tick.major { height: 14px; background: #2A5A8F; }
    /* ── OW Theme Integration ─────────────────────────────── */
    body { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text, #E0E0E0) !important; }
    .header { border-bottom-color: var(--ow-border, #1E3A5F) !important; }
    .card { background: var(--ow-bg-card, #0F1F3A) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .card h3 { color: var(--ow-accent, #00D4FF) !important; }
    h1 { color: var(--ow-accent, #00D4FF) !important; }
    .subtitle { color: var(--ow-text-dim, #5588AA) !important; }
    .btn { background: linear-gradient(135deg, var(--ow-accent, #00D4FF), #0090CC) !important; color: var(--ow-bg, #0A1628) !important; }
    .btn-secondary { background: transparent !important; color: var(--ow-accent, #00D4FF) !important; border-color: var(--ow-border, #1E5A7F) !important; }
    .btn-secondary:hover { background: var(--ow-bg-raise, #0F2A4A) !important; }
    .bp-group { border-color: var(--ow-border, #1E3A5F) !important; }
    .bp-btn { color: var(--ow-text-dim, #88AACC) !important; }
    .bp-btn:hover, .bp-btn.active { background: var(--ow-bg-raise, #0F2A4A) !important; color: var(--ow-accent, #00D4FF) !important; }
    .dimension-badge { background: var(--ow-bg-card, #0F1F3A) !important; border-color: var(--ow-border, #1E3A5F) !important; color: var(--ow-warning, #FFB800) !important; }
    .source-code { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border-subtle, #0F1E30) !important; color: #AAD4EE !important; }
    .viewport-frame { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .viewport-bar { background: var(--ow-bg, #0A1628) !important; border-bottom-color: var(--ow-border, #1E3A5F) !important; }
    .viewport-url { background: var(--ow-bg-raise, #131E30) !important; color: var(--ow-text-muted, #5588AA) !important; }
    .metric-row { border-bottom-color: rgba(0,0,0,0.3) !important; }
    .metric-label { color: var(--ow-text-dim, #7799BB) !important; }
    .metric-value { color: var(--ow-warning, #FFB800) !important; }
    .section-title { color: var(--ow-text-muted, #3A6A8F) !important; }
    .empty-state { color: var(--ow-text-muted, #3A5A7A) !important; }
  </style>
</head>
<body>
  <div class="header">
    <div class="logo">&#9638;</div>
    <div>
      <h1>NEXUS Layout Preview</h1>
      <p class="subtitle">Responsive layout visualization &amp; analysis</p>
    </div>
    <div style="margin-left:auto; display:flex; gap:8px;">
      <button class="btn btn-secondary" onclick="sendCmd('loadActive')">&#8635; Reload</button>
      <button class="btn btn-secondary" onclick="sendCmd('openFile')">&#128196; Open File</button>
      <button class="btn btn-secondary" onclick="openThemePicker()" title="Switch Theme" aria-label="Switch Theme" style="padding:8px 12px;">&#127912;</button>
    </div>
  </div>

  <!-- Toolbar -->
  <div class="toolbar">
    <label style="color:#5588AA;font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:0.8px;">Breakpoint</label>
    <div class="bp-group">
      <button class="bp-btn active" id="bp-desktop" onclick="setBreakpoint('desktop')">
        &#128187; Desktop
      </button>
      <button class="bp-btn" id="bp-tablet" onclick="setBreakpoint('tablet')">
        &#128250; Tablet
      </button>
      <button class="bp-btn" id="bp-mobile" onclick="setBreakpoint('mobile')">
        &#128241; Mobile
      </button>
    </div>
    <button class="btn" onclick="sendCmd('refresh')">&#9654; Refresh Preview</button>
    <div class="dimension-badge" id="dim-badge">1920 &times; 1080</div>
  </div>

  <div class="main-layout">

    <!-- Left: Source Code -->
    <div class="card">
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px;flex-shrink:0;">
        <h3 style="margin:0;">&#128196; NEXUS Source</h3>
        <span style="font-family:monospace;font-size:11px;color:#FFB800;background:#0A1628;border:1px solid #1E3A5F;border-radius:5px;padding:3px 8px;" id="filename-badge">No file</span>
      </div>
      <div class="source-code" id="source-display">
        <div class="empty-state" style="height:200px;">
          <div class="icon">&#9638;</div>
          <div>Open a <strong>.nexus</strong> file in the editor<br>to see its source here.</div>
        </div>
      </div>
    </div>

    <!-- Right: Preview + Analysis -->
    <div style="display:flex;flex-direction:column;gap:16px;overflow:hidden;">

      <!-- Viewport frame -->
      <div class="card" style="flex:1;">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px;flex-shrink:0;">
          <h3 style="margin:0;">&#128064; Layout Preview</h3>
          <div class="legend">
            <div class="legend-item"><div class="legend-dot" style="background:#00D4FF33;border:1px solid #00D4FF44;"></div>Grid cell</div>
            <div class="legend-item"><div class="legend-dot" style="background:#FFB80033;border:1px solid #FFB80044;"></div>Flex item</div>
            <div class="legend-item"><div class="legend-dot" style="background:#00FF8833;border:1px solid #00FF8844;"></div>Container</div>
          </div>
        </div>

        <div class="preview-wrapper">
          <div class="viewport-frame" id="viewport-frame">
            <div class="viewport-bar">
              <div class="viewport-dots">
                <div class="vdot vdot-red"></div>
                <div class="vdot vdot-yellow"></div>
                <div class="vdot vdot-green"></div>
              </div>
              <div class="viewport-url">nexus://preview/<span id="url-filename">untitled.nexus</span></div>
            </div>
            <div class="viewport-content" id="layout-preview">
              <div class="empty-state">
                <div class="icon">&#9638;</div>
                <div>Layout preview will appear here.</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Analysis strip -->
      <div class="card" style="flex-shrink:0;">
        <h3 style="margin:0 0 10px;">&#128202; Layout Analysis</h3>
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(120px,1fr));gap:10px;">
          <div>
            <div class="section-title">Layout Type</div>
            <div id="ana-type"><span class="badge badge-blue">—</span></div>
          </div>
          <div>
            <div class="section-title">Grid Count</div>
            <div style="color:#FFB800;font-family:monospace;font-weight:700;" id="ana-grids">—</div>
          </div>
          <div>
            <div class="section-title">Flex Count</div>
            <div style="color:#00D4FF;font-family:monospace;font-weight:700;" id="ana-flex">—</div>
          </div>
          <div>
            <div class="section-title">Responsive Rules</div>
            <div style="color:#00FF88;font-family:monospace;font-weight:700;" id="ana-responsive">—</div>
          </div>
          <div>
            <div class="section-title">Lines</div>
            <div style="color:#DDA0DD;font-family:monospace;font-weight:700;" id="ana-lines">—</div>
          </div>
          <div>
            <div class="section-title">Complexity</div>
            <div id="ana-complexity"><span class="badge badge-green">—</span></div>
          </div>
        </div>
      </div>

    </div>
  </div>

  <script src="${widgetScriptUri}"></script>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();
    (function owInit() {
      if (typeof OW === 'undefined') return;
      OW.setVscodeApi(vscode);
      try { var s = vscode.getState(); if (s && s.owTheme) { OW.switchTheme(s.owTheme); return; } } catch(e){}
      OW.loadTheme();
    })();
    function openThemePicker() {
      if (typeof OW === 'undefined') return;
      var body = OW.themePicker({ onchange: function(id) {
        OW.switchTheme(id);
        try { vscode.setState(Object.assign(vscode.getState()||{}, { owTheme: id })); } catch(e){}
      }});
      var m = OW.modal({ title: '&#127912; Choose Theme', body: body, size: 'sm',
        buttons: [{ label: 'Close', variant: 'ghost', onclick: function() { m.close(); } }]
      });
    }
    let currentBreakpoint = 'desktop';
    let currentLayout = null;

    const BREAKPOINTS = {
      desktop: { label: '1920 \xd7 1080', width: '100%', maxWidth: '100%' },
      tablet:  { label: '768 \xd7 1024',  width: '768px', maxWidth: '768px' },
      mobile:  { label: '390 \xd7 844',   width: '390px', maxWidth: '390px' },
    };

    function sendCmd(cmd) { vscode.postMessage({ command: cmd }); }

    function setBreakpoint(bp) {
      currentBreakpoint = bp;
      ['desktop','tablet','mobile'].forEach(b => {
        document.getElementById('bp-' + b).classList.toggle('active', b === bp);
      });
      const info = BREAKPOINTS[bp];
      document.getElementById('dim-badge').innerHTML = info.label;
      const frame = document.getElementById('viewport-frame');
      frame.style.maxWidth = info.maxWidth;
      if (info.width !== '100%') {
        frame.style.width = info.width;
      } else {
        frame.style.width = '';
      }
      if (currentLayout) renderLayout(currentLayout);
    }

    function syntaxHighlight(source) {
      const KEYWORDS = ['grid','flex','stack','layout','container','header','footer','sidebar','nav','content','columns','rows','gap','direction','wrap','span','responsive','breakpoint'];
      const PROPS = ['columns','rows','gap','direction','wrap','span','align','justify','padding','margin','width','height','background','border','color'];
      let html = '';
      const lines = source.split('\n');
      for (const line of lines) {
        let esc = line.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
        // Comments
        esc = esc.replace(/(\/\/.*$)/g,'<span class="cm">$1</span>');
        if (!esc.includes('cm')) {
          // Strings
          esc = esc.replace(/(".*?"|'.*?')/g,'<span class="str">$1</span>');
          // Numbers
          esc = esc.replace(/\b(\d+(?:\.\d+)?(?:px|em|rem|%|fr|vh|vw)?)\b/g,'<span class="num">$1</span>');
          // Keywords (types)
          KEYWORDS.forEach(k => {
            esc = esc.replace(new RegExp('\\b(' + k + ')\\b', 'g'), '<span class="kw">$1</span>');
          });
          // Properties (before colon)
          esc = esc.replace(/\b([a-zA-Z_]\w*)\s*(?=:)/g,'<span class="prop">$1</span>');
          // Identifiers after block keyword
          esc = esc.replace(/\b(kw)(\s+)([a-zA-Z_]\w*)/g,'$1$2<span class="id">$3</span>');
        }
        html += esc + '\n';
      }
      return html;
    }

    function renderLayout(layout) {
      const container = document.getElementById('layout-preview');
      const bp = currentBreakpoint;

      if (!layout) {
        container.innerHTML = '<div class="empty-state"><div class="icon">&#9638;</div><div>No layout data.</div></div>';
        return;
      }

      const cols = bp === 'mobile' ? 1 : bp === 'tablet' ? Math.min(layout.columns || 2, 2) : (layout.columns || 3);
      const isGrid = layout.type === 'grid';
      const isFlex = layout.type === 'flex';

      let html = '';

      if (isGrid) {
        html += '<div style="display:grid;grid-template-columns:repeat(' + cols + ',1fr);gap:10px;">';
      } else if (isFlex) {
        const dir = bp === 'mobile' ? 'column' : (layout.direction || 'row');
        html += '<div style="display:flex;flex-direction:' + dir + ';gap:10px;flex-wrap:wrap;">';
      } else {
        html += '<div style="display:flex;flex-direction:column;gap:10px;">';
      }

      if (layout.children && layout.children.length > 0) {
        layout.children.forEach((child, i) => {
          const color = child.color || '#00D4FF';
          const span = bp === 'mobile' ? 1 : (child.span || 1);
          let style = 'background:' + color + '14;border:1px solid ' + color + '44;color:' + color + ';';
          if (isGrid && span > 1) style += 'grid-column:span ' + Math.min(span, cols) + ';';
          if (isFlex && child.span) style += 'flex:' + child.span + ';min-width:120px;';

          const heightMap = {
            'header': '70px', 'footer': '55px', 'sidebar': '220px',
            'nav': '50px', 'content': '200px',
          };
          const minH = heightMap[child.type] || '80px';
          style += 'min-height:' + minH + ';';

          html += '<div class="layout-box" style="' + style + '">';
          html += '<div class="layout-box-label">' + (child.label || child.id || child.type) + '</div>';
          html += '<div class="layout-box-type">' + child.type;
          if (child.span && child.span > 1) html += ' &bull; span ' + child.span;
          html += '</div>';
          html += '<div class="layout-box-ruler">' + (isGrid ? 'col ' + (i % cols + 1) : (isFlex ? 'flex ' + child.span : 'block')) + '</div>';
          html += '</div>';
        });
      } else {
        html += '<div style="color:#3A5A7A;font-size:12px;padding:20px;">No layout blocks found.</div>';
      }

      html += '</div>';
      container.innerHTML = html;
    }

    window.addEventListener('message', event => {
      const msg = event.data;

      switch (msg.type) {
        case 'layoutLoaded': {
          currentLayout = msg.layout;
          document.getElementById('filename-badge').textContent = msg.fileName;
          document.getElementById('url-filename').textContent = msg.fileName;

          // Source highlight
          const sd = document.getElementById('source-display');
          if (msg.source.trim()) {
            sd.innerHTML = syntaxHighlight(msg.source);
          } else {
            sd.innerHTML = '<div class="empty-state" style="height:200px;"><div class="icon">&#9638;</div><div>File is empty.</div></div>';
          }

          renderLayout(msg.layout);

          // Analysis
          const ana = msg.analysis;
          const typeLabel = msg.layout.type === 'grid' ? 'Grid' : msg.layout.type === 'flex' ? 'Flex' : 'Block';
          const typeBadge = '<span class="badge badge-' + (typeLabel === 'Grid' ? 'blue' : typeLabel === 'Flex' ? 'yellow' : 'green') + '">' + typeLabel + '</span>';
          document.getElementById('ana-type').innerHTML = typeBadge;
          document.getElementById('ana-grids').textContent = ana.gridCount;
          document.getElementById('ana-flex').textContent = ana.flexCount;
          document.getElementById('ana-responsive').textContent = ana.responsiveRules;
          document.getElementById('ana-lines').textContent = ana.lines;
          const compBadgeClass = { Simple: 'green', Moderate: 'yellow', Complex: 'red' }[ana.complexity] || 'blue';
          document.getElementById('ana-complexity').innerHTML = '<span class="badge badge-' + compBadgeClass + '">' + ana.complexity + '</span>';
          break;
        }

        case 'noLayout': {
          currentLayout = null;
          document.getElementById('filename-badge').textContent = 'No .nexus file';
          document.getElementById('source-display').innerHTML =
            '<div class="empty-state" style="height:200px;"><div class="icon">&#9638;</div><div>Open a <strong>.nexus</strong> file in the editor.</div></div>';
          document.getElementById('layout-preview').innerHTML =
            '<div class="empty-state"><div class="icon">&#9638;</div><div>No layout to preview.</div></div>';
          break;
        }
      }
    });
  </script>
</body>
</html>`;
    }
    dispose() {
        LayoutPreviewPanel.currentPanel = undefined;
        this._panel.dispose();
        while (this._disposables.length) {
            const d = this._disposables.pop();
            if (d)
                d.dispose();
        }
    }
}
exports.LayoutPreviewPanel = LayoutPreviewPanel;
LayoutPreviewPanel.viewType = 'omnisystem.layoutPreview';
//# sourceMappingURL=LayoutPreview.js.map