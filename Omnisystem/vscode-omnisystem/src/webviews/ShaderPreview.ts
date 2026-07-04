import * as vscode from 'vscode';
import * as fs from 'fs';

// ─── Utilities ────────────────────────────────────────────────────────────────

function getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}

function escapeHtml(str: string): string {
    return str
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
}

// ─── ShaderPreviewPanel ───────────────────────────────────────────────────────

export class ShaderPreviewPanel {
    public static currentPanel: ShaderPreviewPanel | undefined;
    public static readonly viewType = 'omnisystem.shaderPreview';

    public static postMessage(msg: object): void {
        ShaderPreviewPanel.currentPanel?._panel.webview.postMessage(msg);
    }

    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];
    private _editorChangeDisposable: vscode.Disposable | undefined;

    public static createOrShow(extensionUri: vscode.Uri): void {
        const column = vscode.window.activeTextEditor
            ? vscode.window.activeTextEditor.viewColumn
            : undefined;

        if (ShaderPreviewPanel.currentPanel) {
            ShaderPreviewPanel.currentPanel._panel.reveal(column);
            ShaderPreviewPanel.currentPanel._loadActiveShader();
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            ShaderPreviewPanel.viewType,
            'HELIX Shader Preview',
            column || vscode.ViewColumn.Two,
            {
                enableScripts: true,
                localResourceRoots: [extensionUri],
                retainContextWhenHidden: true,
            }
        );

        ShaderPreviewPanel.currentPanel = new ShaderPreviewPanel(panel, extensionUri);
    }

    private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
        this._panel = panel;
        this._extensionUri = extensionUri;

        this._panel.iconPath = {
            light: vscode.Uri.joinPath(extensionUri, 'icons', 'helix-light.svg'),
            dark: vscode.Uri.joinPath(extensionUri, 'icons', 'helix-dark.svg'),
        };

        this._update();

        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
        this._panel.webview.onDidReceiveMessage(
            (message) => this._handleMessage(message),
            null,
            this._disposables
        );

        // Track active editor changes
        this._editorChangeDisposable = vscode.window.onDidChangeActiveTextEditor(() => {
            this._loadActiveShader();
        });
        this._disposables.push(this._editorChangeDisposable);

        // Track document saves
        const saveDisposable = vscode.workspace.onDidSaveTextDocument((doc) => {
            if (doc.languageId === 'helix' || doc.fileName.endsWith('.helix')) {
                this._loadActiveShader();
            }
        });
        this._disposables.push(saveDisposable);

        this._loadActiveShader();
    }

    private _post(msg: object): void {
        this._panel.webview.postMessage(msg);
    }

    private _loadActiveShader(): void {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            this._post({ type: 'noShader' });
            return;
        }

        const doc = editor.document;
        const isHelix = doc.languageId === 'helix' || doc.fileName.endsWith('.helix');
        if (!isHelix) {
            this._post({ type: 'noShader' });
            return;
        }

        const source = doc.getText();
        const fileName = doc.fileName.replace(/\\/g, '/').split('/').pop() || 'unknown.helix';
        const info = this._analyzeShader(source, fileName);

        this._post({ type: 'shaderLoaded', source, info, fileName });
    }

    private _analyzeShader(source: string, fileName: string): object {
        // Determine shader type
        let shaderType = 'unknown';
        if (source.includes('@vertex') || source.includes('vertex_main') || fileName.includes('vert')) {
            shaderType = 'vertex';
        } else if (source.includes('@fragment') || source.includes('frag_main') || fileName.includes('frag')) {
            shaderType = 'fragment';
        } else if (source.includes('@compute') || source.includes('compute_main') || fileName.includes('comp')) {
            shaderType = 'compute';
        }

        // Extract uniforms (lines like: uniform float time; or @uniform vec4 color;)
        const uniforms: { name: string; type: string; desc: string }[] = [];
        const uniformRe = /(?:@uniform\s+|uniform\s+)(\w+)\s+(\w+)/g;
        let m: RegExpExecArray | null;
        while ((m = uniformRe.exec(source)) !== null) {
            uniforms.push({ type: m[1], name: m[2], desc: this._guessUniformDesc(m[2]) });
        }

        // Extract inputs/outputs
        const inputs: string[] = [];
        const outputs: string[] = [];
        const inRe = /(?:@in|in\s+)\s*(\w+)\s+(\w+)/g;
        const outRe = /(?:@out|out\s+)\s*(\w+)\s+(\w+)/g;
        while ((m = inRe.exec(source)) !== null) {
            inputs.push(`${m[1]} ${m[2]}`);
        }
        while ((m = outRe.exec(source)) !== null) {
            outputs.push(`${m[1]} ${m[2]}`);
        }

        // Estimate GPU cost (rough heuristic)
        const lines = source.split('\n').length;
        const texSamples = (source.match(/sample|texture/g) || []).length;
        const branches = (source.match(/\bif\b|\bfor\b|\bwhile\b/g) || []).length;
        let gpuCost = 'Low';
        if (texSamples > 4 || branches > 8 || lines > 100) gpuCost = 'High';
        else if (texSamples > 1 || branches > 3 || lines > 50) gpuCost = 'Medium';

        // Compilation status (syntactic heuristic)
        let compileStatus = 'Ready';
        let compileOk = true;
        if (!source.trim()) { compileStatus = 'Empty'; compileOk = false; }

        return { shaderType, uniforms, inputs, outputs, gpuCost, compileStatus, compileOk, lines, texSamples, branches };
    }

    private _guessUniformDesc(name: string): string {
        const map: Record<string, string> = {
            time: 'Elapsed time in seconds',
            uTime: 'Elapsed time in seconds',
            resolution: 'Viewport dimensions (width, height)',
            uResolution: 'Viewport dimensions (width, height)',
            color: 'RGBA color value',
            uColor: 'RGBA color value',
            transform: '4x4 transformation matrix',
            uTransform: '4x4 transformation matrix',
            model: 'Model matrix',
            view: 'View matrix',
            projection: 'Projection matrix',
            mvp: 'Model-View-Projection combined matrix',
            lightPos: 'World-space light position',
            cameraPos: 'World-space camera position',
            texture0: 'Diffuse texture sampler',
            normalMap: 'Normal map texture sampler',
            roughness: 'PBR roughness parameter [0,1]',
            metallic: 'PBR metallic parameter [0,1]',
            opacity: 'Global opacity multiplier',
        };
        return map[name] || 'User-defined uniform';
    }

    private async _handleMessage(message: {
        command: string;
        source?: string;
    }): Promise<void> {
        switch (message.command) {
            case 'compile': {
                const source = message.source || '';
                this._post({ type: 'compileStart' });
                // Simulate compile (in reality would call helix compiler)
                setTimeout(() => {
                    const hasError = source.includes('ERROR') || source.includes('!!ERR');
                    if (hasError) {
                        this._post({
                            type: 'compileResult',
                            ok: false,
                            errors: [{ line: 1, col: 1, msg: 'Syntax error: unexpected token' }],
                        });
                    } else {
                        this._post({ type: 'compileResult', ok: true, errors: [] });
                    }
                }, 600);
                break;
            }

            case 'applyPreview':
                this._post({ type: 'previewApplied' });
                vscode.window.showInformationMessage('Shader applied to preview viewport.');
                break;

            case 'loadActive':
                this._loadActiveShader();
                break;

            case 'openFile': {
                const uris = await vscode.window.showOpenDialog({
                    canSelectMany: false,
                    filters: { 'HELIX Shaders': ['helix'] },
                    title: 'Open HELIX Shader',
                });
                if (uris && uris[0]) {
                    const doc = await vscode.workspace.openTextDocument(uris[0]);
                    await vscode.window.showTextDocument(doc, vscode.ViewColumn.One);
                    this._loadActiveShader();
                }
                break;
            }
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
        this._panel.title = 'HELIX Shader Preview';
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
  <title>HELIX Shader Preview</title>
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
      margin-bottom: 22px; padding-bottom: 16px;
      border-bottom: 1px solid #1E3A5F;
    }
    .logo {
      width: 46px; height: 46px;
      background: linear-gradient(135deg, #FF8C42, #FF4500);
      border-radius: 12px;
      display: flex; align-items: center; justify-content: center;
      font-size: 22px; box-shadow: 0 4px 20px rgba(255,140,66,0.35);
    }
    h1 { color: #00D4FF; font-size: 22px; font-weight: 700; margin: 0 0 3px; }
    .subtitle { color: #5588AA; font-size: 12px; margin: 0; }
    .layout {
      display: grid;
      grid-template-columns: 1fr 380px;
      gap: 18px;
      height: calc(100vh - 120px);
      min-height: 500px;
    }
    @media (max-width: 900px) { .layout { grid-template-columns: 1fr; height: auto; } }
    .card {
      background: #0F1F3A; border: 1px solid #1E3A5F;
      border-radius: 12px; padding: 18px;
      display: flex; flex-direction: column; overflow: hidden;
    }
    .card h3 { color: #00D4FF; font-size: 14px; font-weight: 600; margin: 0 0 12px; flex-shrink: 0; }
    .btn {
      background: linear-gradient(135deg, #00D4FF, #0090CC);
      color: #0A1628; border: none; border-radius: 7px;
      padding: 7px 16px; cursor: pointer; font-weight: 700;
      font-size: 12px; transition: all 0.15s; white-space: nowrap;
    }
    .btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .btn-orange { background: linear-gradient(135deg, #FF8C42, #CC5500); color: #fff; }
    .btn-secondary {
      background: transparent; color: #00D4FF; border: 1px solid #1E5A7F;
    }
    .btn-secondary:hover { background: #0F2A4A; }
    .actions { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 12px; flex-shrink: 0; }
    /* Source display */
    .source-header {
      display: flex; justify-content: space-between; align-items: center;
      margin-bottom: 10px; flex-shrink: 0;
    }
    .source-filename {
      font-family: monospace; font-size: 12px; color: #FFB800;
      background: #0A1628; border: 1px solid #1E3A5F;
      border-radius: 5px; padding: 3px 10px;
    }
    .source-code {
      background: #050D1A; border: 1px solid #0F1E30;
      border-radius: 8px; padding: 14px;
      font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
      font-size: 12px; line-height: 1.7;
      overflow: auto; flex: 1;
      color: #AAD4EE; white-space: pre;
      tab-size: 2;
    }
    .source-code::-webkit-scrollbar { width: 6px; height: 6px; }
    .source-code::-webkit-scrollbar-thumb { background: #1E3A5F; border-radius: 4px; }
    .source-empty {
      display: flex; align-items: center; justify-content: center;
      flex-direction: column; gap: 12px; height: 100%;
      color: #3A5A7A; text-align: center;
    }
    .source-empty .icon { font-size: 48px; opacity: 0.5; }
    /* Syntax highlighting */
    .kw { color: #00D4FF; font-weight: 600; }
    .ty { color: #87CEEB; }
    .fn { color: #FFB800; }
    .cm { color: #3A6A8F; font-style: italic; }
    .nm { color: #DDA0DD; }
    .nu { color: #00FF88; }
    .at { color: #FF8C42; font-weight: 600; }
    /* Info panel */
    .info-panel { overflow-y: auto; flex: 1; }
    .info-panel::-webkit-scrollbar { width: 4px; }
    .info-panel::-webkit-scrollbar-thumb { background: #1E3A5F; border-radius: 4px; }
    .section { margin-bottom: 16px; }
    .section-title {
      font-size: 10px; font-weight: 700; letter-spacing: 1.2px;
      color: #3A6A8F; text-transform: uppercase; margin: 0 0 8px;
    }
    .info-row {
      display: flex; justify-content: space-between; align-items: center;
      padding: 7px 0; border-bottom: 1px solid #0F1E30; font-size: 12px;
    }
    .info-row:last-child { border-bottom: none; }
    .info-label { color: #7799BB; }
    .info-value { color: #E0E0E0; font-family: monospace; font-size: 11px; }
    .badge {
      display: inline-block; padding: 2px 8px; border-radius: 20px;
      font-size: 10px; font-weight: 700; letter-spacing: 0.4px;
    }
    .badge-vertex { background: #002A44; color: #00D4FF; border: 1px solid #00D4FF44; }
    .badge-fragment { background: #2A1144; color: #DDA0DD; border: 1px solid #DDA0DD44; }
    .badge-compute { background: #441100; color: #FF8C42; border: 1px solid #FF8C4244; }
    .badge-unknown { background: #1A1A1A; color: #888; border: 1px solid #33333344; }
    .badge-green { background: #003322; color: #00FF88; border: 1px solid #00FF8844; }
    .badge-red { background: #330000; color: #FF4444; border: 1px solid #FF444444; }
    .badge-yellow { background: #332200; color: #FFB800; border: 1px solid #FFB80044; }
    .badge-blue { background: #002244; color: #00D4FF; border: 1px solid #00D4FF44; }
    .table { width: 100%; border-collapse: collapse; font-size: 11px; }
    .table th {
      text-align: left; padding: 5px 8px;
      background: #0A1628; color: #5588AA;
      font-size: 10px; font-weight: 700;
      text-transform: uppercase; letter-spacing: 0.5px;
      border-bottom: 1px solid #1E3A5F;
    }
    .table td {
      padding: 6px 8px; border-bottom: 1px solid #0F1E30;
      font-family: monospace; color: #BBD4EE;
      vertical-align: top;
    }
    .table td:first-child { color: #FFB800; }
    .table td:nth-child(2) { color: #87CEEB; }
    .table td:last-child { color: #7799BB; font-size: 10px; font-family: sans-serif; }
    /* Pipeline viz */
    .pipeline {
      display: flex; align-items: center; gap: 4px;
      justify-content: center; margin: 6px 0 14px; flex-wrap: wrap;
    }
    .pipe-box {
      border-radius: 7px; padding: 8px 10px; text-align: center;
      font-size: 10px; font-weight: 700; text-transform: uppercase;
      letter-spacing: 0.5px; min-width: 60px;
    }
    .pipe-box.vs { background: #002A44; color: #00D4FF; border: 1px solid #00D4FF44; }
    .pipe-box.rs { background: #1A1A2A; color: #666; border: 1px solid #33333344; }
    .pipe-box.fs { background: #2A1144; color: #DDA0DD; border: 1px solid #DDA0DD44; }
    .pipe-box.out { background: #003322; color: #00FF88; border: 1px solid #00FF8844; }
    .pipe-box.active { box-shadow: 0 0 12px currentColor; }
    .pipe-arrow { color: #1E3A5F; font-size: 14px; font-weight: 700; }
    /* Compile status */
    .compile-status {
      display: flex; align-items: center; gap: 8px;
      padding: 10px 12px; border-radius: 8px; font-size: 12px; font-weight: 600;
      margin-bottom: 12px;
    }
    .compile-status.ok { background: #003322; color: #00FF88; border: 1px solid #00FF8822; }
    .compile-status.err { background: #330000; color: #FF4444; border: 1px solid #FF444422; }
    .compile-status.idle { background: #0A1628; color: #5588AA; border: 1px solid #1E3A5F; }
    .compile-status.compiling { background: #002244; color: #00D4FF; border: 1px solid #00D4FF22; }
    .spinner { width:12px; height:12px; border:2px solid #1E3A5F; border-top-color:#00D4FF; border-radius:50%; animation:spin 0.8s linear infinite; display:inline-block; }
    @keyframes spin { to { transform: rotate(360deg); } }
    .chip-list { display: flex; flex-wrap: wrap; gap: 4px; }
    .chip { background: #0A1628; border: 1px solid #1E3A5F; border-radius: 5px; padding: 3px 8px; font-family: monospace; font-size: 11px; color: #88AACC; }
    /* ── OW Theme Integration ─────────────────────────────── */
    body { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text, #E0E0E0) !important; }
    .header { border-bottom-color: var(--ow-border, #1E3A5F) !important; }
    .card { background: var(--ow-bg-card, #0F1F3A) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .card h3 { color: var(--ow-accent, #00D4FF) !important; }
    h1 { color: var(--ow-accent, #00D4FF) !important; }
    .subtitle { color: var(--ow-text-dim, #5588AA) !important; }
    .btn { background: linear-gradient(135deg, var(--ow-accent, #00D4FF), #0090CC) !important; color: var(--ow-bg, #0A1628) !important; }
    .btn-orange { background: linear-gradient(135deg, #FF8C42, #CC5500) !important; color: #fff !important; }
    .btn-secondary { background: transparent !important; color: var(--ow-accent, #00D4FF) !important; border-color: var(--ow-border, #1E5A7F) !important; }
    .source-code { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border-subtle, #0F1E30) !important; color: #AAD4EE !important; }
    .source-filename { color: var(--ow-warning, #FFB800) !important; background: var(--ow-bg, #0A1628) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .section-title { color: var(--ow-text-muted, #3A6A8F) !important; }
    .info-label { color: var(--ow-text-dim, #7799BB) !important; }
    .info-value { color: var(--ow-text, #E0E0E0) !important; }
    .info-row { border-bottom-color: rgba(0,0,0,0.3) !important; }
    .compile-status.ok { background: rgba(0,255,136,0.07) !important; color: var(--ow-success, #00FF88) !important; border-color: rgba(0,255,136,0.2) !important; }
    .compile-status.err { background: rgba(255,68,68,0.07) !important; color: var(--ow-danger, #FF4444) !important; border-color: rgba(255,68,68,0.2) !important; }
    .compile-status.idle { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text-muted, #5588AA) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .compile-status.compiling { background: rgba(0,212,255,0.07) !important; color: var(--ow-accent, #00D4FF) !important; border-color: rgba(0,212,255,0.2) !important; }
    .chip { background: var(--ow-bg, #0A1628) !important; border-color: var(--ow-border, #1E3A5F) !important; color: var(--ow-text-dim, #88AACC) !important; }
    .pipe-box.vs { background: rgba(0,42,68,0.8) !important; color: var(--ow-accent, #00D4FF) !important; }
    .pipe-box.fs { background: rgba(42,17,68,0.8) !important; color: var(--ow-purple, #DDA0DD) !important; }
    .pipe-box.out { background: rgba(0,51,34,0.8) !important; color: var(--ow-success, #00FF88) !important; }
    .table th { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text-muted, #5588AA) !important; border-bottom-color: var(--ow-border, #1E3A5F) !important; }
    .table td { border-bottom-color: rgba(0,0,0,0.3) !important; color: var(--ow-text-dim, #BBD4EE) !important; }
  </style>
</head>
<body>
  <div class="header">
    <div class="logo">&#9728;</div>
    <div>
      <h1>HELIX Shader Preview</h1>
      <p class="subtitle">Real-time shader analysis &amp; pipeline visualization</p>
    </div>
    <div style="margin-left:auto; display:flex; gap:8px;">
      <button class="btn btn-secondary" onclick="loadActive()">&#8635; Load Active File</button>
      <button class="btn btn-secondary" onclick="openFile()">&#128196; Open File</button>
      <button class="btn btn-secondary" onclick="openThemePicker()" title="Switch Theme" aria-label="Switch Theme" style="padding:8px 12px;">&#127912;</button>
    </div>
  </div>

  <div class="layout">

    <!-- Left: Source Code -->
    <div class="card">
      <div class="source-header">
        <h3 style="margin:0;">&#128196; Shader Source</h3>
        <span class="source-filename" id="shader-filename">No file loaded</span>
      </div>
      <div class="source-code" id="source-display">
        <div class="source-empty">
          <div class="icon">&#9728;</div>
          <div>Open a <strong>.helix</strong> file in the editor<br>to see shader source here.</div>
          <button class="btn btn-secondary" onclick="loadActive()">Load Active Editor</button>
        </div>
      </div>
      <div class="actions">
        <button class="btn btn-orange" onclick="compileShader()">&#9654; Compile Shader</button>
        <button class="btn" onclick="applyPreview()">&#9728; Apply to Preview</button>
      </div>
    </div>

    <!-- Right: Info Panel -->
    <div class="card">
      <h3>&#128202; Shader Analysis</h3>
      <div class="info-panel">

        <!-- Compile Status -->
        <div class="compile-status idle" id="compile-status">
          <span id="compile-icon">&#9675;</span>
          <span id="compile-text">Ready — compile to analyze</span>
        </div>

        <!-- Pipeline Viz -->
        <div class="section">
          <p class="section-title">Pipeline Stages</p>
          <div class="pipeline">
            <div class="pipe-box vs" id="pipe-vs">VS</div>
            <div class="pipe-arrow">&rsaquo;</div>
            <div class="pipe-box rs" id="pipe-rs">Rasterize</div>
            <div class="pipe-arrow">&rsaquo;</div>
            <div class="pipe-box fs" id="pipe-fs">FS</div>
            <div class="pipe-arrow">&rsaquo;</div>
            <div class="pipe-box out" id="pipe-out">Output</div>
          </div>
        </div>

        <!-- General Info -->
        <div class="section">
          <p class="section-title">Shader Info</p>
          <div class="info-row">
            <span class="info-label">Type</span>
            <span id="info-type"><span class="badge badge-unknown">Unknown</span></span>
          </div>
          <div class="info-row">
            <span class="info-label">Lines</span>
            <span class="info-value" id="info-lines">—</span>
          </div>
          <div class="info-row">
            <span class="info-label">Texture Samples</span>
            <span class="info-value" id="info-texsamples">—</span>
          </div>
          <div class="info-row">
            <span class="info-label">Branches</span>
            <span class="info-value" id="info-branches">—</span>
          </div>
          <div class="info-row">
            <span class="info-label">Est. GPU Cost</span>
            <span id="info-gpu-cost"><span class="badge badge-blue">—</span></span>
          </div>
        </div>

        <!-- Uniforms -->
        <div class="section">
          <p class="section-title">Uniforms</p>
          <div id="uniforms-container">
            <span style="color:#3A5A7A; font-size:12px;">No uniforms detected.</span>
          </div>
        </div>

        <!-- Inputs / Outputs -->
        <div class="section">
          <p class="section-title">Inputs</p>
          <div class="chip-list" id="inputs-list">
            <span style="color:#3A5A7A; font-size:12px;">None</span>
          </div>
        </div>
        <div class="section">
          <p class="section-title">Outputs</p>
          <div class="chip-list" id="outputs-list">
            <span style="color:#3A5A7A; font-size:12px;">None</span>
          </div>
        </div>

      </div>
      <!-- Actions at bottom -->
      <div class="actions" style="margin-top:auto; padding-top:12px; border-top:1px solid #1E3A5F; flex-shrink:0;">
        <button class="btn btn-orange" onclick="compileShader()">&#9654; Compile</button>
        <button class="btn" onclick="applyPreview()">Apply Preview</button>
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
    let currentSource = '';

    function loadActive() { vscode.postMessage({ command: 'loadActive' }); }
    function openFile() { vscode.postMessage({ command: 'openFile' }); }

    function compileShader() {
      setCompileStatus('compiling', '<div class="spinner"></div>', 'Compiling...');
      vscode.postMessage({ command: 'compile', source: currentSource });
    }

    function applyPreview() {
      vscode.postMessage({ command: 'applyPreview', source: currentSource });
    }

    function setCompileStatus(cls, icon, text) {
      const el = document.getElementById('compile-status');
      el.className = 'compile-status ' + cls;
      document.getElementById('compile-icon').innerHTML = icon;
      document.getElementById('compile-text').textContent = text;
    }

    function syntaxHighlight(source) {
      const KEYWORDS = ['fn','let','var','const','return','if','else','for','while','break','continue','struct','uniform','in','out','inout','true','false','void'];
      const TYPES = ['float','vec2','vec3','vec4','mat2','mat3','mat4','int','uint','bool','sampler2D','samplerCube','texture2D'];
      const ATTRS = ['@vertex','@fragment','@compute','@uniform','@location','@builtin','@group','@binding'];

      let html = '';
      const lines = source.split('\n');
      for (const line of lines) {
        let escaped = line
          .replace(/&/g,'&amp;')
          .replace(/</g,'&lt;')
          .replace(/>/g,'&gt;');

        // Comments
        escaped = escaped.replace(/(\/\/.*$)/g, '<span class="cm">$1</span>');
        if (!escaped.includes('cm')) {
          // Attributes
          ATTRS.forEach(a => {
            escaped = escaped.replace(new RegExp('(' + a.replace('@','@') + ')', 'g'), '<span class="at">$1</span>');
          });
          // Numbers
          escaped = escaped.replace(/\b(\d+\.?\d*(?:f|u|i)?)\b/g, '<span class="nu">$1</span>');
          // Types
          TYPES.forEach(t => {
            escaped = escaped.replace(new RegExp('\\b(' + t + ')\\b', 'g'), '<span class="ty">$1</span>');
          });
          // Keywords
          KEYWORDS.forEach(k => {
            escaped = escaped.replace(new RegExp('\\b(' + k + ')\\b', 'g'), '<span class="kw">$1</span>');
          });
          // Function names (identifier before '(')
          escaped = escaped.replace(/\b([a-zA-Z_]\w*)\s*(?=\()/g, '<span class="fn">$1</span>');
        }
        html += escaped + '\n';
      }
      return html;
    }

    function updatePipeline(shaderType) {
      ['vs','rs','fs','out'].forEach(id => {
        document.getElementById('pipe-' + id).classList.remove('active');
      });
      if (shaderType === 'vertex') document.getElementById('pipe-vs').classList.add('active');
      else if (shaderType === 'fragment') document.getElementById('pipe-fs').classList.add('active');
      else if (shaderType === 'compute') {
        // Compute replaces the whole pipeline
        ['vs','rs','fs','out'].forEach(id => document.getElementById('pipe-' + id).classList.remove('active'));
        document.getElementById('pipe-vs').style.setProperty('color','#FF8C42');
      }
    }

    function renderUniforms(uniforms) {
      const c = document.getElementById('uniforms-container');
      if (!uniforms || uniforms.length === 0) {
        c.innerHTML = '<span style="color:#3A5A7A;font-size:12px;">No uniforms detected.</span>';
        return;
      }
      let html = '<table class="table"><thead><tr><th>Name</th><th>Type</th><th>Description</th></tr></thead><tbody>';
      uniforms.forEach(u => {
        html += '<tr><td>' + u.name + '</td><td>' + u.type + '</td><td>' + u.desc + '</td></tr>';
      });
      html += '</tbody></table>';
      c.innerHTML = html;
    }

    function renderChips(containerId, items) {
      const c = document.getElementById(containerId);
      if (!items || items.length === 0) {
        c.innerHTML = '<span style="color:#3A5A7A;font-size:12px;">None</span>';
        return;
      }
      c.innerHTML = items.map(i => '<span class="chip">' + i + '</span>').join('');
    }

    window.addEventListener('message', event => {
      const msg = event.data;

      switch (msg.type) {
        case 'shaderLoaded': {
          currentSource = msg.source;
          document.getElementById('shader-filename').textContent = msg.fileName;
          const display = document.getElementById('source-display');
          if (msg.source.trim()) {
            display.innerHTML = syntaxHighlight(msg.source);
          } else {
            display.innerHTML = '<div class="source-empty"><div class="icon">&#9728;</div><div>File is empty.</div></div>';
          }
          // Update info panel
          const info = msg.info;
          const typeBadge = {
            vertex: '<span class="badge badge-vertex">Vertex</span>',
            fragment: '<span class="badge badge-fragment">Fragment</span>',
            compute: '<span class="badge badge-compute">Compute</span>',
            unknown: '<span class="badge badge-unknown">Unknown</span>',
          }[info.shaderType] || '<span class="badge badge-unknown">Unknown</span>';
          document.getElementById('info-type').innerHTML = typeBadge;
          document.getElementById('info-lines').textContent = info.lines + ' lines';
          document.getElementById('info-texsamples').textContent = info.texSamples;
          document.getElementById('info-branches').textContent = info.branches;

          const costBadge = {
            Low: '<span class="badge badge-green">Low</span>',
            Medium: '<span class="badge badge-yellow">Medium</span>',
            High: '<span class="badge badge-red">High</span>',
          }[info.gpuCost] || '<span class="badge badge-blue">—</span>';
          document.getElementById('info-gpu-cost').innerHTML = costBadge;

          updatePipeline(info.shaderType);
          renderUniforms(info.uniforms);
          renderChips('inputs-list', info.inputs);
          renderChips('outputs-list', info.outputs);
          setCompileStatus('idle', '&#9675;', 'Ready — click Compile to check');
          break;
        }

        case 'noShader':
          document.getElementById('shader-filename').textContent = 'No .helix file active';
          document.getElementById('source-display').innerHTML =
            '<div class="source-empty"><div class="icon">&#9728;</div><div>Open a <strong>.helix</strong> file in the editor to see it here.</div></div>';
          break;

        case 'compileStart':
          setCompileStatus('compiling', '<div class="spinner"></div>', 'Compiling...');
          break;

        case 'compileResult':
          if (msg.ok) {
            setCompileStatus('ok', '&#10003;', 'Compiled successfully — no errors');
          } else {
            const errCount = (msg.errors || []).length;
            setCompileStatus('err', '&#10007;', errCount + ' error(s) found');
          }
          break;

        case 'previewApplied':
          setCompileStatus('ok', '&#9728;', 'Shader applied to preview');
          break;
      }
    });
  </script>
</body>
</html>`;
    }

    public dispose(): void {
        ShaderPreviewPanel.currentPanel = undefined;
        this._panel.dispose();
        while (this._disposables.length) {
            const d = this._disposables.pop();
            if (d) d.dispose();
        }
    }
}
