import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as os from 'os';
import * as fs from 'fs';
import * as path from 'path';

// ─── Utilities ────────────────────────────────────────────────────────────────

function getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}

// ─── OmnisystemDashboardPanel ─────────────────────────────────────────────────────

export class OmnisystemDashboardPanel {
    public static currentPanel: OmnisystemDashboardPanel | undefined;
    public static readonly viewType = 'omnisystem.desktopDashboard';

    public static postMessage(msg: object): void {
        OmnisystemDashboardPanel.currentPanel?._panel.webview.postMessage(msg);
    }

    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];
    private _buildProcess: cp.ChildProcess | undefined;

    public static createOrShow(extensionUri: vscode.Uri): void {
        const column = vscode.window.activeTextEditor
            ? vscode.window.activeTextEditor.viewColumn
            : undefined;

        if (OmnisystemDashboardPanel.currentPanel) {
            OmnisystemDashboardPanel.currentPanel._panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            OmnisystemDashboardPanel.viewType,
            'Omnisystem Ecosystem Dashboard',
            column || vscode.ViewColumn.One,
            {
                enableScripts: true,
                localResourceRoots: [extensionUri],
                retainContextWhenHidden: true,
            }
        );

        OmnisystemDashboardPanel.currentPanel = new OmnisystemDashboardPanel(panel, extensionUri);
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

    private _run(
        cmd: string,
        args: string[],
        cwd?: string,
        label = 'Build'
    ): void {
        if (this._buildProcess) {
            this._log(`[${label}] Another process is running. Stop it first.`);
            return;
        }
        this._log(`[${label}] > ${cmd} ${args.join(' ')}`);
        this._post({ type: 'building', value: true });

        const proc = cp.spawn(cmd, args, {
            cwd: cwd || process.cwd(),
            shell: true,
        });
        this._buildProcess = proc;

        proc.stdout?.on('data', (d) => this._log(d.toString().trimEnd()));
        proc.stderr?.on('data', (d) => this._log(d.toString().trimEnd()));
        proc.on('close', (code) => {
            this._log(`[${label}] Exited with code ${code}`);
            this._post({ type: 'building', value: false });
            this._buildProcess = undefined;
        });
    }

    private async _handleMessage(message: { command: string; arg?: string }): Promise<void> {
        const workspacePath =
            vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd();

        switch (message.command) {
            case 'getStatus': {
                const platform =
                    os.platform() === 'win32' ? 'Windows x64' :
                    os.platform() === 'darwin' ? 'macOS' : 'Linux x64';

                let adbStatus = 'Not found';
                let adbDevice = '—';
                try {
                    const result = cp.execSync('adb devices', { timeout: 3000 }).toString();
                    const lines = result.split('\n').filter(l => l.trim() && !l.startsWith('List'));
                    if (lines.length > 0) {
                        adbStatus = 'Connected';
                        adbDevice = lines[0].split('\t')[0].trim();
                    } else {
                        adbStatus = 'No devices';
                    }
                } catch {
                    adbStatus = 'ADB not found';
                }

                // Check Control Panel reachability
                let cpOnline = false;
                try {
                    cp.execSync('curl -s --max-time 1 http://localhost:12345/api/v1/system', { timeout: 2000 });
                    cpOnline = true;
                } catch { /* offline */ }

                this._post({
                    type: 'status',
                    launcher: { platform },
                    buddy: { adb: adbStatus, device: adbDevice },
                    controlPanel: { online: cpOnline },
                });
                break;
            }

            case 'getEcosystemFiles': {
                const wsPath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
                const ecoBase = wsPath
                    ? path.join(wsPath, 'src', 'systems', 'desktop')
                    : null;

                const titanFiles = [
                    { name: 'INITIALIZATION.ti',         rel: 'INITIALIZATION.ti' },
                    { name: 'control-panel/core.ti',     rel: path.join('control-panel', 'core.ti') },
                    { name: 'control-panel/api_server.ti', rel: path.join('control-panel', 'api_server.ti') },
                    { name: 'notifications/notification_daemon.ti', rel: path.join('notifications', 'notification_daemon.ti') },
                    { name: 'system-tray/core.ti',       rel: path.join('system-tray', 'core.ti') },
                    { name: 'file-associations/core.ti', rel: path.join('file-associations', 'core.ti') },
                    { name: 'theme-system/core.ti',      rel: path.join('theme-system', 'core.ti') },
                    { name: 'installer/core.ti',         rel: path.join('installer', 'core.ti') },
                    { name: 'installer/host_detection.ti', rel: path.join('installer', 'host_detection.ti') },
                    { name: 'integration/omnisystem_integration.ti', rel: path.join('integration', 'omnisystem_integration.ti') },
                ];

                const results = titanFiles.map(f => {
                    if (!ecoBase) { return { name: f.name, loc: 0, found: false }; }
                    const fullPath = path.join(ecoBase, f.rel);
                    if (!fs.existsSync(fullPath)) { return { name: f.name, loc: 0, found: false }; }
                    try {
                        const content = fs.readFileSync(fullPath, 'utf8');
                        return { name: f.name, loc: content.split('\n').length, found: true };
                    } catch {
                        return { name: f.name, loc: 0, found: false };
                    }
                });

                const totalLoc = results.reduce((s, r) => s + r.loc, 0);
                this._post({ type: 'ecosystemFiles', files: results, totalLoc, ecoFound: !!ecoBase && fs.existsSync(ecoBase) });
                break;
            }

            case 'omnisystemLaunch':
                this._run('npx', ['tauri', 'dev'], workspacePath, 'Omnisystem Launch');
                break;

            case 'omnisystemBuild':
                this._run('npx', ['tauri', 'build'], workspacePath, 'Omnisystem Build');
                break;

            case 'omnisystemBuddyBuild':
                this._run(
                    'npx',
                    ['react-native', 'build-android', '--mode=release'],
                    workspacePath,
                    'Buddy APK'
                );
                break;

            case 'omnisystemBuddyConnect':
                this._run('adb', ['reverse', 'tcp:8081', 'tcp:8081'], workspacePath, 'ADB Connect');
                break;

            case 'openBuddyApp':
                vscode.window.showInformationMessage(
                    `Opening Omnisystem Buddy sub-app: ${message.arg}`
                );
                break;

            case 'omnisystemBrowserExtBuild':
                this._run(
                    'npm',
                    ['run', 'build'],
                    workspacePath,
                    'Browser Ext Build'
                );
                break;

            case 'omnisystemBrowserExtInstall':
                vscode.window.showInformationMessage(
                    'Load unpacked extension from the dist/ folder in Chrome at chrome://extensions'
                );
                break;

            case 'omnisystemControlPanel':
                // Control Panel Titan server runs on port 12345 (api_server.ti)
                vscode.env.openExternal(vscode.Uri.parse('http://localhost:12345'));
                break;

            case 'omnisystemControlPanelStart':
                this._run('npm', ['run', 'dev'], workspacePath, 'Control Panel');
                break;

            case 'omnisystemSharedUiBuild':
                this._run('npm', ['run', 'build:lib'], workspacePath, 'Shared UI');
                break;

            case 'omnisystemNotifications':
                this._log('[Omnisystem] Notification System — notifications/notification_daemon.ti');
                vscode.commands.executeCommand('omnisystem.desktopNotifications');
                break;

            case 'omnisystemSystemTray':
                this._log('[Omnisystem] System Tray — system-tray/core.ti');
                vscode.commands.executeCommand('omnisystem.desktopSystemTray');
                break;

            case 'omnisystemInit':
                this._log('[Omnisystem Init] Triggering ecosystem initialization — INITIALIZATION.ti');
                vscode.commands.executeCommand('omnisystem.desktopInit');
                break;

            case 'omnisystemDiagnostics':
                this._log('[Omnisystem] Running diagnostics mode — INITIALIZATION.ti::omnisystem_ecosystem_diagnostics()');
                vscode.window.showInformationMessage('Omnisystem Ecosystem: Diagnostics mode triggered.');
                break;

            case 'omnisystemRepair':
                this._log('[Omnisystem] Running repair mode — INITIALIZATION.ti::omnisystem_ecosystem_repair()');
                vscode.window.showInformationMessage('Omnisystem Ecosystem: Repair mode triggered.');
                break;

            case 'omnisystemDeployAll':
            case 'deployAll':
                this._log('[Deploy] Starting full Omnisystem release build...');
                this._run('npm', ['run', 'build:all'], workspacePath, 'Deploy All');
                break;

            case 'stopBuild':
                if (this._buildProcess) {
                    this._buildProcess.kill();
                    this._buildProcess = undefined;
                    this._log('[Build] Stopped by user.');
                    this._post({ type: 'building', value: false });
                }
                break;

            case 'refresh':
                this._handleMessage({ command: 'getStatus' });
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
        this._panel.title = 'Omnisystem Ecosystem Dashboard';
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
  <title>Omnisystem Ecosystem Dashboard</title>
  <link rel="stylesheet" href="${widgetStyleUri}"/>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body {
      background: #0A1628;
      color: #E0E0E0;
      font-family: 'Segoe UI', system-ui, sans-serif;
      margin: 0;
      padding: 20px;
      min-height: 100vh;
    }
    .header {
      display: flex;
      align-items: center;
      gap: 16px;
      margin-bottom: 32px;
      padding-bottom: 20px;
      border-bottom: 1px solid #1E3A5F;
    }
    .logo {
      width: 52px; height: 52px;
      background: linear-gradient(135deg, #00D4FF, #0060FF);
      border-radius: 14px;
      display: flex; align-items: center; justify-content: center;
      font-size: 26px;
      flex-shrink: 0;
      box-shadow: 0 4px 20px rgba(0,212,255,0.3);
    }
    .header-text h1 {
      color: #00D4FF;
      font-size: 26px;
      font-weight: 700;
      margin: 0 0 4px;
      letter-spacing: -0.5px;
    }
    .header-text p { color: #5588AA; margin: 0; font-size: 13px; }
    .header-actions { margin-left: auto; display: flex; gap: 10px; align-items: center; }
    .build-indicator {
      display: none;
      align-items: center;
      gap: 6px;
      color: #00FF88;
      font-size: 12px;
      font-weight: 600;
    }
    .build-indicator.active { display: flex; }
    .spinner {
      width: 12px; height: 12px;
      border: 2px solid #1E3A5F;
      border-top-color: #00FF88;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }
    @keyframes spin { to { transform: rotate(360deg); } }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
      gap: 20px;
    }
    .card {
      background: #0F1F3A;
      border: 1px solid #1E3A5F;
      border-radius: 14px;
      padding: 20px;
      transition: border-color 0.2s;
    }
    .card:hover { border-color: #2A5A8F; }
    .card h3 {
      color: #00D4FF;
      margin: 0 0 6px;
      font-size: 15px;
      font-weight: 600;
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .card-desc { color: #5588AA; font-size: 12px; margin: 0 0 16px; }
    .status-dot {
      width: 8px; height: 8px;
      border-radius: 50%;
      display: inline-block;
      flex-shrink: 0;
    }
    .status-dot.active { background: #00FF88; box-shadow: 0 0 8px #00FF8866; }
    .status-dot.inactive { background: #FF4444; }
    .status-dot.pending { background: #FFB800; box-shadow: 0 0 8px #FFB80066; }
    .btn {
      background: linear-gradient(135deg, #00D4FF, #0090CC);
      color: #0A1628;
      border: none;
      border-radius: 7px;
      padding: 8px 16px;
      cursor: pointer;
      font-weight: 700;
      font-size: 12px;
      letter-spacing: 0.3px;
      transition: all 0.15s;
      white-space: nowrap;
    }
    .btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .btn:active { transform: translateY(0); }
    .btn-secondary {
      background: transparent;
      color: #00D4FF;
      border: 1px solid #1E5A7F;
    }
    .btn-secondary:hover { background: #0F2A4A; border-color: #00D4FF; }
    .btn-danger {
      background: transparent;
      color: #FF6666;
      border: 1px solid #5F1E1E;
    }
    .btn-danger:hover { background: #2A0F0F; border-color: #FF4444; }
    .metric {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 9px 0;
      border-bottom: 1px solid #131E30;
      font-size: 13px;
    }
    .metric:last-child { border-bottom: none; }
    .metric-label { color: #7799BB; }
    .metric-value { color: #FFB800; font-weight: 600; font-size: 12px; font-family: monospace; }
    .app-grid {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 6px;
      margin-top: 12px;
    }
    .app-chip {
      background: #132035;
      border: 1px solid #1E3A5F;
      border-radius: 7px;
      padding: 7px 6px;
      font-size: 11px;
      text-align: center;
      cursor: pointer;
      color: #88BBDD;
      transition: all 0.15s;
      user-select: none;
    }
    .app-chip:hover { background: #1E3A5F; color: #00D4FF; border-color: #00D4FF44; }
    .actions { display: flex; gap: 8px; margin-top: 16px; flex-wrap: wrap; }
    .log {
      background: #050D1A;
      border: 1px solid #0F1E30;
      border-radius: 8px;
      padding: 12px;
      font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
      font-size: 11px;
      height: 150px;
      overflow-y: auto;
      color: #66AACC;
      line-height: 1.6;
      white-space: pre-wrap;
      word-break: break-all;
    }
    .log::-webkit-scrollbar { width: 4px; }
    .log::-webkit-scrollbar-track { background: transparent; }
    .log::-webkit-scrollbar-thumb { background: #1E3A5F; border-radius: 4px; }
    .log .ok { color: #00FF88; }
    .log .err { color: #FF6666; }
    .log .info { color: #00D4FF; }
    .badge {
      display: inline-block;
      padding: 2px 8px;
      border-radius: 20px;
      font-size: 10px;
      font-weight: 700;
      letter-spacing: 0.5px;
    }
    .badge-green { background: #00FF8822; color: #00FF88; border: 1px solid #00FF8844; }
    .badge-yellow { background: #FFB80022; color: #FFB800; border: 1px solid #FFB80044; }
    .badge-blue { background: #00D4FF22; color: #00D4FF; border: 1px solid #00D4FF44; }
    .section-title {
      font-size: 10px;
      font-weight: 700;
      letter-spacing: 1.5px;
      color: #3A6A8F;
      text-transform: uppercase;
      margin: 0 0 8px;
    }
    .full-width { grid-column: 1 / -1; }
    /* ── OW Theme Integration ─────────────────────────────── */
    body { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text, #E0E0E0) !important; }
    .header { border-bottom-color: var(--ow-border, #1E3A5F) !important; }
    .card { background: var(--ow-bg-card, #0F1F3A) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .card:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.5)) !important; }
    .card h3 { color: var(--ow-accent, #00D4FF) !important; }
    .card-desc { color: var(--ow-text-dim, #5588AA) !important; }
    .metric { border-bottom-color: rgba(0,0,0,0.3) !important; }
    .metric-label { color: var(--ow-text-dim, #7799BB) !important; }
    .metric-value { color: var(--ow-warning, #FFB800) !important; }
    .btn { background: linear-gradient(135deg, var(--ow-accent, #00D4FF), #0090CC) !important; color: var(--ow-bg, #0A1628) !important; }
    .btn-secondary { background: transparent !important; color: var(--ow-accent, #00D4FF) !important; border-color: var(--ow-border, #1E5A7F) !important; }
    .btn-secondary:hover { background: var(--ow-bg-raise, #0F2A4A) !important; border-color: var(--ow-accent, #00D4FF) !important; }
    .btn-danger { background: transparent !important; color: var(--ow-danger, #FF6666) !important; border-color: rgba(255,68,68,0.3) !important; }
    .btn-danger:hover { background: rgba(255,68,68,0.07) !important; }
    .log { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border-subtle, #0F1E30) !important; color: var(--ow-text-dim, #66AACC) !important; }
    .badge-green { background: rgba(0,255,136,0.13) !important; color: var(--ow-success, #00FF88) !important; border-color: rgba(0,255,136,0.26) !important; }
    .badge-yellow { background: rgba(255,184,0,0.13) !important; color: var(--ow-warning, #FFB800) !important; border-color: rgba(255,184,0,0.26) !important; }
    .badge-blue { background: var(--ow-accent-dim, rgba(0,212,255,0.13)) !important; color: var(--ow-accent, #00D4FF) !important; border-color: var(--ow-border, rgba(0,212,255,0.26)) !important; }
    .section-title { color: var(--ow-text-muted, #3A6A8F) !important; }
    .header-text p { color: var(--ow-text-dim, #5588AA) !important; }
    .header-text h1 { color: var(--ow-accent, #00D4FF) !important; }
    .app-chip { background: var(--ow-bg-raise, #132035) !important; border-color: var(--ow-border, #1E3A5F) !important; color: var(--ow-text-dim, #88BBDD) !important; }
    .app-chip:hover { background: var(--ow-bg-card, #1E3A5F) !important; color: var(--ow-accent, #00D4FF) !important; }
  </style>
</head>
<body>
  <div class="header">
    <div class="logo">🌿</div>
    <div class="header-text">
      <h1>Omnisystem Ecosystem</h1>
      <p>Omnisystem Integrated Development Environment &mdash; v2.0.0</p>
    </div>
    <div class="header-actions">
      <div class="build-indicator" id="build-indicator">
        <div class="spinner"></div>
        <span>Building...</span>
      </div>
      <button class="btn btn-danger" id="stop-btn" style="display:none" onclick="sendCommand('stopBuild')">Stop</button>
      <button class="btn" onclick="deployAll()">&#9654; Deploy All</button>
      <button class="btn btn-secondary" onclick="refreshStatus()">&#8635; Refresh</button>
      <button class="btn btn-secondary" onclick="openThemePicker()" title="Switch Theme" aria-label="Switch Theme" style="padding:8px 12px;">&#127912;</button>
    </div>
  </div>

  <div class="grid">

    <!-- Omnisystem Launcher -->
    <div class="card">
      <h3><span class="status-dot active"></span>Omnisystem Launcher <span class="badge badge-green" style="margin-left:auto;font-size:10px;">Tauri</span></h3>
      <p class="card-desc">Cross-platform desktop application built with Tauri + Rust</p>
      <div class="metric">
        <span class="metric-label">Platform</span>
        <span class="metric-value" id="launcher-platform">Detecting...</span>
      </div>
      <div class="metric">
        <span class="metric-label">Build Status</span>
        <span class="metric-value" id="launcher-status"><span class="badge badge-green">Ready</span></span>
      </div>
      <div class="metric">
        <span class="metric-label">Version</span>
        <span class="metric-value">2.0.0</span>
      </div>
      <div class="metric">
        <span class="metric-label">Runtime</span>
        <span class="metric-value">Tauri v2 + WebKit</span>
      </div>
      <div class="actions">
        <button class="btn" onclick="sendCommand('omnisystemLaunch')">&#9654; Launch Dev</button>
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemBuild')">&#9109; Build Release</button>
      </div>
    </div>

    <!-- Buddy Android -->
    <div class="card">
      <h3><span class="status-dot pending" id="buddy-dot"></span>Omnisystem Buddy <span class="badge badge-yellow" style="margin-left:auto;">Android</span></h3>
      <p class="card-desc">Mobile AI assistant &mdash; 9 integrated sub-applications</p>
      <div class="metric">
        <span class="metric-label">ADB Status</span>
        <span class="metric-value" id="buddy-adb">Checking...</span>
      </div>
      <div class="metric">
        <span class="metric-label">Device</span>
        <span class="metric-value" id="buddy-device">&mdash;</span>
      </div>
      <p class="section-title" style="margin-top:14px;">Sub-Applications</p>
      <div class="app-grid">
        <div class="app-chip" onclick="sendCommand('openBuddyApp','app')">&#127968; Main</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','academy')">&#127979; Academy</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','ai-power')">&#129504; AI Power</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','compute')">&#9096; Compute</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','dev-suite')">&#128736; Dev Suite</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','models')">&#129302; Models</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','node')">&#128246; Node Ctrl</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','workspace')">&#128193; Workspace</div>
        <div class="app-chip" onclick="sendCommand('openBuddyApp','remote')">&#127758; Remote</div>
      </div>
      <div class="actions">
        <button class="btn" onclick="sendCommand('omnisystemBuddyBuild')">&#9109; Build APK</button>
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemBuddyConnect')">&#128246; Connect ADB</button>
      </div>
    </div>

    <!-- Browser Extension -->
    <div class="card">
      <h3><span class="status-dot active"></span>Browser Extension <span class="badge badge-blue" style="margin-left:auto;">Svelte</span></h3>
      <p class="card-desc">Chrome &amp; Firefox extension &mdash; Svelte-based UI</p>
      <div class="metric">
        <span class="metric-label">Chrome</span>
        <span class="metric-value"><span class="badge badge-green">Built</span></span>
      </div>
      <div class="metric">
        <span class="metric-label">Firefox</span>
        <span class="metric-value"><span class="badge badge-green">Built</span></span>
      </div>
      <div class="metric">
        <span class="metric-label">Bundle Size</span>
        <span class="metric-value">55 KB gzip</span>
      </div>
      <div class="metric">
        <span class="metric-label">Manifest</span>
        <span class="metric-value">v3</span>
      </div>
      <div class="actions">
        <button class="btn" onclick="sendCommand('omnisystemBrowserExtBuild')">&#9109; Build</button>
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemBrowserExtInstall')">&#128268; Install Dev</button>
      </div>
    </div>

    <!-- Control Panel -->
    <div class="card">
      <h3><span class="status-dot" id="cp-dot" style="background:#5588AA;"></span>Control Panel <span class="badge badge-blue" style="margin-left:auto;">Titan</span></h3>
      <p class="card-desc">Native Titan REST API — system monitoring &amp; service management</p>
      <div class="metric">
        <span class="metric-label">URL</span>
        <span class="metric-value">localhost:12345</span>
      </div>
      <div class="metric">
        <span class="metric-label">Status</span>
        <span class="metric-value" id="cp-status"><span class="badge badge-yellow">Checking...</span></span>
      </div>
      <div class="metric">
        <span class="metric-label">Endpoints</span>
        <span class="metric-value">30+ REST APIs</span>
      </div>
      <div class="metric">
        <span class="metric-label">Source</span>
        <span class="metric-value">core.ti · api_server.ti</span>
      </div>
      <div class="actions">
        <button class="btn" onclick="sendCommand('omnisystemControlPanel')">&#127758; Open on :12345</button>
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemControlPanelStart')">&#9654; Start Server</button>
      </div>
    </div>

    <!-- Notification System -->
    <div class="card">
      <h3><span class="status-dot active"></span>Notification System <span class="badge badge-green" style="margin-left:auto;">Titan</span></h3>
      <p class="card-desc">Native platform notifications — Windows WinRT · macOS · Linux D-Bus</p>
      <div class="metric">
        <span class="metric-label">Queue Limit</span>
        <span class="metric-value">1,000 (SQLite)</span>
      </div>
      <div class="metric">
        <span class="metric-label">Types</span>
        <span class="metric-value">info · success · warn · error · urgent</span>
      </div>
      <div class="metric">
        <span class="metric-label">Features</span>
        <span class="metric-value">DnD · history · actions · badge</span>
      </div>
      <div class="metric">
        <span class="metric-label">Source</span>
        <span class="metric-value">notification_daemon.ti</span>
      </div>
      <div class="actions">
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemNotifications')">&#128276; Open Dashboard</button>
      </div>
    </div>

    <!-- System Tray -->
    <div class="card">
      <h3><span class="status-dot active"></span>System Tray <span class="badge badge-green" style="margin-left:auto;">Titan</span></h3>
      <p class="card-desc">Cross-platform tray icon — Win32 NotifyIcon · NSStatusBar · D-Bus StatusNotifierItem</p>
      <div class="metric">
        <span class="metric-label">Menu Items</span>
        <span class="metric-value">11 items</span>
      </div>
      <div class="metric">
        <span class="metric-label">Events</span>
        <span class="metric-value">left · right · double-click</span>
      </div>
      <div class="metric">
        <span class="metric-label">Features</span>
        <span class="metric-value">badge · quick panel · status indicators</span>
      </div>
      <div class="metric">
        <span class="metric-label">Source</span>
        <span class="metric-value">system-tray/core.ti</span>
      </div>
      <div class="actions">
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemSystemTray')">&#128190; View Source</button>
      </div>
    </div>

    <!-- Shared UI Library -->
    <div class="card">
      <h3><span class="status-dot active"></span>@omnisystem/shared-ui <span class="badge badge-blue" style="margin-left:auto;">Library</span></h3>
      <p class="card-desc">Shared Svelte component library used across all Omnisystem apps</p>
      <div class="metric">
        <span class="metric-label">Components</span>
        <span class="metric-value">42 components</span>
      </div>
      <div class="metric">
        <span class="metric-label">Bundle</span>
        <span class="metric-value">31 KB compiled</span>
      </div>
      <div class="metric">
        <span class="metric-label">Theming</span>
        <span class="metric-value">CSS custom properties</span>
      </div>
      <div class="metric">
        <span class="metric-label">Tree-shakeable</span>
        <span class="metric-value"><span class="badge badge-green">Yes</span></span>
      </div>
      <div class="actions">
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemSharedUiBuild')">&#9109; Rebuild Library</button>
      </div>
    </div>

    <!-- Titan Infrastructure Source Files -->
    <div class="card full-width" id="titan-source-card">
      <h3>&#128196; Titan Infrastructure — Live Source Analysis</h3>
      <p class="card-desc" id="eco-status-label">Scanning Omnisystem codebase for Omnisystem infrastructure modules...</p>
      <table id="titan-source-table" style="width:100%;border-collapse:collapse;font-size:12px;margin-bottom:12px;">
        <thead>
          <tr style="color:#3A6A8F;font-size:10px;font-weight:700;letter-spacing:1px;text-transform:uppercase;border-bottom:1px solid #1E3A5F;">
            <th style="text-align:left;padding:6px 8px;">&#10003;</th>
            <th style="text-align:left;padding:6px 8px;">Module</th>
            <th style="text-align:right;padding:6px 8px;">LOC</th>
          </tr>
        </thead>
        <tbody id="titan-source-body">
          <tr><td colspan="3" style="color:#5588AA;padding:8px;text-align:center;">Loading...</td></tr>
        </tbody>
      </table>
      <div style="display:flex;gap:16px;align-items:center;font-size:12px;border-top:1px solid #1E3A5F;padding-top:10px;">
        <span style="color:#7799BB;">Total LOC:</span>
        <span id="titan-total-loc" style="color:#FFB800;font-weight:700;font-family:monospace;">—</span>
        <span style="color:#7799BB;margin-left:8px;">Ecosystem root:</span>
        <span id="eco-root-status" style="color:#FFB800;font-family:monospace;font-size:11px;">detecting...</span>
        <button class="btn btn-secondary" style="margin-left:auto;" onclick="loadEcosystemFiles()">&#8635; Rescan</button>
      </div>
    </div>

    <!-- Initialization Controls -->
    <div class="card full-width" style="border-color:#003344;">
      <h3>&#9881;&#65039; Omnisystem Ecosystem Initialization</h3>
      <p class="card-desc">5-phase startup: Omnisystem Integration → Infrastructure → Application Services → OS Integration → Health Check</p>
      <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:10px;margin-bottom:14px;">
        <div style="background:#0A1A2A;border:1px solid #1E3A5F;border-radius:8px;padding:10px;">
          <div style="font-size:10px;color:#3A6A8F;font-weight:700;text-transform:uppercase;margin-bottom:4px;">Phase 1</div>
          <div style="font-size:13px;color:#88BBDD;">Omnisystem Integration</div>
          <div style="font-size:11px;color:#5588AA;">50+ capabilities registered</div>
        </div>
        <div style="background:#0A1A2A;border:1px solid #1E3A5F;border-radius:8px;padding:10px;">
          <div style="font-size:10px;color:#3A6A8F;font-weight:700;text-transform:uppercase;margin-bottom:4px;">Phase 2</div>
          <div style="font-size:13px;color:#88BBDD;">System Infrastructure</div>
          <div style="font-size:11px;color:#5588AA;">Control Panel · Notifications · Tray</div>
        </div>
        <div style="background:#0A1A2A;border:1px solid #1E3A5F;border-radius:8px;padding:10px;">
          <div style="font-size:10px;color:#3A6A8F;font-weight:700;text-transform:uppercase;margin-bottom:4px;">Phase 3</div>
          <div style="font-size:13px;color:#88BBDD;">Application Services</div>
          <div style="font-size:11px;color:#5588AA;">Workspace · Buddy · Browser Ext</div>
        </div>
        <div style="background:#0A1A2A;border:1px solid #1E3A5F;border-radius:8px;padding:10px;">
          <div style="font-size:10px;color:#3A6A8F;font-weight:700;text-transform:uppercase;margin-bottom:4px;">Phase 4</div>
          <div style="font-size:13px;color:#88BBDD;">OS-level Integration</div>
          <div style="font-size:11px;color:#5588AA;">File Assoc · Theme · Installer</div>
        </div>
        <div style="background:#0A1A2A;border:1px solid #1E3A5F;border-radius:8px;padding:10px;">
          <div style="font-size:10px;color:#3A6A8F;font-weight:700;text-transform:uppercase;margin-bottom:4px;">Phase 5</div>
          <div style="font-size:13px;color:#88BBDD;">Verification &amp; Health</div>
          <div style="font-size:11px;color:#5588AA;">Health check daemon</div>
        </div>
      </div>
      <div class="actions">
        <button class="btn" onclick="sendCommand('omnisystemInit')">&#9654; Initialize Ecosystem</button>
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemDiagnostics')">&#128270; Diagnostics</button>
        <button class="btn btn-secondary" onclick="sendCommand('omnisystemRepair')">&#128295; Repair</button>
      </div>
    </div>

    <!-- Build Log -->
    <div class="card full-width">
      <h3>&#128203; Build &amp; Deploy Log</h3>
      <div class="log" id="build-log">Ready. Click an action above to start...</div>
      <div class="actions" style="margin-top:12px;">
        <button class="btn" onclick="sendCommand('omnisystemDeployAll')">&#128640; Deploy Release</button>
        <button class="btn btn-secondary" onclick="clearLog()">&#215; Clear Log</button>
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

    function sendCommand(cmd, arg) {
      vscode.postMessage({ command: cmd, arg: arg });
    }

    function deployAll() { sendCommand('deployAll'); }
    function refreshStatus() { sendCommand('refresh'); }
    function loadEcosystemFiles() { sendCommand('getEcosystemFiles'); }

    function clearLog() {
      document.getElementById('build-log').textContent = 'Log cleared.';
    }

    function appendLog(text) {
      const log = document.getElementById('build-log');
      const line = document.createElement('span');
      const t = text.trim();
      if (t.startsWith('[') && t.includes('error')) {
        line.className = 'err';
      } else if (t.includes('warning') || t.includes('warn')) {
        line.style.color = '#FFB800';
      } else if (t.match(/success|done|complete|built/i)) {
        line.className = 'ok';
      }
      line.textContent = text;
      log.appendChild(document.createTextNode('\n'));
      log.appendChild(line);
      log.scrollTop = log.scrollHeight;
    }

    window.addEventListener('message', event => {
      const msg = event.data;

      if (msg.type === 'log') { appendLog(msg.text); }

      if (msg.type === 'building') {
        const ind = document.getElementById('build-indicator');
        const stopBtn = document.getElementById('stop-btn');
        if (msg.value) {
          ind.classList.add('active');
          stopBtn.style.display = 'inline-block';
        } else {
          ind.classList.remove('active');
          stopBtn.style.display = 'none';
        }
      }

      if (msg.type === 'status') {
        if (msg.launcher) {
          document.getElementById('launcher-platform').textContent = msg.launcher.platform;
        }
        if (msg.buddy) {
          const adbEl = document.getElementById('buddy-adb');
          const devEl = document.getElementById('buddy-device');
          const dot = document.getElementById('buddy-dot');
          adbEl.textContent = msg.buddy.adb;
          devEl.textContent = msg.buddy.device;
          if (msg.buddy.adb === 'Connected') {
            dot.className = 'status-dot active';
          } else if (msg.buddy.adb === 'No devices' || msg.buddy.adb === 'ADB not found') {
            dot.className = 'status-dot inactive';
          }
        }
        if (msg.controlPanel) {
          const cpDot = document.getElementById('cp-dot');
          const cpStatus = document.getElementById('cp-status');
          if (msg.controlPanel.online) {
            cpDot.className = 'status-dot active';
            cpStatus.innerHTML = '<span class="badge badge-green">Online :12345</span>';
          } else {
            cpDot.style.background = '#FF4444';
            cpStatus.innerHTML = '<span class="badge badge-yellow">Offline</span>';
          }
        }
      }

      if (msg.type === 'ecosystemFiles') {
        const tbody = document.getElementById('titan-source-body');
        const totalEl = document.getElementById('titan-total-loc');
        const rootEl = document.getElementById('eco-root-status');
        const labelEl = document.getElementById('eco-status-label');

        rootEl.textContent = msg.ecoFound ? '$(check) omnisystem-ecosystem/ found' : '$(warning) not found';
        labelEl.textContent = msg.ecoFound
          ? 'Live Titan source files — click to open in editor'
          : 'Ecosystem not found at expected path. Set omnisystem.desktopPath in settings.';

        tbody.innerHTML = msg.files.map(f => {
          const tick = f.found
            ? '<span style="color:#00FF88">&#10003;</span>'
            : '<span style="color:#FF6666">&#10007;</span>';
          const locText = f.found ? f.loc.toLocaleString() : '—';
          const nameStyle = f.found ? 'color:#88BBDD' : 'color:#5588AA';
          return '<tr style="border-bottom:1px solid #0F1E30;">'
            + '<td style="padding:5px 8px;width:20px;">' + tick + '</td>'
            + '<td style="padding:5px 8px;font-family:monospace;font-size:11px;' + nameStyle + '">' + f.name + '</td>'
            + '<td style="padding:5px 8px;text-align:right;color:#FFB800;font-family:monospace;">' + locText + '</td>'
            + '</tr>';
        }).join('');

        totalEl.textContent = msg.totalLoc.toLocaleString() + ' LOC';
      }
    });

    // Request initial data on load
    sendCommand('getStatus');
    loadEcosystemFiles();
  </script>
</body>
</html>`;
    }

    public dispose(): void {
        OmnisystemDashboardPanel.currentPanel = undefined;
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
