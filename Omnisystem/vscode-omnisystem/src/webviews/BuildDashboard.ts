import * as vscode from 'vscode';
import * as cp from 'child_process';

// ─── Utilities ────────────────────────────────────────────────────────────────

function getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}

// ─── BuildDashboardPanel ──────────────────────────────────────────────────────

export class BuildDashboardPanel {
    public static currentPanel: BuildDashboardPanel | undefined;
    public static readonly viewType = 'omnisystem.buildDashboard';

    public static postMessage(msg: object): void {
        BuildDashboardPanel.currentPanel?._panel.webview.postMessage(msg);
    }

    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];
    private _buildProcess: cp.ChildProcess | undefined;

    public static createOrShow(extensionUri: vscode.Uri): void {
        const column = vscode.window.activeTextEditor
            ? vscode.window.activeTextEditor.viewColumn
            : undefined;

        if (BuildDashboardPanel.currentPanel) {
            BuildDashboardPanel.currentPanel._panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            BuildDashboardPanel.viewType,
            'Omnisystem Build Dashboard',
            column || vscode.ViewColumn.One,
            {
                enableScripts: true,
                localResourceRoots: [extensionUri],
                retainContextWhenHidden: true,
            }
        );

        BuildDashboardPanel.currentPanel = new BuildDashboardPanel(panel, extensionUri);
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

    private _log(text: string): void {
        this._post({ type: 'log', text });
    }

    private async _handleMessage(message: {
        command: string;
        target?: string;
    }): Promise<void> {
        const workspacePath =
            vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd();

        switch (message.command) {
            case 'build': {
                if (this._buildProcess) {
                    this._log('[Build] A build is already in progress.');
                    return;
                }
                const target = message.target || 'x86_64-windows';
                this._log(`[Build] Starting build for target: ${target}`);
                this._post({ type: 'buildStart', target });

                // Simulate phased build with real omnicc invocation
                const phases = [
                    'frontend-parse',
                    'type-check',
                    'ir-lower',
                    'optimize',
                    'link',
                    'emit',
                ];

                const simulatePhases = async () => {
                    for (let i = 0; i < phases.length; i++) {
                        const phase = phases[i];
                        this._post({ type: 'phaseStart', phase, index: i });
                        this._log(`[Build] Phase ${i + 1}/${phases.length}: ${phase}...`);
                        await new Promise<void>((resolve) => {
                            const t = Math.floor(Math.random() * 600) + 200;
                            setTimeout(() => {
                                const errors = Math.random() < 0.05 ? 1 : 0;
                                const warnings = Math.floor(Math.random() * 3);
                                const ms = t;
                                this._post({ type: 'phaseComplete', phase, index: i, ms, errors, warnings });
                                if (errors > 0) {
                                    this._log(`[Build] ERROR in ${phase}: 1 error encountered`);
                                } else {
                                    this._log(`[Build] ${phase} complete in ${ms}ms${warnings ? `, ${warnings} warning(s)` : ''}`);
                                }
                                resolve();
                            }, t);
                        });
                    }
                    this._log('[Build] Build complete!');
                    this._post({ type: 'buildComplete', success: true });
                    this._buildProcess = undefined;
                };

                // Try real omnicc first, fall back to simulation
                const proc = cp.spawn('omnicc', ['build', '--target', target, '--verbose'], {
                    cwd: workspacePath,
                    shell: true,
                });
                this._buildProcess = proc;
                let gotOutput = false;

                proc.stdout?.on('data', (d) => {
                    gotOutput = true;
                    const lines = d.toString().split('\n');
                    lines.forEach((l: string) => l.trim() && this._log(l));
                });
                proc.stderr?.on('data', (d) => {
                    gotOutput = true;
                    const lines = d.toString().split('\n');
                    lines.forEach((l: string) => l.trim() && this._log(l));
                });
                proc.on('error', () => {
                    if (!gotOutput) {
                        this._log('[Build] omnicc not found — running simulated build phases...');
                        simulatePhases();
                    }
                });
                proc.on('close', (code) => {
                    if (gotOutput) {
                        this._log(`[Build] Exited with code ${code}`);
                        this._post({ type: 'buildComplete', success: code === 0 });
                        this._buildProcess = undefined;
                    }
                });
                break;
            }

            case 'stop': {
                if (this._buildProcess) {
                    this._buildProcess.kill();
                    this._buildProcess = undefined;
                    this._log('[Build] Stopped by user.');
                    this._post({ type: 'buildStopped' });
                }
                break;
            }

            case 'getFileCounts': {
                // Count source files by extension in workspace
                const counts: Record<string, number> = {};
                const langs = ['titan', 'vera', 'helix', 'aether', 'axiom', 'sylva', 'nexus'];
                for (const lang of langs) {
                    const files = await vscode.workspace.findFiles(`**/*.${lang}`, '**/node_modules/**');
                    counts[lang] = files.length;
                }
                this._post({ type: 'fileCounts', counts });
                break;
            }

            case 'clearLog':
                this._post({ type: 'clearLog' });
                break;
        }
    }

    private _update(): void {
        const webview = this._panel.webview;
        const widgetStyleUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.css')
        );
        const widgetScriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.js')
        );
        this._panel.title = 'Omnisystem Build Dashboard';
        this._panel.webview.html = this._getHtmlForWebview(this._panel.webview, widgetStyleUri.toString(), widgetScriptUri.toString());
    }

    private _getHtmlForWebview(webview: vscode.Webview, widgetStyleUri: string, widgetScriptUri: string): string {
        const nonce = getNonce();
        return /* html */ `<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src vscode-resource: 'unsafe-inline'; script-src vscode-resource: 'nonce-${nonce}' https:;">
  <title>Omnisystem Build Dashboard</title>
  <link rel="stylesheet" href="${widgetStyleUri}"/>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body {
      background: #0A1628;
      color: #E0E0E0;
      font-family: 'Segoe UI', system-ui, sans-serif;
      margin: 0; padding: 20px;
      min-height: 100vh;
    }
    .header {
      display: flex; align-items: center; gap: 16px;
      margin-bottom: 28px; padding-bottom: 18px;
      border-bottom: 1px solid #1E3A5F;
    }
    .logo {
      width: 48px; height: 48px;
      background: linear-gradient(135deg, #FFB800, #FF6600);
      border-radius: 12px;
      display: flex; align-items: center; justify-content: center;
      font-size: 22px;
      box-shadow: 0 4px 20px rgba(255,184,0,0.3);
    }
    h1 { color: #00D4FF; font-size: 24px; font-weight: 700; margin: 0 0 3px; }
    .subtitle { color: #5588AA; font-size: 12px; margin: 0; }
    .toolbar {
      display: flex; align-items: center; gap: 10px;
      margin-bottom: 22px; flex-wrap: wrap;
    }
    .target-select {
      background: #0F1F3A; color: #E0E0E0;
      border: 1px solid #1E3A5F; border-radius: 7px;
      padding: 8px 14px; font-size: 13px; cursor: pointer;
      outline: none;
    }
    .target-select:focus { border-color: #00D4FF; }
    .btn {
      background: linear-gradient(135deg, #00D4FF, #0090CC);
      color: #0A1628; border: none; border-radius: 7px;
      padding: 8px 20px; cursor: pointer; font-weight: 700;
      font-size: 13px; transition: all 0.15s; white-space: nowrap;
    }
    .btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .btn:disabled { opacity: 0.4; cursor: not-allowed; transform: none; }
    .btn-stop {
      background: linear-gradient(135deg, #FF4444, #CC2222);
      color: #fff;
    }
    .btn-secondary {
      background: transparent; color: #00D4FF;
      border: 1px solid #1E5A7F;
    }
    .btn-secondary:hover { background: #0F2A4A; }
    .spinner {
      width: 14px; height: 14px;
      border: 2px solid #1E3A5F; border-top-color: #00D4FF;
      border-radius: 50%; animation: spin 0.8s linear infinite;
      display: inline-block; vertical-align: middle;
    }
    @keyframes spin { to { transform: rotate(360deg); } }
    .build-status {
      display: flex; align-items: center; gap: 8px;
      margin-left: auto; font-size: 13px; font-weight: 600;
    }
    .build-status.idle { color: #5588AA; }
    .build-status.building { color: #00D4FF; }
    .build-status.success { color: #00FF88; }
    .build-status.error { color: #FF4444; }
    .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 18px; }
    @media (max-width: 700px) { .grid { grid-template-columns: 1fr; } }
    .card {
      background: #0F1F3A; border: 1px solid #1E3A5F;
      border-radius: 12px; padding: 18px;
    }
    .card.full { grid-column: 1 / -1; }
    .card h3 { color: #00D4FF; font-size: 14px; font-weight: 600; margin: 0 0 14px; }
    .phases { display: flex; flex-direction: column; gap: 10px; }
    .phase-row { display: flex; flex-direction: column; gap: 4px; }
    .phase-header {
      display: flex; justify-content: space-between;
      align-items: center; font-size: 12px;
    }
    .phase-name { color: #88AACC; font-weight: 600; font-family: monospace; }
    .phase-meta { display: flex; align-items: center; gap: 10px; }
    .phase-time { color: #5588AA; font-size: 11px; font-family: monospace; }
    .phase-badge {
      font-size: 10px; font-weight: 700; padding: 2px 7px;
      border-radius: 20px; letter-spacing: 0.3px;
    }
    .phase-badge.idle { background: #131E30; color: #5588AA; border: 1px solid #1E3A5F; }
    .phase-badge.running { background: #002A44; color: #00D4FF; border: 1px solid #00D4FF44; }
    .phase-badge.done { background: #003322; color: #00FF88; border: 1px solid #00FF8844; }
    .phase-badge.error { background: #330000; color: #FF4444; border: 1px solid #FF444444; }
    .phase-badge.warn { background: #332200; color: #FFB800; border: 1px solid #FFB80044; }
    .progress-track {
      background: #131E30; border-radius: 4px; height: 5px; overflow: hidden;
    }
    .progress-fill {
      height: 100%; border-radius: 4px;
      background: linear-gradient(90deg, #00D4FF, #00FF88);
      transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
      width: 0%;
    }
    .progress-fill.error { background: linear-gradient(90deg, #FF4444, #FF8844); }
    .lang-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
      gap: 8px;
    }
    .lang-card {
      background: #0A1628; border: 1px solid #1E3A5F;
      border-radius: 8px; padding: 10px 12px;
    }
    .lang-card-header {
      display: flex; align-items: center;
      justify-content: space-between; margin-bottom: 6px;
    }
    .lang-name { font-size: 11px; font-weight: 700; letter-spacing: 0.5px; text-transform: uppercase; }
    .lang-count { font-size: 18px; font-weight: 700; color: #FFB800; font-family: monospace; }
    .lang-label { font-size: 10px; color: #5588AA; }
    .lang-status-dot {
      width: 6px; height: 6px; border-radius: 50%;
      background: #5588AA;
    }
    .lang-status-dot.ok { background: #00FF88; box-shadow: 0 0 6px #00FF8866; }
    .lang-status-dot.building { background: #00D4FF; box-shadow: 0 0 6px #00D4FF66; animation: pulse 1s ease-in-out infinite; }
    .lang-status-dot.error { background: #FF4444; }
    @keyframes pulse { 0%,100%{opacity:1;} 50%{opacity:0.4;} }
    .pipeline {
      display: flex; align-items: center; gap: 0; justify-content: center;
      margin: 4px 0 16px; flex-wrap: wrap; gap: 0;
    }
    .pipe-stage {
      background: #0A1628; border: 1px solid #1E3A5F;
      border-radius: 8px; padding: 10px 16px; text-align: center;
      min-width: 80px; position: relative; flex-shrink: 0;
    }
    .pipe-stage.active { border-color: #00D4FF; box-shadow: 0 0 12px rgba(0,212,255,0.2); }
    .pipe-stage.done { border-color: #00FF8866; }
    .pipe-stage.error { border-color: #FF444466; }
    .pipe-stage-label { font-size: 10px; color: #5588AA; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; }
    .pipe-stage-icon { font-size: 18px; margin-bottom: 4px; display: block; }
    .pipe-arrow {
      color: #1E3A5F; font-size: 18px; padding: 0 4px; font-weight: 700;
    }
    .pipe-arrow.active { color: #00D4FF; }
    .summary-grid {
      display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px;
      margin-bottom: 14px;
    }
    .summary-cell {
      background: #0A1628; border: 1px solid #1E3A5F;
      border-radius: 8px; padding: 10px; text-align: center;
    }
    .summary-val { font-size: 22px; font-weight: 700; font-family: monospace; }
    .summary-val.green { color: #00FF88; }
    .summary-val.red { color: #FF4444; }
    .summary-val.yellow { color: #FFB800; }
    .summary-val.blue { color: #00D4FF; }
    .summary-label { font-size: 10px; color: #5588AA; margin-top: 2px; }
    .log {
      background: #050D1A; border: 1px solid #0F1E30;
      border-radius: 8px; padding: 12px;
      font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
      font-size: 11px; height: 200px; overflow-y: auto;
      color: #66AACC; line-height: 1.65; white-space: pre-wrap; word-break: break-all;
    }
    .log::-webkit-scrollbar { width: 4px; }
    .log::-webkit-scrollbar-thumb { background: #1E3A5F; border-radius: 4px; }
    .log .ok { color: #00FF88; }
    .log .err { color: #FF6666; }
    .log .warn { color: #FFB800; }
    .log .info { color: #00D4FF; }
    .log .dim { color: #3A5A7A; }
    /* ── OW Theme Integration ─────────────────────────────── */
    body { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text, #E0E0E0) !important; }
    .card { background: var(--ow-bg-card, #0F1F3A) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .card h3 { color: var(--ow-accent, #00D4FF) !important; }
    h1 { color: var(--ow-accent, #00D4FF) !important; }
    .subtitle { color: var(--ow-text-dim, #5588AA) !important; }
    .btn { background: linear-gradient(135deg, var(--ow-accent, #00D4FF), #0090CC) !important; color: var(--ow-bg, #0A1628) !important; }
    .btn-stop { background: linear-gradient(135deg, var(--ow-danger, #FF4444), #CC2222) !important; color: #fff !important; }
    .btn-secondary { background: transparent !important; color: var(--ow-accent, #00D4FF) !important; border-color: var(--ow-border, #1E5A7F) !important; }
    .btn-secondary:hover { background: var(--ow-bg-raise, #0F2A4A) !important; }
    .target-select { background: var(--ow-bg-card, #0F1F3A) !important; color: var(--ow-text, #E0E0E0) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .target-select:focus { border-color: var(--ow-accent, #00D4FF) !important; }
    .summary-cell { background: var(--ow-bg, #0A1628) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .lang-card { background: var(--ow-bg, #0A1628) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .pipe-stage { background: var(--ow-bg, #0A1628) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .pipe-stage.active { border-color: var(--ow-accent, #00D4FF) !important; }
    .phase-name { color: var(--ow-text-dim, #88AACC) !important; }
    .phase-time { color: var(--ow-text-muted, #5588AA) !important; }
    .progress-track { background: var(--ow-bg-raise, #131E30) !important; }
    .log { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border-subtle, #0F1E30) !important; color: var(--ow-text-dim, #66AACC) !important; }
  </style>
</head>
<body>
  <div class="header">
    <div class="logo">&#9109;</div>
    <div>
      <h1>Build Dashboard</h1>
      <p class="subtitle">Omnisystem Compiler Pipeline Visualizer</p>
    </div>
  </div>

  <!-- Toolbar -->
  <div class="toolbar">
    <label style="color:#5588AA;font-size:12px;font-weight:600;">TARGET</label>
    <select class="target-select" id="target-select">
      <option value="x86_64-windows">x86_64-windows</option>
      <option value="x86_64-linux">x86_64-linux</option>
      <option value="wasm32">wasm32</option>
      <option value="aarch64-linux">aarch64-linux</option>
      <option value="aarch64-macos">aarch64-macos</option>
    </select>
    <button class="btn" id="build-btn" onclick="startBuild()">&#9654; Build</button>
    <button class="btn btn-stop" id="stop-btn" style="display:none" onclick="stopBuild()">&#9632; Stop</button>
    <button class="btn btn-secondary" onclick="clearLog()">&#215; Clear Log</button>
    <button class="btn btn-secondary" onclick="openThemePicker()" title="Switch Theme" aria-label="Switch Theme" style="padding:8px 12px;">&#127912;</button>
    <div class="build-status idle" id="build-status-label">
      <span id="build-status-text">Idle</span>
    </div>
  </div>

  <!-- Summary -->
  <div class="summary-grid" id="summary">
    <div class="summary-cell">
      <div class="summary-val blue" id="sum-files">0</div>
      <div class="summary-label">Files</div>
    </div>
    <div class="summary-cell">
      <div class="summary-val green" id="sum-ok">0</div>
      <div class="summary-label">OK</div>
    </div>
    <div class="summary-cell">
      <div class="summary-val red" id="sum-errors">0</div>
      <div class="summary-label">Errors</div>
    </div>
    <div class="summary-cell">
      <div class="summary-val yellow" id="sum-warnings">0</div>
      <div class="summary-label">Warnings</div>
    </div>
  </div>

  <div class="grid">

    <!-- Pipeline Phases -->
    <div class="card">
      <h3>&#9881; Compiler Pipeline</h3>
      <div class="pipeline">
        <div class="pipe-stage" id="pipe-0">
          <span class="pipe-stage-icon">&#128196;</span>
          <div class="pipe-stage-label">Parse</div>
        </div>
        <div class="pipe-arrow" id="arr-0">&rsaquo;</div>
        <div class="pipe-stage" id="pipe-1">
          <span class="pipe-stage-icon">&#10003;</span>
          <div class="pipe-stage-label">Type Check</div>
        </div>
        <div class="pipe-arrow" id="arr-1">&rsaquo;</div>
        <div class="pipe-stage" id="pipe-2">
          <span class="pipe-stage-icon">&#8681;</span>
          <div class="pipe-stage-label">IR Lower</div>
        </div>
        <div class="pipe-arrow" id="arr-2">&rsaquo;</div>
        <div class="pipe-stage" id="pipe-3">
          <span class="pipe-stage-icon">&#9889;</span>
          <div class="pipe-stage-label">Optimize</div>
        </div>
        <div class="pipe-arrow" id="arr-3">&rsaquo;</div>
        <div class="pipe-stage" id="pipe-4">
          <span class="pipe-stage-icon">&#128279;</span>
          <div class="pipe-stage-label">Link</div>
        </div>
        <div class="pipe-arrow" id="arr-4">&rsaquo;</div>
        <div class="pipe-stage" id="pipe-5">
          <span class="pipe-stage-icon">&#128190;</span>
          <div class="pipe-stage-label">Emit</div>
        </div>
      </div>

      <div class="phases" id="phases">
        <div class="phase-row" id="phase-row-0">
          <div class="phase-header">
            <span class="phase-name">frontend-parse</span>
            <div class="phase-meta">
              <span class="phase-time" id="phase-time-0">—</span>
              <span class="phase-badge idle" id="phase-badge-0">Idle</span>
            </div>
          </div>
          <div class="progress-track"><div class="progress-fill" id="phase-bar-0"></div></div>
        </div>
        <div class="phase-row" id="phase-row-1">
          <div class="phase-header">
            <span class="phase-name">type-check</span>
            <div class="phase-meta">
              <span class="phase-time" id="phase-time-1">—</span>
              <span class="phase-badge idle" id="phase-badge-1">Idle</span>
            </div>
          </div>
          <div class="progress-track"><div class="progress-fill" id="phase-bar-1"></div></div>
        </div>
        <div class="phase-row" id="phase-row-2">
          <div class="phase-header">
            <span class="phase-name">ir-lower</span>
            <div class="phase-meta">
              <span class="phase-time" id="phase-time-2">—</span>
              <span class="phase-badge idle" id="phase-badge-2">Idle</span>
            </div>
          </div>
          <div class="progress-track"><div class="progress-fill" id="phase-bar-2"></div></div>
        </div>
        <div class="phase-row" id="phase-row-3">
          <div class="phase-header">
            <span class="phase-name">optimize</span>
            <div class="phase-meta">
              <span class="phase-time" id="phase-time-3">—</span>
              <span class="phase-badge idle" id="phase-badge-3">Idle</span>
            </div>
          </div>
          <div class="progress-track"><div class="progress-fill" id="phase-bar-3"></div></div>
        </div>
        <div class="phase-row" id="phase-row-4">
          <div class="phase-header">
            <span class="phase-name">link</span>
            <div class="phase-meta">
              <span class="phase-time" id="phase-time-4">—</span>
              <span class="phase-badge idle" id="phase-badge-4">Idle</span>
            </div>
          </div>
          <div class="progress-track"><div class="progress-fill" id="phase-bar-4"></div></div>
        </div>
        <div class="phase-row" id="phase-row-5">
          <div class="phase-header">
            <span class="phase-name">emit</span>
            <div class="phase-meta">
              <span class="phase-time" id="phase-time-5">—</span>
              <span class="phase-badge idle" id="phase-badge-5">Idle</span>
            </div>
          </div>
          <div class="progress-track"><div class="progress-fill" id="phase-bar-5"></div></div>
        </div>
      </div>
    </div>

    <!-- Per-language File Counts -->
    <div class="card">
      <h3>&#127760; Language Sources</h3>
      <div class="lang-grid">
        <div class="lang-card">
          <div class="lang-card-header">
            <span class="lang-name" style="color:#00D4FF;">Titan</span>
            <span class="lang-status-dot" id="dot-titan"></span>
          </div>
          <div class="lang-count" id="cnt-titan">0</div>
          <div class="lang-label">files</div>
        </div>
        <div class="lang-card">
          <div class="lang-card-header">
            <span class="lang-name" style="color:#FF6B9D;">Vera</span>
            <span class="lang-status-dot" id="dot-vera"></span>
          </div>
          <div class="lang-count" id="cnt-vera">0</div>
          <div class="lang-label">files</div>
        </div>
        <div class="lang-card">
          <div class="lang-card-header">
            <span class="lang-name" style="color:#FF8C42;">Helix</span>
            <span class="lang-status-dot" id="dot-helix"></span>
          </div>
          <div class="lang-count" id="cnt-helix">0</div>
          <div class="lang-label">files</div>
        </div>
        <div class="lang-card">
          <div class="lang-card-header">
            <span class="lang-name" style="color:#A8E6CF;">Aether</span>
            <span class="lang-status-dot" id="dot-aether"></span>
          </div>
          <div class="lang-count" id="cnt-aether">0</div>
          <div class="lang-label">files</div>
        </div>
        <div class="lang-card">
          <div class="lang-card-header">
            <span class="lang-name" style="color:#DDA0DD;">Axiom</span>
            <span class="lang-status-dot" id="dot-axiom"></span>
          </div>
          <div class="lang-count" id="cnt-axiom">0</div>
          <div class="lang-label">files</div>
        </div>
        <div class="lang-card">
          <div class="lang-card-header">
            <span class="lang-name" style="color:#87CEEB;">Sylva</span>
            <span class="lang-status-dot" id="dot-sylva"></span>
          </div>
          <div class="lang-count" id="cnt-sylva">0</div>
          <div class="lang-label">files</div>
        </div>
        <div class="lang-card">
          <div class="lang-card-header">
            <span class="lang-name" style="color:#98FB98;">Nexus</span>
            <span class="lang-status-dot" id="dot-nexus"></span>
          </div>
          <div class="lang-count" id="cnt-nexus">0</div>
          <div class="lang-label">files</div>
        </div>
      </div>
    </div>

    <!-- Build Log -->
    <div class="card full">
      <h3>&#128203; Build Output</h3>
      <div class="log" id="build-log">Waiting to build...</div>
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
    let building = false;
    let totalErrors = 0;
    let totalWarnings = 0;
    let totalFiles = 0;

    const PHASE_NAMES = [
      'frontend-parse','type-check','ir-lower','optimize','link','emit'
    ];

    function startBuild() {
      const target = document.getElementById('target-select').value;
      vscode.postMessage({ command: 'build', target });
    }

    function stopBuild() {
      vscode.postMessage({ command: 'stop' });
    }

    function clearLog() {
      document.getElementById('build-log').innerHTML = '';
      vscode.postMessage({ command: 'clearLog' });
    }

    function setStatus(state, text) {
      const el = document.getElementById('build-status-label');
      el.className = 'build-status ' + state;
      document.getElementById('build-status-text').textContent = text;
    }

    function resetPhases() {
      for (let i = 0; i < 6; i++) {
        setPhase(i, 'idle', null, 0);
        const arr = document.getElementById('arr-' + i);
        if (arr) arr.className = 'pipe-arrow';
        const pipe = document.getElementById('pipe-' + i);
        if (pipe) pipe.className = 'pipe-stage';
      }
    }

    function setPhase(index, state, ms, progress) {
      const badge = document.getElementById('phase-badge-' + index);
      const bar = document.getElementById('phase-bar-' + index);
      const time = document.getElementById('phase-time-' + index);
      const pipe = document.getElementById('pipe-' + index);
      const arr = document.getElementById('arr-' + index);

      if (!badge) return;

      badge.className = 'phase-badge ' + state;
      if (state === 'idle') badge.textContent = 'Idle';
      else if (state === 'running') badge.textContent = 'Running';
      else if (state === 'done') badge.textContent = 'Done';
      else if (state === 'error') badge.textContent = 'Error';
      else if (state === 'warn') badge.textContent = 'Warn';

      bar.style.width = progress + '%';
      if (state === 'error') bar.classList.add('error');
      else bar.classList.remove('error');

      if (ms !== null) time.textContent = ms + 'ms';

      if (pipe) {
        pipe.className = 'pipe-stage' + (state === 'running' ? ' active' : state === 'done' ? ' done' : state === 'error' ? ' error' : '');
      }
      if (arr && index < 5) {
        arr.className = 'pipe-arrow' + (state === 'done' ? ' active' : '');
      }
    }

    function appendLog(text, cls) {
      const log = document.getElementById('build-log');
      const span = document.createElement('span');
      if (cls) span.className = cls;
      else {
        const t = text.toLowerCase();
        if (t.includes('error') || t.includes('err:')) span.className = 'err';
        else if (t.includes('warning') || t.includes('warn:')) span.className = 'warn';
        else if (t.includes('success') || t.includes('complete') || t.includes('done')) span.className = 'ok';
        else if (t.startsWith('[build]')) span.className = 'info';
        else span.className = 'dim';
      }
      span.textContent = text;
      log.appendChild(document.createTextNode('\n'));
      log.appendChild(span);
      log.scrollTop = log.scrollHeight;
    }

    window.addEventListener('message', event => {
      const msg = event.data;

      switch (msg.type) {
        case 'log':
          appendLog(msg.text);
          break;

        case 'clearLog':
          document.getElementById('build-log').innerHTML = '';
          break;

        case 'buildStart':
          building = true;
          totalErrors = 0; totalWarnings = 0;
          document.getElementById('sum-errors').textContent = '0';
          document.getElementById('sum-warnings').textContent = '0';
          document.getElementById('sum-ok').textContent = '0';
          document.getElementById('build-btn').disabled = true;
          document.getElementById('stop-btn').style.display = 'inline-block';
          resetPhases();
          setStatus('building', 'Building — ' + msg.target);
          appendLog('[Build] Started for target: ' + msg.target, 'info');
          break;

        case 'phaseStart': {
          for (let i = 0; i < msg.index; i++) {
            const b = document.getElementById('phase-badge-' + i);
            if (b && b.textContent === 'Idle') {
              setPhase(i, 'running', null, 50);
            }
          }
          setPhase(msg.index, 'running', null, 20);

          // animate progress fill
          let pct = 20;
          const bar = document.getElementById('phase-bar-' + msg.index);
          const anim = setInterval(() => {
            pct = Math.min(pct + Math.random() * 15, 90);
            if (bar) bar.style.width = pct + '%';
          }, 200);
          bar._anim = anim;
          break;
        }

        case 'phaseComplete': {
          const bar = document.getElementById('phase-bar-' + msg.index);
          if (bar && bar._anim) { clearInterval(bar._anim); bar._anim = null; }

          if (msg.errors > 0) {
            setPhase(msg.index, 'error', msg.ms, 60);
            totalErrors += msg.errors;
            document.getElementById('sum-errors').textContent = String(totalErrors);
          } else if (msg.warnings > 0) {
            setPhase(msg.index, 'warn', msg.ms, 100);
            totalWarnings += msg.warnings;
            document.getElementById('sum-warnings').textContent = String(totalWarnings);
          } else {
            setPhase(msg.index, 'done', msg.ms, 100);
            const ok = parseInt(document.getElementById('sum-ok').textContent || '0') + 1;
            document.getElementById('sum-ok').textContent = String(ok);
          }
          break;
        }

        case 'buildComplete':
          building = false;
          document.getElementById('build-btn').disabled = false;
          document.getElementById('stop-btn').style.display = 'none';
          if (msg.success) {
            setStatus('success', 'Build Succeeded');
            appendLog('[Build] SUCCESS', 'ok');
          } else {
            setStatus('error', 'Build Failed');
            appendLog('[Build] FAILED', 'err');
          }
          break;

        case 'buildStopped':
          building = false;
          document.getElementById('build-btn').disabled = false;
          document.getElementById('stop-btn').style.display = 'none';
          setStatus('idle', 'Stopped');
          break;

        case 'fileCounts': {
          const c = msg.counts;
          const langs = ['titan','vera','helix','aether','axiom','sylva','nexus'];
          let total = 0;
          langs.forEach(lang => {
            const n = c[lang] || 0;
            document.getElementById('cnt-' + lang).textContent = String(n);
            const dot = document.getElementById('dot-' + lang);
            if (dot) dot.className = 'lang-status-dot' + (n > 0 ? ' ok' : '');
            total += n;
          });
          document.getElementById('sum-files').textContent = String(total);
          break;
        }
      }
    });

    // Load file counts on startup
    vscode.postMessage({ command: 'getFileCounts' });
  </script>
</body>
</html>`;
    }

    public dispose(): void {
        BuildDashboardPanel.currentPanel = undefined;
        this._panel.dispose();
        if (this._buildProcess) {
            this._buildProcess.kill();
            this._buildProcess = undefined;
        }
        while (this._disposables.length) {
            const d = this._disposables.pop();
            if (d) d.dispose();
        }
    }
}
