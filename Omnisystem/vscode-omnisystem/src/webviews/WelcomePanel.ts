import * as vscode from 'vscode';

function getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}

// ─── WelcomePanel ─────────────────────────────────────────────────────────────

export class WelcomePanel {
    public static currentPanel: WelcomePanel | undefined;
    public static readonly viewType = 'omnisystem.welcome';

    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];

    public static createOrShow(extensionUri: vscode.Uri): void {
        const column = vscode.ViewColumn.One;

        if (WelcomePanel.currentPanel) {
            WelcomePanel.currentPanel._panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            WelcomePanel.viewType,
            'Welcome to Omnisystem',
            column,
            {
                enableScripts: true,
                localResourceRoots: [extensionUri],
                retainContextWhenHidden: true,
            }
        );

        WelcomePanel.currentPanel = new WelcomePanel(panel, extensionUri);
    }

    private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
        this._panel = panel;
        this._extensionUri = extensionUri;

        this._update();

        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
        this._panel.webview.onDidReceiveMessage(
            (message) => this._handleMessage(message),
            null,
            this._disposables
        );
    }

    public static postMessage(msg: object): void {
        WelcomePanel.currentPanel?._panel.webview.postMessage(msg);
    }

    private async _handleMessage(message: { command: string; projectType?: string; theme?: string }): Promise<void> {
        switch (message.command) {
            case 'owThemeChange':
                if (message.theme) {
                    vscode.commands.executeCommand('omnisystem._broadcastTheme', message.theme);
                }
                break;
            case 'openDesktop':
                vscode.commands.executeCommand('omnisystem.openOmniOsDesktop');
                break;
            case 'openGallery':
                vscode.commands.executeCommand('omnisystem.widgetGallery');
                break;
            case 'openBuild':
                vscode.commands.executeCommand('omnisystem.openBuildDashboard');
                break;
            case 'openDocs':
                vscode.env.openExternal(vscode.Uri.parse('https://github.com/omnisystem/omnisystem'));
                break;
            case 'dismiss':
                await vscode.workspace.getConfiguration('omnisystem').update('showWelcome', false, true);
                this._panel.dispose();
                break;
            case 'selectProjectType':
                if (message.projectType) {
                    vscode.window.showInformationMessage(
                        `Project type selected: ${message.projectType}. Scaffold coming soon!`
                    );
                }
                break;
            case 'startTour':
                vscode.window.showInformationMessage('Guided tour coming soon! Use the Command Palette (Ctrl+Shift+P) → Omnisystem to explore commands.');
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
        this._panel.title = 'Welcome to Omnisystem';
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
  <title>Welcome to Omnisystem</title>
  <link rel="stylesheet" href="${widgetStyleUri}"/>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body {
      background: var(--ow-bg, #050D1A); color: var(--ow-text, #E8F4FF);
      font-family: 'Segoe UI', system-ui, sans-serif;
      margin: 0; padding: 0; min-height: 100vh;
      overflow-x: hidden;
    }

    /* ── Animated background ── */
    .bg-glow {
      position: fixed; inset: 0; pointer-events: none; z-index: 0;
      background: radial-gradient(ellipse 60% 40% at 50% -10%, rgba(0,212,255,0.12) 0%, transparent 70%),
                  radial-gradient(ellipse 40% 60% at 90% 80%, rgba(0,85,255,0.08) 0%, transparent 60%);
    }

    /* ── Stepper header ── */
    .wizard-header {
      position: relative; z-index: 10;
      padding: 28px 40px 0;
      display: flex; align-items: center; justify-content: space-between;
    }
    .logo-lockup { display: flex; align-items: center; gap: 14px; }
    .logo-icon {
      width: 48px; height: 48px;
      background: linear-gradient(135deg, var(--ow-accent, #00D4FF), var(--ow-accent-2, #0055FF));
      border-radius: 14px; display: flex; align-items: center; justify-content: center;
      font-size: 24px; box-shadow: 0 0 30px rgba(0,212,255,0.4);
    }
    .logo-text h1 {
      font-size: 22px; font-weight: 800; color: var(--ow-accent, #00D4FF);
      margin: 0; letter-spacing: -0.5px;
    }
    .logo-text p { font-size: 12px; color: var(--ow-text-muted, rgba(232,244,255,0.28)); margin: 0; }

    /* Steps indicator */
    .steps { display: flex; align-items: center; gap: 0; }
    .step {
      display: flex; align-items: center; gap: 8px;
      font-size: 11px; font-weight: 700; color: var(--ow-text-muted, rgba(232,244,255,0.28));
      transition: color var(--ow-t, 150ms);
    }
    .step.active { color: var(--ow-accent, #00D4FF); }
    .step.done { color: var(--ow-success, #00FF88); }
    .step-dot {
      width: 28px; height: 28px; border-radius: 50%;
      border: 2px solid var(--ow-border, rgba(0,212,255,0.18));
      display: flex; align-items: center; justify-content: center;
      font-size: 12px; font-weight: 700;
      background: transparent; transition: all var(--ow-t, 150ms);
    }
    .step.active .step-dot { border-color: var(--ow-accent, #00D4FF); background: rgba(0,212,255,0.13); color: var(--ow-accent, #00D4FF); }
    .step.done .step-dot { border-color: var(--ow-success, #00FF88); background: rgba(0,255,136,0.13); color: var(--ow-success, #00FF88); }
    .step-line {
      width: 40px; height: 2px;
      background: var(--ow-border, rgba(0,212,255,0.18));
      transition: background var(--ow-t, 150ms);
    }
    .step-line.done { background: var(--ow-success, #00FF88); }

    /* ── Screens ── */
    .wizard-body {
      position: relative; z-index: 10;
      max-width: 860px; margin: 0 auto;
      padding: 32px 40px 40px;
    }
    .screen { display: none; animation: fadeIn 0.3s var(--ow-ease, ease); }
    .screen.active { display: block; }
    @keyframes fadeIn { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: none; } }

    /* ── Screen 1: Hero ── */
    .hero { text-align: center; padding: 20px 0 32px; }
    .hero-badge {
      display: inline-flex; align-items: center; gap: 6px;
      background: var(--ow-accent-dim, rgba(0,212,255,0.13));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-full, 9999px);
      padding: 5px 14px; font-size: 11px; font-weight: 700;
      color: var(--ow-accent, #00D4FF); margin-bottom: 24px;
    }
    .hero h2 {
      font-size: 42px; font-weight: 800; color: var(--ow-text, #E8F4FF);
      margin: 0 0 16px; line-height: 1.1; letter-spacing: -1px;
    }
    .hero h2 span { color: var(--ow-accent, #00D4FF); }
    .hero p {
      font-size: 16px; color: var(--ow-text-dim, rgba(232,244,255,0.52));
      max-width: 540px; margin: 0 auto 32px; line-height: 1.6;
    }
    .hero-actions { display: flex; justify-content: center; gap: 12px; flex-wrap: wrap; }

    /* Lang chips */
    .lang-grid {
      display: flex; flex-wrap: wrap; gap: 10px;
      justify-content: center; margin-bottom: 32px;
    }
    .lang-chip {
      display: flex; align-items: center; gap: 8px;
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-full, 9999px);
      padding: 6px 14px; font-size: 12px; font-weight: 700;
      cursor: default; transition: all var(--ow-t, 150ms);
    }
    .lang-chip:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); transform: translateY(-2px); }
    .lang-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }

    /* Feature grid */
    .feature-grid {
      display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; margin-bottom: 28px;
    }
    @media (max-width: 600px) { .feature-grid { grid-template-columns: 1fr; } }
    .feature-card {
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-lg, 12px); padding: 20px 18px;
      transition: all var(--ow-t, 150ms);
    }
    .feature-card:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); transform: translateY(-2px); box-shadow: var(--ow-shadow-lg, 0 6px 36px rgba(0,212,255,0.14)); }
    .feature-icon { font-size: 28px; margin-bottom: 10px; }
    .feature-title { font-size: 13px; font-weight: 700; color: var(--ow-accent, #00D4FF); margin: 0 0 6px; }
    .feature-desc { font-size: 11px; color: var(--ow-text-dim, rgba(232,244,255,0.52)); line-height: 1.5; margin: 0; }

    /* ── Screen 2: Project Type ── */
    .screen-title { font-size: 26px; font-weight: 800; color: var(--ow-text, #E8F4FF); margin: 0 0 6px; }
    .screen-subtitle { font-size: 14px; color: var(--ow-text-dim, rgba(232,244,255,0.52)); margin: 0 0 28px; }
    .project-grid {
      display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; margin-bottom: 28px;
    }
    @media (max-width: 600px) { .project-grid { grid-template-columns: 1fr; } }
    .project-card {
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
      border: 2px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-lg, 12px); padding: 22px 18px;
      cursor: pointer; transition: all var(--ow-t, 150ms);
      text-align: center;
    }
    .project-card:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); transform: translateY(-2px); box-shadow: var(--ow-shadow-glow, 0 0 22px rgba(0,212,255,0.28)); }
    .project-card.selected { border-color: var(--ow-accent, #00D4FF); background: rgba(0,212,255,0.07); box-shadow: var(--ow-shadow-glow, 0 0 22px rgba(0,212,255,0.28)); }
    .project-card:focus-visible { outline: 2px solid var(--ow-accent, #00D4FF); outline-offset: 2px; }
    .project-icon { font-size: 32px; margin-bottom: 12px; }
    .project-name { font-size: 14px; font-weight: 700; color: var(--ow-text, #E8F4FF); margin: 0 0 6px; }
    .project-desc { font-size: 11px; color: var(--ow-text-dim, rgba(232,244,255,0.52)); line-height: 1.4; margin: 0; }

    /* ── Screen 3: Features ── */
    .feature-checks { display: flex; flex-direction: column; gap: 12px; margin-bottom: 28px; }
    .feature-check {
      display: flex; align-items: center; gap: 14px;
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-lg, 12px); padding: 14px 18px;
      cursor: pointer; transition: all var(--ow-t, 150ms);
    }
    .feature-check:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); }
    .feature-check.checked { border-color: var(--ow-success, #00FF88); background: rgba(0,255,136,0.05); }
    .check-box {
      width: 22px; height: 22px; border-radius: var(--ow-r-sm, 5px);
      border: 2px solid var(--ow-border, rgba(0,212,255,0.18));
      display: flex; align-items: center; justify-content: center;
      font-size: 14px; flex-shrink: 0; transition: all var(--ow-t, 150ms);
    }
    .feature-check.checked .check-box { border-color: var(--ow-success, #00FF88); background: rgba(0,255,136,0.2); color: var(--ow-success, #00FF88); }
    .check-info { flex: 1; }
    .check-title { font-size: 13px; font-weight: 700; color: var(--ow-text, #E8F4FF); margin: 0 0 2px; }
    .check-desc { font-size: 11px; color: var(--ow-text-dim, rgba(232,244,255,0.52)); margin: 0; }

    /* ── Screen 4: Ready ── */
    .ready-center { text-align: center; padding: 20px 0; }
    .ready-icon { font-size: 64px; margin-bottom: 20px; animation: pulse 2s ease-in-out infinite; }
    @keyframes pulse { 0%,100%{transform:scale(1);} 50%{transform:scale(1.05);} }
    .ready-title { font-size: 32px; font-weight: 800; color: var(--ow-success, #00FF88); margin: 0 0 12px; }
    .ready-sub { font-size: 15px; color: var(--ow-text-dim, rgba(232,244,255,0.52)); margin: 0 0 32px; line-height: 1.6; }
    .quick-actions {
      display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
      gap: 12px; margin-bottom: 28px;
    }
    .qa-btn {
      background: var(--ow-bg-card, rgba(10,20,42,0.86));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      border-radius: var(--ow-r-lg, 12px); padding: 18px 14px;
      cursor: pointer; text-align: center;
      transition: all var(--ow-t, 150ms); color: var(--ow-text, #E8F4FF);
    }
    .qa-btn:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); transform: translateY(-2px); box-shadow: var(--ow-shadow, 0 3px 16px rgba(0,212,255,0.08)); }
    .qa-btn:focus-visible { outline: 2px solid var(--ow-accent, #00D4FF); outline-offset: 2px; }
    .qa-icon { font-size: 24px; margin-bottom: 8px; }
    .qa-label { font-size: 12px; font-weight: 700; color: var(--ow-accent, #00D4FF); }
    .qa-desc { font-size: 10px; color: var(--ow-text-muted, rgba(232,244,255,0.28)); margin-top: 2px; }

    /* ── Navigation ── */
    .wizard-nav {
      display: flex; align-items: center; justify-content: space-between;
      padding-top: 20px; border-top: 1px solid var(--ow-border, rgba(0,212,255,0.18));
      margin-top: 8px;
    }
    .btn-wizard {
      border: none; border-radius: var(--ow-r-lg, 12px); padding: 10px 24px;
      font-size: 13px; font-weight: 700; cursor: pointer; transition: all var(--ow-t, 150ms);
    }
    .btn-wizard:focus-visible { outline: 2px solid var(--ow-accent, #00D4FF); outline-offset: 2px; }
    .btn-next {
      background: linear-gradient(135deg, var(--ow-accent, #00D4FF), var(--ow-accent-2, #0055FF));
      color: var(--ow-bg, #050D1A);
    }
    .btn-next:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .btn-back {
      background: transparent; color: var(--ow-text-dim, rgba(232,244,255,0.52));
      border: 1px solid var(--ow-border, rgba(0,212,255,0.18));
    }
    .btn-back:hover { border-color: var(--ow-border-focus, rgba(0,212,255,0.62)); color: var(--ow-text, #E8F4FF); }
    .btn-skip {
      background: transparent; color: var(--ow-text-muted, rgba(232,244,255,0.28)); border: none;
      font-size: 12px; cursor: pointer; padding: 8px 12px;
    }
    .btn-skip:hover { color: var(--ow-text-dim, rgba(232,244,255,0.52)); }

    /* ── Focus ring global ── */
    :focus-visible { outline: 2px solid var(--ow-accent, #00D4FF); outline-offset: 2px; }
  </style>
</head>
<body>

  <div class="bg-glow" aria-hidden="true"></div>

  <!-- Header -->
  <header class="wizard-header" role="banner">
    <div class="logo-lockup">
      <div class="logo-icon" aria-hidden="true">&#9775;</div>
      <div class="logo-text">
        <h1>Omnisystem</h1>
        <p>Next-generation Omni-Language ecosystem</p>
      </div>
    </div>

    <!-- Step indicator -->
    <nav class="steps" aria-label="Setup progress">
      <div class="step active" id="step-1" aria-current="step">
        <div class="step-dot" aria-label="Step 1: Welcome">1</div>
        <span>Welcome</span>
      </div>
      <div class="step-line" id="line-1" role="separator"></div>
      <div class="step" id="step-2">
        <div class="step-dot" aria-label="Step 2: Project">2</div>
        <span>Project</span>
      </div>
      <div class="step-line" id="line-2" role="separator"></div>
      <div class="step" id="step-3">
        <div class="step-dot" aria-label="Step 3: Features">3</div>
        <span>Features</span>
      </div>
      <div class="step-line" id="line-3" role="separator"></div>
      <div class="step" id="step-4">
        <div class="step-dot" aria-label="Step 4: Ready">4</div>
        <span>Ready</span>
      </div>
    </nav>
  </header>

  <main class="wizard-body" role="main">

    <!-- Screen 1: Welcome -->
    <section class="screen active" id="screen-1" aria-labelledby="s1-title">
      <div class="hero">
        <div class="hero-badge" role="status">&#10024; v2.0.0 — Production Ready</div>
        <h2 id="s1-title">Build with <span>Omni-Languages</span>.<br>Ship anywhere.</h2>
        <p>A complete 7-language ecosystem for building OS-grade applications — compiler, runtime, GPU shaders, ML, formal verification, and reactive UI. Everything in one place.</p>
      </div>

      <div class="lang-grid" role="list" aria-label="Omni languages">
        <div class="lang-chip" role="listitem"><span class="lang-dot" style="background:#00D4FF" aria-hidden="true"></span>Titan</div>
        <div class="lang-chip" role="listitem"><span class="lang-dot" style="background:#FF6B9D" aria-hidden="true"></span>Vera</div>
        <div class="lang-chip" role="listitem"><span class="lang-dot" style="background:#FF8C42" aria-hidden="true"></span>Helix</div>
        <div class="lang-chip" role="listitem"><span class="lang-dot" style="background:#A8E6CF" aria-hidden="true"></span>Aether</div>
        <div class="lang-chip" role="listitem"><span class="lang-dot" style="background:#DDA0DD" aria-hidden="true"></span>Axiom</div>
        <div class="lang-chip" role="listitem"><span class="lang-dot" style="background:#87CEEB" aria-hidden="true"></span>Sylva</div>
        <div class="lang-chip" role="listitem"><span class="lang-dot" style="background:#98FB98" aria-hidden="true"></span>Nexus</div>
      </div>

      <div class="feature-grid" role="list" aria-label="Key features">
        <div class="feature-card" role="listitem">
          <div class="feature-icon" aria-hidden="true">&#9889;</div>
          <p class="feature-title">7-Language Compiler</p>
          <p class="feature-desc">Full compiler pipeline with lexer, parser, type checker, IR lowering, and ELF/PE binary output.</p>
        </div>
        <div class="feature-card" role="listitem">
          <div class="feature-icon" aria-hidden="true">&#127775;</div>
          <p class="feature-title">GPU-Accelerated UI</p>
          <p class="feature-desc">Helix shader pipeline with glassmorphism, glow effects, and 6 switchable live themes.</p>
        </div>
        <div class="feature-card" role="listitem">
          <div class="feature-icon" aria-hidden="true">&#129504;</div>
          <p class="feature-title">ML & Formal Proof</p>
          <p class="feature-desc">Sylva ML runtime with BLAS dispatch + Axiom theorem prover with SMT2 emission.</p>
        </div>
        <div class="feature-card" role="listitem">
          <div class="feature-icon" aria-hidden="true">&#127968;</div>
          <p class="feature-title">OS Desktop</p>
          <p class="feature-desc">OmniOS desktop environment with window manager, file system, and 152 production systems.</p>
        </div>
        <div class="feature-card" role="listitem">
          <div class="feature-icon" aria-hidden="true">&#127807;</div>
          <p class="feature-title">Omnisystem Ecosystem</p>
          <p class="feature-desc">Tauri desktop, Android, browser extension, and control panel — all from one codebase.</p>
        </div>
        <div class="feature-card" role="listitem">
          <div class="feature-icon" aria-hidden="true">&#9775;</div>
          <p class="feature-title">Widget System</p>
          <p class="feature-desc">40+ production UI widgets in JS, CSS, TypeScript, Vera, Nexus, Titan, and Helix.</p>
        </div>
      </div>

      <div class="wizard-nav">
        <button class="btn-wizard btn-skip" onclick="dismiss()" aria-label="Dismiss welcome screen">Skip Setup</button>
        <button class="btn-wizard btn-next" onclick="goTo(2)" aria-label="Next step">Next &rarr;</button>
      </div>
    </section>

    <!-- Screen 2: Project Type -->
    <section class="screen" id="screen-2" aria-labelledby="s2-title">
      <h2 class="screen-title" id="s2-title">What are you building?</h2>
      <p class="screen-subtitle">Select your primary project type. You can always change this later.</p>

      <div class="project-grid" role="radiogroup" aria-label="Project type selection">
        <div class="project-card" id="pt-desktop" tabindex="0" role="radio" aria-checked="false"
          onclick="selectProject('desktop')" onkeydown="if(event.key==='Enter'||event.key===' ') selectProject('desktop')">
          <div class="project-icon" aria-hidden="true">&#128187;</div>
          <p class="project-name">Desktop App</p>
          <p class="project-desc">OmniOS + Tauri application with native window manager and GPU rendering.</p>
        </div>
        <div class="project-card" id="pt-compiler" tabindex="0" role="radio" aria-checked="false"
          onclick="selectProject('compiler')" onkeydown="if(event.key==='Enter'||event.key===' ') selectProject('compiler')">
          <div class="project-icon" aria-hidden="true">&#9889;</div>
          <p class="project-name">Compiler / Language</p>
          <p class="project-desc">Build with the 7-language compiler ecosystem. Parse, type-check, emit IR and binaries.</p>
        </div>
        <div class="project-card" id="pt-ml" tabindex="0" role="radio" aria-checked="false"
          onclick="selectProject('ml')" onkeydown="if(event.key==='Enter'||event.key===' ') selectProject('ml')">
          <div class="project-icon" aria-hidden="true">&#129504;</div>
          <p class="project-name">ML Model</p>
          <p class="project-desc">Sylva ML training, evaluation, and export to .syl or ONNX format.</p>
        </div>
        <div class="project-card" id="pt-mobile" tabindex="0" role="radio" aria-checked="false"
          onclick="selectProject('mobile')" onkeydown="if(event.key==='Enter'||event.key===' ') selectProject('mobile')">
          <div class="project-icon" aria-hidden="true">&#128241;</div>
          <p class="project-name">Mobile (Omnisystem Buddy)</p>
          <p class="project-desc">Android application with 9 integrated sub-apps and ADB integration.</p>
        </div>
        <div class="project-card" id="pt-ui" tabindex="0" role="radio" aria-checked="false"
          onclick="selectProject('ui')" onkeydown="if(event.key==='Enter'||event.key===' ') selectProject('ui')">
          <div class="project-icon" aria-hidden="true">&#9775;</div>
          <p class="project-name">UI Library</p>
          <p class="project-desc">Build components with Vera + Nexus + Helix. 40+ widget types ready to use.</p>
        </div>
        <div class="project-card" id="pt-server" tabindex="0" role="radio" aria-checked="false"
          onclick="selectProject('server')" onkeydown="if(event.key==='Enter'||event.key===' ') selectProject('server')">
          <div class="project-icon" aria-hidden="true">&#127760;</div>
          <p class="project-name">Server / API</p>
          <p class="project-desc">Aether actor-based server with Titan HTTP runtime and 152 production system modules.</p>
        </div>
      </div>

      <div class="wizard-nav">
        <button class="btn-wizard btn-back" onclick="goTo(1)" aria-label="Previous step">&larr; Back</button>
        <button class="btn-wizard btn-next" onclick="goTo(3)" aria-label="Next step">Next &rarr;</button>
      </div>
    </section>

    <!-- Screen 3: Features -->
    <section class="screen" id="screen-3" aria-labelledby="s3-title">
      <h2 class="screen-title" id="s3-title">Enable features</h2>
      <p class="screen-subtitle">All features are pre-configured. Toggle what you need.</p>

      <div class="feature-checks" role="group" aria-label="Feature selection">
        <div class="feature-check checked" tabindex="0" role="checkbox" aria-checked="true"
          onclick="toggleFeature(this)" onkeydown="if(event.key===' ') { event.preventDefault(); toggleFeature(this); }">
          <div class="check-box" aria-hidden="true">&#10003;</div>
          <div class="check-info">
            <p class="check-title">Widget Gallery &amp; Theme System</p>
            <p class="check-desc">40+ UI widgets with 6 switchable themes. Available via Command Palette.</p>
          </div>
        </div>
        <div class="feature-check checked" tabindex="0" role="checkbox" aria-checked="true"
          onclick="toggleFeature(this)" onkeydown="if(event.key===' ') { event.preventDefault(); toggleFeature(this); }">
          <div class="check-box" aria-hidden="true">&#10003;</div>
          <div class="check-info">
            <p class="check-title">Language Server (LSP)</p>
            <p class="check-desc">Hover, completion, and diagnostics for all 7 Omni-Languages.</p>
          </div>
        </div>
        <div class="feature-check checked" tabindex="0" role="checkbox" aria-checked="true"
          onclick="toggleFeature(this)" onkeydown="if(event.key===' ') { event.preventDefault(); toggleFeature(this); }">
          <div class="check-box" aria-hidden="true">&#10003;</div>
          <div class="check-info">
            <p class="check-title">Build Dashboard</p>
            <p class="check-desc">Visual 6-phase compiler pipeline visualizer with per-language file counts.</p>
          </div>
        </div>
        <div class="feature-check checked" tabindex="0" role="checkbox" aria-checked="true"
          onclick="toggleFeature(this)" onkeydown="if(event.key===' ') { event.preventDefault(); toggleFeature(this); }">
          <div class="check-box" aria-hidden="true">&#10003;</div>
          <div class="check-info">
            <p class="check-title">Sylva ML Studio</p>
            <p class="check-desc">Visual model architecture, training curves, epoch metrics, and export.</p>
          </div>
        </div>
        <div class="feature-check" tabindex="0" role="checkbox" aria-checked="false"
          onclick="toggleFeature(this)" onkeydown="if(event.key===' ') { event.preventDefault(); toggleFeature(this); }">
          <div class="check-box" aria-hidden="true"></div>
          <div class="check-info">
            <p class="check-title">OmniOS Desktop Environment</p>
            <p class="check-desc">Full desktop with taskbar, start menu, window manager, and app launcher.</p>
          </div>
        </div>
        <div class="feature-check" tabindex="0" role="checkbox" aria-checked="false"
          onclick="toggleFeature(this)" onkeydown="if(event.key===' ') { event.preventDefault(); toggleFeature(this); }">
          <div class="check-box" aria-hidden="true"></div>
          <div class="check-info">
            <p class="check-title">HELIX Shader Preview</p>
            <p class="check-desc">Real-time analysis of .helix files: uniforms, pipeline stages, GPU cost estimation.</p>
          </div>
        </div>
      </div>

      <div class="wizard-nav">
        <button class="btn-wizard btn-back" onclick="goTo(2)" aria-label="Previous step">&larr; Back</button>
        <button class="btn-wizard btn-next" onclick="goTo(4)" aria-label="Next step">Finish Setup &rarr;</button>
      </div>
    </section>

    <!-- Screen 4: Ready -->
    <section class="screen" id="screen-4" aria-labelledby="s4-title">
      <div class="ready-center">
        <div class="ready-icon" aria-hidden="true">&#127881;</div>
        <h2 class="ready-title" id="s4-title">You're all set!</h2>
        <p class="ready-sub">Omnisystem is configured and ready. Everything runs in VS Code — no extra setup needed. Start building with Omni-Languages now.</p>
      </div>

      <div class="quick-actions" role="list" aria-label="Quick start actions">
        <button class="qa-btn" onclick="openDesktop()" role="listitem">
          <div class="qa-icon" aria-hidden="true">&#128187;</div>
          <div class="qa-label">OmniOS Desktop</div>
          <div class="qa-desc">Launch the desktop environment</div>
        </button>
        <button class="qa-btn" onclick="openGallery()" role="listitem">
          <div class="qa-icon" aria-hidden="true">&#9775;</div>
          <div class="qa-label">Widget Gallery</div>
          <div class="qa-desc">Browse 40+ UI components</div>
        </button>
        <button class="qa-btn" onclick="openBuild()" role="listitem">
          <div class="qa-icon" aria-hidden="true">&#9889;</div>
          <div class="qa-label">Build Dashboard</div>
          <div class="qa-desc">Compile your first project</div>
        </button>
        <button class="qa-btn" onclick="startTour()" role="listitem">
          <div class="qa-icon" aria-hidden="true">&#127760;</div>
          <div class="qa-label">Guided Tour</div>
          <div class="qa-desc">Learn the workspace layout</div>
        </button>
        <button class="qa-btn" onclick="openDocs()" role="listitem">
          <div class="qa-icon" aria-hidden="true">&#128196;</div>
          <div class="qa-label">Documentation</div>
          <div class="qa-desc">Language references &amp; guides</div>
        </button>
        <button class="qa-btn" onclick="dismiss()" role="listitem">
          <div class="qa-icon" aria-hidden="true">&#10005;</div>
          <div class="qa-label">Dismiss</div>
          <div class="qa-desc">Don't show this again</div>
        </button>
      </div>

      <div class="wizard-nav">
        <button class="btn-wizard btn-back" onclick="goTo(3)" aria-label="Previous step">&larr; Back</button>
        <button class="btn-wizard btn-next" onclick="openDesktop()" aria-label="Launch OmniOS desktop">
          &#128187; Launch OmniOS
        </button>
      </div>
    </section>

  </main>

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

    var currentStep = 1;
    var selectedProject = null;

    function goTo(step) {
      // Hide current, show new
      document.querySelectorAll('.screen').forEach(function(s, i) {
        s.classList.toggle('active', (i + 1) === step);
        s.setAttribute('aria-hidden', (i + 1) !== step ? 'true' : 'false');
      });

      // Update stepper
      for (var i = 1; i <= 4; i++) {
        var stepEl = document.getElementById('step-' + i);
        var lineEl = document.getElementById('line-' + i);
        stepEl.classList.remove('active', 'done');
        if (i < step) { stepEl.classList.add('done'); stepEl.querySelector('.step-dot').textContent = '✓'; }
        else if (i === step) { stepEl.classList.add('active'); stepEl.setAttribute('aria-current', 'step'); }
        else { stepEl.removeAttribute('aria-current'); stepEl.querySelector('.step-dot').textContent = i; }
        if (lineEl) lineEl.classList.toggle('done', i < step);
      }

      currentStep = step;
      window.scrollTo(0, 0);
    }

    function selectProject(type) {
      selectedProject = type;
      document.querySelectorAll('.project-card').forEach(function(c) {
        var isThis = c.id === 'pt-' + type;
        c.classList.toggle('selected', isThis);
        c.setAttribute('aria-checked', isThis ? 'true' : 'false');
      });
      vscode.postMessage({ command: 'selectProjectType', projectType: type });
    }

    function toggleFeature(el) {
      var checked = el.classList.toggle('checked');
      el.setAttribute('aria-checked', checked ? 'true' : 'false');
      var box = el.querySelector('.check-box');
      if (box) box.innerHTML = checked ? '&#10003;' : '';
    }

    function openDesktop() { vscode.postMessage({ command: 'openDesktop' }); }
    function openGallery() { vscode.postMessage({ command: 'openGallery' }); }
    function openBuild()   { vscode.postMessage({ command: 'openBuild' }); }
    function openDocs()    { vscode.postMessage({ command: 'openDocs' }); }
    function startTour()   { vscode.postMessage({ command: 'startTour' }); }
    function dismiss()     { vscode.postMessage({ command: 'dismiss' }); }

    // Initialize: hide aria-hidden on inactive screens
    document.querySelectorAll('.screen').forEach(function(s, i) {
      if (i > 0) s.setAttribute('aria-hidden', 'true');
    });
  </script>
</body>
</html>`;
    }

    public dispose(): void {
        WelcomePanel.currentPanel = undefined;
        this._panel.dispose();
        while (this._disposables.length) {
            const d = this._disposables.pop();
            if (d) d.dispose();
        }
    }
}
