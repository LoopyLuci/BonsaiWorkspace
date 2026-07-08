import * as vscode from 'vscode';

function getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}

// ─── WidgetGalleryPanel ────────────────────────────────────────────────────────

export class WidgetGalleryPanel {
    public static currentPanel: WidgetGalleryPanel | undefined;
    public static readonly viewType = 'omnisystem.widgetGallery';

    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];

    public static createOrShow(extensionUri: vscode.Uri): void {
        const column = vscode.window.activeTextEditor
            ? vscode.window.activeTextEditor.viewColumn
            : undefined;

        if (WidgetGalleryPanel.currentPanel) {
            WidgetGalleryPanel.currentPanel._panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            WidgetGalleryPanel.viewType,
            'Widget Gallery',
            column || vscode.ViewColumn.One,
            {
                enableScripts: true,
                localResourceRoots: [extensionUri],
                retainContextWhenHidden: true,
            }
        );

        WidgetGalleryPanel.currentPanel = new WidgetGalleryPanel(panel, extensionUri);
    }

    private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
        this._panel = panel;
        this._extensionUri = extensionUri;

        this._panel.iconPath = {
            light: vscode.Uri.joinPath(extensionUri, 'icons', 'titan-light.svg'),
            dark: vscode.Uri.joinPath(extensionUri, 'icons', 'titan-dark.svg'),
        };

        this._update();

        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
        this._panel.onDidChangeViewState(
            () => { if (this._panel.visible) { this._update(); } },
            null,
            this._disposables
        );
        this._panel.webview.onDidReceiveMessage(
            (message) => this._handleMessage(message),
            null,
            this._disposables
        );
    }

    private _post(msg: object): void {
        this._panel.webview.postMessage(msg);
    }

    public static postMessage(msg: object): void {
        WidgetGalleryPanel.currentPanel?._panel.webview.postMessage(msg);
    }

    private async _handleMessage(message: { command: string; widgetId?: string; theme?: string }): Promise<void> {
        switch (message.command) {
            case 'owThemeChange':
                if (message.theme) {
                    vscode.commands.executeCommand('omnisystem._broadcastTheme', message.theme);
                }
                break;
            case 'copyCode':
                if (message.widgetId) {
                    await vscode.env.clipboard.writeText(this._getWidgetCode(message.widgetId));
                    vscode.window.showInformationMessage(`Widget code copied: ${message.widgetId}`);
                }
                break;
            case 'openDocs':
                vscode.window.showInformationMessage('Widget documentation coming soon.');
                break;
            case 'applyTheme':
                if (message.theme) {
                    this._post({ type: 'themeApplied', theme: message.theme });
                }
                break;
        }
    }

    private _getWidgetCode(widgetId: string): string {
        const codes: Record<string, string> = {
            'btn-primary':   `OW.btn({ label: 'Click Me', variant: 'primary', size: 'md' })`,
            'btn-secondary': `OW.btn({ label: 'Cancel', variant: 'secondary', size: 'md' })`,
            'btn-ghost':     `OW.btn({ label: 'Learn More', variant: 'ghost', size: 'md' })`,
            'btn-danger':    `OW.btn({ label: 'Delete', variant: 'danger', size: 'md' })`,
            'card-basic':    `OW.card({ title: 'Card Title', body: 'Card content here.', footer: 'Footer' })`,
            'stat-card':     `OW.statCard({ value: '99.9%', label: 'Uptime', delta: '+0.1%', color: 'success' })`,
            'badge':         `OW.badge({ label: 'v2.0', variant: 'primary' })`,
            'progress':      `OW.progress({ value: 75, max: 100, color: 'primary', animated: true })`,
            'toggle':        `OW.toggle({ checked: true, label: 'Enable feature', onChange: (v) => console.log(v) })`,
            'chip':          `OW.chip({ label: 'Titan', color: '#00D4FF', removable: true })`,
            'spinner':       `OW.spinner({ size: 'md', color: 'primary' })`,
            'alert-info':    `OW.alert({ type: 'info', title: 'Info', message: 'Something to note.' })`,
            'alert-success': `OW.alert({ type: 'success', title: 'Done!', message: 'Operation succeeded.' })`,
            'alert-error':   `OW.alert({ type: 'error', title: 'Error', message: 'Something went wrong.' })`,
            'tabs':          `OW.tabs({ tabs: [{ label: 'Tab 1', content: el1 }, { label: 'Tab 2', content: el2 }] })`,
            'modal':         `const m = OW.modal({ title: 'Modal Title', body: content, size: 'md', buttons: [{ label: 'Close', variant: 'ghost', onclick: () => m.close() }] })`,
            'health-ring':   `OW.healthRing(85, { label: 'Health', size: 120, color: 'success' })`,
            'theme-picker':  `OW.themePicker({ onchange: (id) => OW.switchTheme(id) })`,
            'widget-browser':`OW.widgetBrowser({ onselect: (w) => console.log(w.id) })`,
            'metric':        `OW.metric({ label: 'Requests/sec', value: '12,450', delta: '+5%', trend: 'up' })`,
        };
        return codes[widgetId] || `OW.${widgetId}()`;
    }

    private _update(): void {
        const webview = this._panel.webview;
        const widgetStyleUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.css')
        );
        const widgetScriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.js')
        );
        this._panel.title = 'Widget Gallery';
        this._panel.webview.html = this._getHtml(widgetStyleUri.toString(), widgetScriptUri.toString());
    }

    private _getHtml(widgetStyleUri: string, widgetScriptUri: string): string {
        const nonce = getNonce();
        return /* html */`<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src vscode-resource: 'unsafe-inline'; script-src vscode-resource: 'nonce-${nonce}' https:;">
  <title>Widget Gallery</title>
  <link rel="stylesheet" href="${widgetStyleUri}"/>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body {
      background: var(--ow-bg, #050D1A); color: var(--ow-text, #E8F4FF);
      font-family: 'Segoe UI', system-ui, sans-serif;
      margin: 0; padding: 0; min-height: 100vh; overflow-x: hidden;
    }
    /* ── Header ── */
    .gallery-header {
      position: sticky; top: 0; z-index: 100;
      background: var(--ow-glass, rgba(8,18,36,0.92));
      backdrop-filter: blur(20px);
      border-bottom: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      padding: 14px 24px; display: flex; align-items: center; gap: 16px;
    }
    .gallery-logo {
      width: 40px; height: 40px;
      background: linear-gradient(135deg, var(--ow-accent, #00D4FF), var(--ow-accent-2, #0055FF));
      border-radius: 10px;
      display: flex; align-items: center; justify-content: center;
      font-size: 20px; flex-shrink: 0;
      box-shadow: var(--ow-shadow-glow, 0 0 22px rgba(0,212,255,0.28));
    }
    .gallery-title { font-size: 20px; font-weight: 700; color: var(--ow-accent, #00D4FF); margin: 0; }
    .gallery-subtitle { font-size: 11px; color: var(--ow-text-muted, rgba(232,244,255,0.28)); margin: 0; }
    .header-search {
      flex: 1; max-width: 320px;
      background: var(--ow-bg-raise, rgba(0,20,50,0.52));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-full, 9999px);
      padding: 7px 16px; font-size: 13px; color: var(--ow-text, #E8F4FF);
      outline: none; transition: border-color var(--ow-t, 150ms);
    }
    .header-search:focus { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); }
    .header-search::placeholder { color: var(--ow-text-muted, rgba(232,244,255,0.28)); }
    .header-actions { margin-left: auto; display: flex; gap: 8px; align-items: center; }
    .stat-pill {
      background: var(--ow-accent-dim, rgba(0,212,255,0.13));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-full, 9999px);
      padding: 4px 12px; font-size: 11px; font-weight: 700;
      color: var(--ow-accent, #00D4FF);
    }
    /* ── Layout ── */
    .gallery-body { display: flex; height: calc(100vh - 65px); }
    /* ── Sidebar ── */
    .sidebar {
      width: 200px; flex-shrink: 0;
      border-right: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      padding: 16px 12px; overflow-y: auto;
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
    }
    .sidebar::-webkit-scrollbar { width: 3px; }
    .sidebar::-webkit-scrollbar-thumb { background: var(--ow-border, rgba(0,212,255,0.18)); border-radius: 4px; }
    .sidebar-section { margin-bottom: 20px; }
    .sidebar-label {
      font-size: 9px; font-weight: 700; letter-spacing: 1.5px; text-transform: uppercase;
      color: var(--ow-text-muted, rgba(232,244,255,0.28)); margin: 0 0 8px 4px;
    }
    .cat-btn {
      display: flex; align-items: center; gap: 8px;
      width: 100%; background: transparent; border: none;
      border-radius: var(--ow-r-md, 8px); padding: 8px 10px;
      font-size: 12px; font-weight: 600; color: var(--ow-text-dim, rgba(232,244,255,0.52));
      cursor: pointer; transition: all var(--ow-t, 150ms); text-align: left;
    }
    .cat-btn:hover { background: var(--ow-bg-raise, rgba(0,20,50,0.52)); color: var(--ow-text, #E8F4FF); }
    .cat-btn.active {
      background: var(--ow-accent-dim, rgba(0,212,255,0.13));
      color: var(--ow-accent, #00D4FF);
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
    }
    .cat-count {
      margin-left: auto; font-size: 10px; font-weight: 700;
      background: var(--ow-bg, #050D1A); border-radius: var(--ow-r-full, 9999px);
      padding: 1px 6px; color: var(--ow-text-muted, rgba(232,244,255,0.28));
    }
    /* ── Grid ── */
    .gallery-grid-wrap { flex: 1; overflow-y: auto; padding: 20px; }
    .gallery-grid-wrap::-webkit-scrollbar { width: 5px; }
    .gallery-grid-wrap::-webkit-scrollbar-thumb { background: var(--ow-border, rgba(0,212,255,0.18)); border-radius: 4px; }
    .section-header { display: flex; align-items: center; gap: 10px; margin: 0 0 14px; }
    .section-header h2 {
      font-size: 14px; font-weight: 700; color: var(--ow-accent, #00D4FF); margin: 0;
      letter-spacing: 0.5px;
    }
    .section-divider { flex: 1; height: 1px; background: var(--ow-border, rgba(0,212,255,0.18)); }
    .widget-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
      gap: 14px; margin-bottom: 32px;
    }
    /* ── Widget Card ── */
    .wg-card {
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-lg, 12px);
      padding: 16px; cursor: pointer;
      transition: all var(--ow-t, 150ms);
      position: relative; overflow: hidden;
      display: flex; flex-direction: column; gap: 10px;
    }
    .wg-card::before {
      content: ''; position: absolute; inset: 0;
      background: linear-gradient(135deg, var(--ow-accent, #00D4FF), transparent);
      opacity: 0; transition: opacity var(--ow-t, 150ms); pointer-events: none;
    }
    .wg-card:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); transform: translateY(-2px); box-shadow: var(--ow-shadow-lg, 0 6px 36px rgba(0,212,255,0.14)); }
    .wg-card:hover::before { opacity: 0.04; }
    .wg-card.selected { border-color: var(--ow-accent, #00D4FF); box-shadow: var(--ow-shadow-glow, 0 0 22px rgba(0,212,255,0.28)); }
    .wg-preview {
      height: 72px; border-radius: var(--ow-r-md, 8px);
      background: var(--ow-bg, #050D1A);
      border: 1px solid var(--ow-border-subtle, rgba(0,212,255,0.07));
      display: flex; align-items: center; justify-content: center;
      font-size: 22px; position: relative; overflow: hidden;
    }
    .wg-preview-label {
      position: absolute; bottom: 4px; right: 6px;
      font-size: 9px; color: var(--ow-text-muted, rgba(232,244,255,0.28));
      font-family: monospace;
    }
    .wg-name { font-size: 12px; font-weight: 700; color: var(--ow-text, #E8F4FF); margin: 0; }
    .wg-desc { font-size: 11px; color: var(--ow-text-dim, rgba(232,244,255,0.52)); margin: 0; line-height: 1.4; }
    .wg-footer { display: flex; align-items: center; gap: 6px; margin-top: auto; }
    .wg-tag {
      font-size: 9px; font-weight: 700; padding: 2px 6px;
      border-radius: var(--ow-r-full, 9999px);
      background: var(--ow-accent-dim, rgba(0,212,255,0.13));
      color: var(--ow-accent, #00D4FF);
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
    }
    .wg-copy-btn {
      margin-left: auto; background: transparent; border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-sm, 5px); padding: 3px 8px;
      font-size: 10px; color: var(--ow-text-dim, rgba(232,244,255,0.52));
      cursor: pointer; transition: all var(--ow-t-fast, 70ms);
    }
    .wg-copy-btn:hover { border-color: var(--ow-accent, #00D4FF); color: var(--ow-accent, #00D4FF); }
    /* ── Detail panel (right) ── */
    .detail-panel {
      width: 320px; flex-shrink: 0;
      border-left: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
      display: flex; flex-direction: column; overflow: hidden;
    }
    .detail-header {
      padding: 16px 18px 12px;
      border-bottom: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      flex-shrink: 0;
    }
    .detail-title { font-size: 15px; font-weight: 700; color: var(--ow-accent, #00D4FF); margin: 0 0 4px; }
    .detail-cat { font-size: 11px; color: var(--ow-text-muted, rgba(232,244,255,0.28)); }
    .detail-body { flex: 1; overflow-y: auto; padding: 16px 18px; }
    .detail-body::-webkit-scrollbar { width: 3px; }
    .detail-body::-webkit-scrollbar-thumb { background: var(--ow-border, rgba(0,212,255,0.18)); border-radius: 4px; }
    .detail-preview {
      height: 120px; border-radius: var(--ow-r-lg, 12px);
      background: var(--ow-bg, #050D1A);
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      display: flex; align-items: center; justify-content: center;
      font-size: 36px; margin-bottom: 16px;
    }
    .detail-section { margin-bottom: 16px; }
    .detail-section-label {
      font-size: 9px; font-weight: 700; letter-spacing: 1.2px; text-transform: uppercase;
      color: var(--ow-text-muted, rgba(232,244,255,0.28)); margin: 0 0 8px;
    }
    .detail-desc { font-size: 12px; color: var(--ow-text-dim, rgba(232,244,255,0.52)); line-height: 1.5; }
    .code-block {
      background: var(--ow-bg, #050D1A); border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-md, 8px); padding: 12px;
      font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 11px;
      color: #AAD4EE; line-height: 1.6; overflow-x: auto; white-space: pre-wrap;
    }
    .detail-footer {
      padding: 14px 18px; border-top: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      display: flex; gap: 8px; flex-shrink: 0;
    }
    .btn-primary-ow {
      background: linear-gradient(135deg, var(--ow-accent, #00D4FF), #0090CC);
      color: var(--ow-bg, #050D1A); border: none; border-radius: var(--ow-r-md, 8px);
      padding: 8px 16px; cursor: pointer; font-weight: 700; font-size: 12px;
      transition: all var(--ow-t, 150ms); flex: 1;
    }
    .btn-primary-ow:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .btn-ghost-ow {
      background: transparent; color: var(--ow-accent, #00D4FF);
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-md, 8px); padding: 8px 12px;
      cursor: pointer; font-weight: 600; font-size: 12px;
      transition: all var(--ow-t, 150ms);
    }
    .btn-ghost-ow:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); background: var(--ow-accent-dim, rgba(0,212,255,0.13)); }
    /* ── Empty state ── */
    .empty-state {
      display: flex; flex-direction: column; align-items: center; justify-content: center;
      gap: 12px; padding: 60px 20px; color: var(--ow-text-muted, rgba(232,244,255,0.28));
      text-align: center;
    }
    .empty-state .icon { font-size: 48px; opacity: 0.4; }
    .empty-state p { font-size: 13px; margin: 0; }
    /* ── No-selection placeholder ── */
    .no-selection {
      flex: 1; display: flex; flex-direction: column; align-items: center;
      justify-content: center; gap: 10px; padding: 30px;
      color: var(--ow-text-muted, rgba(232,244,255,0.28)); text-align: center;
    }
    .no-selection .icon { font-size: 42px; opacity: 0.3; }
    .no-selection p { font-size: 12px; line-height: 1.5; }
    /* ── Focus rings for accessibility ── */
    :focus-visible { outline: 2px solid var(--ow-accent, #00D4FF); outline-offset: 2px; }
    button:focus-visible { outline: 2px solid var(--ow-accent, #00D4FF); outline-offset: 2px; }
  </style>
</head>
<body>

  <header class="gallery-header" role="banner">
    <div class="gallery-logo" aria-hidden="true">&#9775;</div>
    <div>
      <h1 class="gallery-title">Widget Gallery</h1>
      <p class="gallery-subtitle">Omni Widget System · 40+ components · 6 themes</p>
    </div>
    <input class="header-search" type="search" id="search-input"
      placeholder="Search widgets..." aria-label="Search widgets"
      oninput="onSearch(this.value)"/>
    <div class="header-actions">
      <span class="stat-pill" id="widget-count-pill" aria-live="polite">40+ widgets</span>
      <button class="btn-ghost-ow" onclick="openThemePicker()" title="Switch Theme" aria-label="Switch Theme" style="padding:8px 12px;">&#127912; Theme</button>
    </div>
  </header>

  <div class="gallery-body">

    <!-- Sidebar: categories -->
    <nav class="sidebar" aria-label="Widget categories">
      <div class="sidebar-section">
        <p class="sidebar-label">Categories</p>
        <button class="cat-btn active" id="cat-all" onclick="filterCat('all')" aria-pressed="true">
          &#9632; All
          <span class="cat-count" id="cnt-all">40</span>
        </button>
        <button class="cat-btn" id="cat-Buttons" onclick="filterCat('Buttons')" aria-pressed="false">
          &#9654; Buttons
          <span class="cat-count" id="cnt-Buttons">0</span>
        </button>
        <button class="cat-btn" id="cat-Inputs" onclick="filterCat('Inputs')" aria-pressed="false">
          &#9998; Inputs
          <span class="cat-count" id="cnt-Inputs">0</span>
        </button>
        <button class="cat-btn" id="cat-Cards" onclick="filterCat('Cards')" aria-pressed="false">
          &#9646; Cards
          <span class="cat-count" id="cnt-Cards">0</span>
        </button>
        <button class="cat-btn" id="cat-Navigation" onclick="filterCat('Navigation')" aria-pressed="false">
          &#10143; Navigation
          <span class="cat-count" id="cnt-Navigation">0</span>
        </button>
        <button class="cat-btn" id="cat-Feedback" onclick="filterCat('Feedback')" aria-pressed="false">
          &#128276; Feedback
          <span class="cat-count" id="cnt-Feedback">0</span>
        </button>
        <button class="cat-btn" id="cat-Data" onclick="filterCat('Data')" aria-pressed="false">
          &#128202; Data
          <span class="cat-count" id="cnt-Data">0</span>
        </button>
        <button class="cat-btn" id="cat-Overlays" onclick="filterCat('Overlays')" aria-pressed="false">
          &#10752; Overlays
          <span class="cat-count" id="cnt-Overlays">0</span>
        </button>
        <button class="cat-btn" id="cat-Special" onclick="filterCat('Special')" aria-pressed="false">
          &#10024; Special
          <span class="cat-count" id="cnt-Special">0</span>
        </button>
      </div>
      <div class="sidebar-section">
        <p class="sidebar-label">Themes</p>
        <button class="cat-btn" onclick="switchTheme('omni-dark')" aria-label="OmniDark theme">&#9679; OmniDark</button>
        <button class="cat-btn" onclick="switchTheme('omni-light')" aria-label="OmniLight theme">&#9679; OmniLight</button>
        <button class="cat-btn" onclick="switchTheme('omni-neon')" aria-label="OmniNeon theme">&#9679; OmniNeon</button>
        <button class="cat-btn" onclick="switchTheme('omni-forest')" aria-label="OmniForest theme">&#9679; OmniForest</button>
        <button class="cat-btn" onclick="switchTheme('omni-aurora')" aria-label="OmniAurora theme">&#9679; OmniAurora</button>
        <button class="cat-btn" onclick="switchTheme('omni-sunset')" aria-label="OmniSunset theme">&#9679; OmniSunset</button>
      </div>
    </nav>

    <!-- Main grid -->
    <main class="gallery-grid-wrap" role="main" aria-label="Widget grid" id="widget-grid-main">
      <div id="grid-content">
        <!-- Populated by JS -->
        <div class="empty-state"><div class="icon">&#9775;</div><p>Loading widgets...</p></div>
      </div>
    </main>

    <!-- Detail panel -->
    <aside class="detail-panel" role="complementary" aria-label="Widget details" id="detail-panel">
      <div class="no-selection" id="no-selection-state">
        <div class="icon">&#8592;</div>
        <p>Select any widget to see its<br>description, preview, and code.</p>
      </div>
      <div id="detail-content" style="display:none; flex-direction:column; height:100%;">
        <div class="detail-header">
          <div class="detail-title" id="detail-name">—</div>
          <div class="detail-cat" id="detail-cat">—</div>
        </div>
        <div class="detail-body">
          <div class="detail-preview" id="detail-preview" aria-hidden="true">&#9775;</div>
          <div class="detail-section">
            <p class="detail-section-label">Description</p>
            <p class="detail-desc" id="detail-desc">—</p>
          </div>
          <div class="detail-section">
            <p class="detail-section-label">Usage (JavaScript)</p>
            <div class="code-block" id="detail-code">—</div>
          </div>
          <div class="detail-section">
            <p class="detail-section-label">CSS Class</p>
            <div class="code-block" id="detail-css">—</div>
          </div>
        </div>
        <div class="detail-footer">
          <button class="btn-primary-ow" onclick="copyWidgetCode()" id="copy-btn" aria-label="Copy widget code">
            &#128203; Copy Code
          </button>
          <button class="btn-ghost-ow" onclick="openDocs()" aria-label="Open documentation">
            &#128196; Docs
          </button>
        </div>
      </div>
    </aside>

  </div>

  <script src="${widgetScriptUri}"></script>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();

    // Restore theme + wire up IPC bridge
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

    function switchTheme(id) {
      if (typeof OW === 'undefined') return;
      OW.switchTheme(id);
      try { vscode.setState(Object.assign(vscode.getState()||{}, { owTheme: id })); } catch(e){}
    }

    // Widget database — built from OW.widgetDB if available, else local copy
    var WIDGETS = [];
    var selectedWidget = null;
    var currentCat = 'all';
    var currentSearch = '';

    var WIDGET_ICONS = {
      'Buttons':    '&#9654;',
      'Inputs':     '&#9998;',
      'Cards':      '&#9646;',
      'Navigation': '&#10143;',
      'Feedback':   '&#128276;',
      'Data':       '&#128202;',
      'Overlays':   '&#10752;',
      'Special':    '&#10024;',
    };

    var FALLBACK_WIDGETS = [
      // Buttons
      { id:'btn-primary',    cat:'Buttons',    label:'Primary Button',    desc:'Main call-to-action. Filled accent background with ripple effect.' },
      { id:'btn-secondary',  cat:'Buttons',    label:'Secondary Button',  desc:'Alternative action. Outlined style.' },
      { id:'btn-ghost',      cat:'Buttons',    label:'Ghost Button',      desc:'Minimal presence for tertiary actions.' },
      { id:'btn-danger',     cat:'Buttons',    label:'Danger Button',     desc:'Destructive actions. Red accent.' },
      { id:'btn-success',    cat:'Buttons',    label:'Success Button',    desc:'Confirmations and positive outcomes.' },
      { id:'btn-icon',       cat:'Buttons',    label:'Icon Button',       desc:'Square button containing only an icon.' },
      { id:'btn-loading',    cat:'Buttons',    label:'Loading Button',    desc:'Button with inline spinner state.' },
      // Inputs
      { id:'text-input',     cat:'Inputs',     label:'Text Input',        desc:'Single-line text entry with label and validation.' },
      { id:'select-input',   cat:'Inputs',     label:'Select',            desc:'Dropdown selection with custom styling.' },
      { id:'toggle',         cat:'Inputs',     label:'Toggle Switch',     desc:'Binary on/off control with smooth animation.' },
      { id:'checkbox',       cat:'Inputs',     label:'Checkbox',          desc:'Multi-select boolean input.' },
      { id:'radio',          cat:'Inputs',     label:'Radio Group',       desc:'Single-select from a list of options.' },
      { id:'range-slider',   cat:'Inputs',     label:'Range Slider',      desc:'Continuous numeric value picker.' },
      // Cards
      { id:'card-basic',     cat:'Cards',      label:'Basic Card',        desc:'Container with header, body, and optional footer.' },
      { id:'stat-card',      cat:'Cards',      label:'Stat Card',         desc:'Metric display with value, label, delta, and trend bar.' },
      { id:'metric',         cat:'Cards',      label:'Metric',            desc:'Compact key-value metric with trend indicator.' },
      { id:'feature-card',   cat:'Cards',      label:'Feature Card',      desc:'Marketing-style card with icon and CTA.' },
      { id:'glass-card',     cat:'Cards',      label:'Glass Card',        desc:'Glassmorphism card with blur backdrop.' },
      // Navigation
      { id:'tabs',           cat:'Navigation', label:'Tabs',              desc:'Horizontal tabbed interface with panel switching.' },
      { id:'breadcrumb',     cat:'Navigation', label:'Breadcrumb',        desc:'Hierarchical path indicator.' },
      { id:'sidebar-nav',    cat:'Navigation', label:'Sidebar Nav',       desc:'Vertical navigation with icons and labels.' },
      { id:'stepper',        cat:'Navigation', label:'Stepper',           desc:'Multi-step progress indicator.' },
      // Feedback
      { id:'alert-info',     cat:'Feedback',   label:'Info Alert',        desc:'Informational message banner.' },
      { id:'alert-success',  cat:'Feedback',   label:'Success Alert',     desc:'Positive confirmation banner.' },
      { id:'alert-error',    cat:'Feedback',   label:'Error Alert',       desc:'Error or danger notification.' },
      { id:'alert-warning',  cat:'Feedback',   label:'Warning Alert',     desc:'Cautionary message banner.' },
      { id:'toast',          cat:'Feedback',   label:'Toast',             desc:'Temporary popup notification from corner.' },
      { id:'spinner',        cat:'Feedback',   label:'Spinner',           desc:'Circular loading indicator.' },
      { id:'progress',       cat:'Feedback',   label:'Progress Bar',      desc:'Linear progress track with animated fill.' },
      // Data
      { id:'badge',          cat:'Data',       label:'Badge',             desc:'Small label pill for status and counts.' },
      { id:'chip',           cat:'Data',       label:'Chip',              desc:'Compact interactive tag, optionally removable.' },
      { id:'health-ring',    cat:'Data',       label:'Health Ring',       desc:'SVG ring chart for health/status scores.' },
      { id:'data-table',     cat:'Data',       label:'Data Table',        desc:'Sortable table with row hover and selection.' },
      { id:'sparkline',      cat:'Data',       label:'Sparkline',         desc:'Mini inline SVG chart for trends.' },
      // Overlays
      { id:'modal',          cat:'Overlays',   label:'Modal Dialog',      desc:'Centered overlay with backdrop, title, body, actions.' },
      { id:'tooltip',        cat:'Overlays',   label:'Tooltip',           desc:'Hover-triggered contextual label.' },
      { id:'popover',        cat:'Overlays',   label:'Popover',           desc:'Click-triggered floating panel.' },
      { id:'drawer',         cat:'Overlays',   label:'Drawer',            desc:'Side-sliding panel overlay.' },
      // Special
      { id:'theme-picker',   cat:'Special',    label:'Theme Picker',      desc:'6 theme swatches for live theme switching.' },
      { id:'widget-browser', cat:'Special',    label:'Widget Browser',    desc:'Searchable gallery of all OW widgets.' },
      { id:'health-dashboard',cat:'Special',   label:'Health Dashboard',  desc:'Combined health ring + metrics layout.' },
    ];

    function initWidgets() {
      WIDGETS = (typeof OW !== 'undefined' && OW.widgetDB && OW.widgetDB.length > 0)
        ? OW.widgetDB
        : FALLBACK_WIDGETS;

      // Count per category
      var cats = ['Buttons','Inputs','Cards','Navigation','Feedback','Data','Overlays','Special'];
      var total = WIDGETS.length;
      document.getElementById('widget-count-pill').textContent = total + ' widgets';
      document.getElementById('cnt-all').textContent = total;
      cats.forEach(function(c) {
        var el = document.getElementById('cnt-' + c);
        if (el) el.textContent = WIDGETS.filter(function(w) { return w.cat === c; }).length;
      });

      renderGrid();
    }

    function getFilteredWidgets() {
      return WIDGETS.filter(function(w) {
        var catOk = currentCat === 'all' || w.cat === currentCat;
        var searchOk = !currentSearch ||
          (w.label||'').toLowerCase().includes(currentSearch) ||
          (w.desc||'').toLowerCase().includes(currentSearch) ||
          (w.id||'').toLowerCase().includes(currentSearch);
        return catOk && searchOk;
      });
    }

    function renderGrid() {
      var filtered = getFilteredWidgets();
      var container = document.getElementById('grid-content');

      if (filtered.length === 0) {
        container.innerHTML = '<div class="empty-state"><div class="icon">&#128270;</div><p>No widgets found for &ldquo;' + escHtml(currentSearch) + '&rdquo;</p></div>';
        return;
      }

      // Group by category
      var groups = {};
      filtered.forEach(function(w) {
        if (!groups[w.cat]) groups[w.cat] = [];
        groups[w.cat].push(w);
      });

      var html = '';
      Object.keys(groups).forEach(function(cat) {
        var icon = WIDGET_ICONS[cat] || '&#9632;';
        html += '<div class="section-header" role="heading" aria-level="2"><h2>' + icon + ' ' + escHtml(cat) + '</h2><div class="section-divider"></div></div>';
        html += '<div class="widget-grid" role="list" aria-label="' + escHtml(cat) + ' widgets">';
        groups[cat].forEach(function(w) {
          var isSelected = selectedWidget && selectedWidget.id === w.id;
          html += renderWidgetCard(w, isSelected);
        });
        html += '</div>';
      });

      container.innerHTML = html;
    }

    function renderWidgetCard(w, isSelected) {
      var icon = WIDGET_ICONS[w.cat] || '&#9632;';
      return '<article class="wg-card' + (isSelected ? ' selected' : '') + '" role="listitem"' +
        ' tabindex="0" aria-label="' + escHtml(w.label) + ' widget"' +
        ' onclick="selectWidget(' + JSON.stringify(w.id) + ')"' +
        ' onkeydown="if(event.key===\'Enter\'||event.key===\' \') selectWidget(' + JSON.stringify(w.id) + ')">' +
        '<div class="wg-preview" aria-hidden="true"><span style="font-size:28px;">' + icon + '</span>' +
        '<span class="wg-preview-label">' + escHtml(w.id) + '</span></div>' +
        '<p class="wg-name">' + escHtml(w.label) + '</p>' +
        '<p class="wg-desc">' + escHtml((w.desc||'').substring(0, 60)) + (w.desc && w.desc.length > 60 ? '...' : '') + '</p>' +
        '<div class="wg-footer">' +
        '<span class="wg-tag">' + escHtml(w.cat) + '</span>' +
        '<button class="wg-copy-btn" onclick="event.stopPropagation(); copyById(\'' + escHtml(w.id) + '\')" aria-label="Copy ' + escHtml(w.label) + ' code">&#128203;</button>' +
        '</div></article>';
    }

    function selectWidget(id) {
      selectedWidget = WIDGETS.find(function(w) { return w.id === id; }) || null;
      if (!selectedWidget) return;

      // Update detail panel
      document.getElementById('no-selection-state').style.display = 'none';
      var dc = document.getElementById('detail-content');
      dc.style.display = 'flex';
      dc.style.flexDirection = 'column';
      dc.style.height = '100%';

      var icon = WIDGET_ICONS[selectedWidget.cat] || '&#9632;';
      document.getElementById('detail-name').textContent = selectedWidget.label;
      document.getElementById('detail-cat').textContent = selectedWidget.cat;
      document.getElementById('detail-preview').innerHTML = '<span style="font-size:42px;">' + icon + '</span>';
      document.getElementById('detail-desc').textContent = selectedWidget.desc || 'No description.';
      document.getElementById('detail-code').textContent = getCode(selectedWidget.id);
      document.getElementById('detail-css').textContent = '.ow-' + selectedWidget.id.replace(/-/g, '-');

      renderGrid(); // re-render to show selection
    }

    function getCode(id) {
      var codes = {
        'btn-primary':    "OW.btn({ label: 'Click Me', variant: 'primary', size: 'md' })",
        'btn-secondary':  "OW.btn({ label: 'Cancel', variant: 'secondary', size: 'md' })",
        'btn-ghost':      "OW.btn({ label: 'Learn More', variant: 'ghost', size: 'md' })",
        'btn-danger':     "OW.btn({ label: 'Delete', variant: 'danger', size: 'md' })",
        'btn-success':    "OW.btn({ label: 'Submit', variant: 'success', size: 'md' })",
        'btn-icon':       "OW.btn({ icon: '🔍', variant: 'ghost', size: 'sm' })",
        'btn-loading':    "OW.btn({ label: 'Saving...', loading: true, variant: 'primary' })",
        'card-basic':     "OW.card({ title: 'Title', body: 'Content', footer: 'Footer' })",
        'stat-card':      "OW.statCard({ value: '99.9%', label: 'Uptime', delta: '+0.1%' })",
        'metric':         "OW.metric({ label: 'RPS', value: '12,450', delta: '+5%' })",
        'badge':          "OW.badge({ label: 'v2.0', variant: 'primary' })",
        'chip':           "OW.chip({ label: 'Titan', removable: true })",
        'progress':       "OW.progress({ value: 75, max: 100, animated: true })",
        'toggle':         "OW.toggle({ checked: true, label: 'Enable feature' })",
        'spinner':        "OW.spinner({ size: 'md', color: 'primary' })",
        'alert-info':     "OW.alert({ type: 'info', title: 'Info', message: 'Note this.' })",
        'alert-success':  "OW.alert({ type: 'success', title: 'Done!', message: 'Success.' })",
        'alert-error':    "OW.alert({ type: 'error', title: 'Error', message: 'Failed.' })",
        'alert-warning':  "OW.alert({ type: 'warning', title: 'Warning', message: 'Careful.' })",
        'toast':          "OW.toast({ message: 'Saved!', type: 'success', duration: 3000 })",
        'tabs':           "OW.tabs({ tabs: [{ label: 'A', content: el1 }, { label: 'B', content: el2 }] })",
        'modal':          "var m = OW.modal({ title: 'Title', body: content,\n  buttons: [{ label: 'Close', onclick: () => m.close() }] })",
        'health-ring':    "OW.healthRing(85, { label: 'Health', size: 120 })",
        'theme-picker':   "OW.themePicker({ onchange: (id) => OW.switchTheme(id) })",
        'widget-browser': "OW.widgetBrowser({ onselect: (w) => console.log(w) })",
      };
      return codes[id] || 'OW.' + id + '({ /* options */ })';
    }

    function filterCat(cat) {
      currentCat = cat;
      // Update sidebar buttons
      document.querySelectorAll('.cat-btn[id^="cat-"]').forEach(function(btn) {
        var active = btn.id === 'cat-' + cat;
        btn.classList.toggle('active', active);
        btn.setAttribute('aria-pressed', active ? 'true' : 'false');
      });
      renderGrid();
    }

    function onSearch(q) {
      currentSearch = q.trim().toLowerCase();
      renderGrid();
    }

    function copyById(id) {
      var code = getCode(id);
      vscode.postMessage({ command: 'copyCode', widgetId: id });
    }

    function copyWidgetCode() {
      if (!selectedWidget) return;
      vscode.postMessage({ command: 'copyCode', widgetId: selectedWidget.id });
    }

    function openDocs() { vscode.postMessage({ command: 'openDocs' }); }

    function escHtml(s) {
      return String(s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
    }

    // Init on load
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', initWidgets);
    } else {
      initWidgets();
    }
  </script>
</body>
</html>`;
    }

    public dispose(): void {
        WidgetGalleryPanel.currentPanel = undefined;
        this._panel.dispose();
        while (this._disposables.length) {
            const d = this._disposables.pop();
            if (d) d.dispose();
        }
    }
}
