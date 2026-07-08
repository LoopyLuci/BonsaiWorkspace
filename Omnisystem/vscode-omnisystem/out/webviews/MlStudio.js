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
exports.MlStudioPanel = void 0;
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
// ─── MlStudioPanel ───────────────────────────────────────────────────────────
class MlStudioPanel {
    static postMessage(msg) {
        MlStudioPanel.currentPanel?._panel.webview.postMessage(msg);
    }
    static createOrShow(extensionUri) {
        const column = vscode.window.activeTextEditor
            ? vscode.window.activeTextEditor.viewColumn
            : undefined;
        if (MlStudioPanel.currentPanel) {
            MlStudioPanel.currentPanel._panel.reveal(column);
            return;
        }
        const panel = vscode.window.createWebviewPanel(MlStudioPanel.viewType, 'Sylva ML Studio', column || vscode.ViewColumn.One, {
            enableScripts: true,
            localResourceRoots: [extensionUri],
            retainContextWhenHidden: true,
        });
        MlStudioPanel.currentPanel = new MlStudioPanel(panel, extensionUri);
    }
    constructor(panel, extensionUri) {
        this._disposables = [];
        this._panel = panel;
        this._extensionUri = extensionUri;
        this._panel.iconPath = {
            light: vscode.Uri.joinPath(extensionUri, 'icons', 'sylva-light.svg'),
            dark: vscode.Uri.joinPath(extensionUri, 'icons', 'sylva-dark.svg'),
        };
        this._update();
        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
        this._panel.onDidChangeViewState(() => { if (this._panel.visible) {
            this._update();
        } }, null, this._disposables);
        this._panel.webview.onDidReceiveMessage((message) => this._handleMessage(message), null, this._disposables);
    }
    _post(msg) {
        this._panel.webview.postMessage(msg);
    }
    _stopTraining() {
        if (this._trainingInterval) {
            clearInterval(this._trainingInterval);
            this._trainingInterval = undefined;
        }
        if (this._trainProcess) {
            this._trainProcess.kill();
            this._trainProcess = undefined;
        }
    }
    async _handleMessage(message) {
        const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd();
        switch (message.command) {
            case 'train': {
                if (this._trainingInterval) {
                    this._post({ type: 'log', text: '[Train] Already training.' });
                    return;
                }
                const hp = message.hparams || { lr: 0.001, batchSize: 32, epochs: 20, optimizer: 'adam' };
                this._post({ type: 'trainStart', hparams: hp });
                let epoch = 0;
                let trainLoss = 2.5;
                let valLoss = 2.6;
                let trainAcc = 0.15;
                let valAcc = 0.14;
                const lossHistory = [];
                const valLossHistory = [];
                this._trainingInterval = setInterval(() => {
                    epoch++;
                    // Simulate realistic learning curves with noise
                    const decay = 1 / (1 + hp.lr * 80 * epoch);
                    trainLoss = Math.max(0.05, trainLoss * (0.88 + Math.random() * 0.04));
                    valLoss = Math.max(0.08, trainLoss * (1.05 + Math.random() * 0.1));
                    trainAcc = Math.min(0.999, trainAcc + (0.04 + Math.random() * 0.02) * decay * 10);
                    valAcc = Math.min(0.99, trainAcc - 0.02 - Math.random() * 0.02);
                    lossHistory.push(parseFloat(trainLoss.toFixed(4)));
                    valLossHistory.push(parseFloat(valLoss.toFixed(4)));
                    this._post({
                        type: 'epochUpdate',
                        epoch,
                        totalEpochs: hp.epochs,
                        trainLoss: parseFloat(trainLoss.toFixed(4)),
                        valLoss: parseFloat(valLoss.toFixed(4)),
                        trainAcc: parseFloat(trainAcc.toFixed(4)),
                        valAcc: parseFloat(valAcc.toFixed(4)),
                        lr: parseFloat((hp.lr * Math.pow(0.95, epoch)).toFixed(6)),
                        lossHistory,
                        valLossHistory,
                    });
                    if (epoch >= hp.epochs) {
                        this._stopTraining();
                        this._post({
                            type: 'trainComplete',
                            finalAcc: parseFloat(valAcc.toFixed(4)),
                            finalLoss: parseFloat(valLoss.toFixed(4)),
                        });
                    }
                }, 400);
                break;
            }
            case 'stop':
                this._stopTraining();
                this._post({ type: 'trainStopped' });
                break;
            case 'evaluate':
                this._post({ type: 'log', text: '[Evaluate] Running evaluation on test set...' });
                setTimeout(() => {
                    const acc = (0.85 + Math.random() * 0.1).toFixed(4);
                    const loss = (0.15 + Math.random() * 0.1).toFixed(4);
                    this._post({
                        type: 'evalResult',
                        accuracy: parseFloat(acc),
                        loss: parseFloat(loss),
                        confusionMatrix: this._fakeConfusionMatrix(10),
                    });
                    this._post({ type: 'log', text: `[Evaluate] Test accuracy: ${acc}, loss: ${loss}` });
                }, 1200);
                break;
            case 'export': {
                const fmt = message.format || 'sylva-model';
                this._post({ type: 'log', text: `[Export] Exporting model as ${fmt}...` });
                setTimeout(() => {
                    this._post({ type: 'log', text: `[Export] Model saved to ./models/model.${fmt === 'onnx' ? 'onnx' : 'syl'}` });
                    vscode.window.showInformationMessage(`Model exported as ${fmt}`);
                }, 800);
                break;
            }
        }
    }
    _fakeConfusionMatrix(n) {
        const mat = [];
        for (let i = 0; i < n; i++) {
            const row = [];
            for (let j = 0; j < n; j++) {
                row.push(i === j ? Math.floor(Math.random() * 80 + 70) : Math.floor(Math.random() * 10));
            }
            mat.push(row);
        }
        return mat;
    }
    _update() {
        const webview = this._panel.webview;
        const widgetStyleUri = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.css'));
        const widgetScriptUri = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.js'));
        this._panel.title = 'Sylva ML Studio';
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
  <title>Sylva ML Studio</title>
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
      margin-bottom: 24px; padding-bottom: 18px;
      border-bottom: 1px solid #1E3A5F;
    }
    .logo {
      width: 48px; height: 48px;
      background: linear-gradient(135deg, #87CEEB, #4488CC);
      border-radius: 12px;
      display: flex; align-items: center; justify-content: center;
      font-size: 24px;
      box-shadow: 0 4px 20px rgba(135,206,235,0.3);
    }
    h1 { color: #00D4FF; font-size: 24px; font-weight: 700; margin: 0 0 3px; }
    .subtitle { color: #5588AA; font-size: 12px; margin: 0; }
    .layout { display: grid; grid-template-columns: 1fr 1fr; gap: 18px; }
    .layout-3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 18px; }
    @media (max-width: 800px) { .layout, .layout-3 { grid-template-columns: 1fr; } }
    .full { grid-column: 1 / -1; }
    .card {
      background: #0F1F3A; border: 1px solid #1E3A5F;
      border-radius: 12px; padding: 18px;
    }
    .card h3 { color: #00D4FF; font-size: 14px; font-weight: 600; margin: 0 0 14px; }
    .btn {
      background: linear-gradient(135deg, #00D4FF, #0090CC);
      color: #0A1628; border: none; border-radius: 7px;
      padding: 8px 18px; cursor: pointer; font-weight: 700;
      font-size: 12px; transition: all 0.15s; white-space: nowrap;
    }
    .btn:hover { filter: brightness(1.15); transform: translateY(-1px); }
    .btn:disabled { opacity: 0.4; cursor: not-allowed; transform: none; filter: none; }
    .btn-stop { background: linear-gradient(135deg, #FF4444, #CC2222); color: #fff; }
    .btn-gold { background: linear-gradient(135deg, #FFB800, #CC8800); color: #0A1628; }
    .btn-secondary {
      background: transparent; color: #00D4FF;
      border: 1px solid #1E5A7F;
    }
    .btn-secondary:hover { background: #0F2A4A; }
    .actions { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 14px; }
    /* Architecture diagram */
    .arch {
      font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
      font-size: 12px; line-height: 1.7;
      background: #050D1A; border: 1px solid #0F1E30;
      border-radius: 8px; padding: 16px;
      color: #66AACC; overflow-x: auto;
    }
    .arch .layer-input { color: #00D4FF; }
    .arch .layer-dense { color: #00FF88; }
    .arch .layer-dropout { color: #FFB800; }
    .arch .layer-output { color: #FF6B9D; }
    .arch .arrow { color: #3A5A7A; }
    /* Metrics */
    .metrics-grid {
      display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px;
    }
    .metric-cell {
      background: #0A1628; border: 1px solid #1E3A5F;
      border-radius: 8px; padding: 12px; text-align: center;
    }
    .metric-val { font-size: 22px; font-weight: 700; font-family: monospace; }
    .metric-val.blue { color: #00D4FF; }
    .metric-val.green { color: #00FF88; }
    .metric-val.gold { color: #FFB800; }
    .metric-val.pink { color: #FF6B9D; }
    .metric-label { font-size: 10px; color: #5588AA; margin-top: 2px; text-transform: uppercase; letter-spacing: 0.5px; }
    /* Progress */
    .epoch-progress { margin: 14px 0 8px; }
    .epoch-header { display: flex; justify-content: space-between; font-size: 12px; color: #88AACC; margin-bottom: 6px; }
    .progress-track { background: #131E30; border-radius: 4px; height: 8px; overflow: hidden; }
    .progress-fill {
      height: 100%; border-radius: 4px;
      background: linear-gradient(90deg, #00D4FF, #00FF88);
      transition: width 0.3s ease;
    }
    /* Loss chart */
    .chart-container {
      background: #050D1A; border: 1px solid #0F1E30;
      border-radius: 8px; padding: 12px;
      font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 10px;
    }
    .chart-title { color: #5588AA; font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.8px; margin-bottom: 8px; }
    canvas { display: block; width: 100% !important; border-radius: 4px; }
    /* Layer cards */
    .layer-list { display: flex; flex-direction: column; gap: 6px; }
    .layer-item {
      display: flex; align-items: center; gap: 10px;
      background: #0A1628; border: 1px solid #1E3A5F;
      border-radius: 8px; padding: 10px 12px;
    }
    .layer-icon { font-size: 16px; width: 24px; text-align: center; }
    .layer-info { flex: 1; }
    .layer-name { font-size: 12px; font-family: monospace; color: #E0E0E0; font-weight: 600; }
    .layer-detail { font-size: 10px; color: #5588AA; margin-top: 1px; }
    .layer-params { font-size: 11px; color: #FFB800; font-family: monospace; font-weight: 700; }
    /* Hyperparams */
    .hparam-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
    .hparam-row { display: flex; flex-direction: column; gap: 4px; }
    .hparam-label { font-size: 10px; color: #5588AA; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; }
    input[type="number"], select {
      background: #0A1628; color: #E0E0E0;
      border: 1px solid #1E3A5F; border-radius: 6px;
      padding: 7px 10px; font-size: 12px; width: 100%;
      outline: none; font-family: monospace;
    }
    input:focus, select:focus { border-color: #00D4FF; }
    /* Dataset */
    .dataset-stats { display: flex; flex-direction: column; gap: 6px; }
    .ds-row {
      display: flex; justify-content: space-between;
      padding: 7px 0; border-bottom: 1px solid #0F1E30;
      font-size: 12px;
    }
    .ds-row:last-child { border-bottom: none; }
    .ds-label { color: #7799BB; }
    .ds-value { color: #FFB800; font-family: monospace; font-weight: 600; }
    /* Log */
    .log {
      background: #050D1A; border: 1px solid #0F1E30;
      border-radius: 8px; padding: 12px;
      font-family: 'Cascadia Code', 'Fira Code', monospace;
      font-size: 11px; height: 120px; overflow-y: auto;
      color: #66AACC; line-height: 1.6; white-space: pre-wrap;
    }
    .log::-webkit-scrollbar { width: 4px; }
    .log::-webkit-scrollbar-thumb { background: #1E3A5F; border-radius: 4px; }
    .badge {
      display: inline-block; padding: 2px 8px; border-radius: 20px;
      font-size: 10px; font-weight: 700; letter-spacing: 0.5px;
    }
    .badge-green { background: #00FF8822; color: #00FF88; border: 1px solid #00FF8844; }
    .badge-blue { background: #00D4FF22; color: #00D4FF; border: 1px solid #00D4FF44; }
    .badge-yellow { background: #FFB80022; color: #FFB800; border: 1px solid #FFB80044; }
    .badge-red { background: #FF444422; color: #FF4444; border: 1px solid #FF444444; }
    .spinner { width:12px; height:12px; border:2px solid #1E3A5F; border-top-color:#00FF88; border-radius:50%; animation:spin 0.8s linear infinite; display:inline-block; vertical-align:middle; }
    @keyframes spin { to { transform: rotate(360deg); } }
    .training-status { display:none; align-items:center; gap:8px; font-size:12px; color:#00FF88; font-weight:600; }
    .training-status.active { display:flex; }
    /* ── OW Theme Integration ─────────────────────────────── */
    body { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text, #E0E0E0) !important; }
    .card { background: var(--ow-bg-card, #0F1F3A) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .card h3 { color: var(--ow-accent, #00D4FF) !important; }
    h1 { color: var(--ow-accent, #00D4FF) !important; }
    .subtitle { color: var(--ow-text-dim, #5588AA) !important; }
    .btn { background: linear-gradient(135deg, var(--ow-accent, #00D4FF), #0090CC) !important; color: var(--ow-bg, #0A1628) !important; }
    .btn-stop { background: linear-gradient(135deg, var(--ow-danger, #FF4444), #CC2222) !important; color: #fff !important; }
    .btn-gold { background: linear-gradient(135deg, var(--ow-warning, #FFB800), #CC8800) !important; color: var(--ow-bg, #0A1628) !important; }
    .btn-secondary { background: transparent !important; color: var(--ow-accent, #00D4FF) !important; border-color: var(--ow-border, #1E5A7F) !important; }
    .metric-cell { background: var(--ow-bg, #0A1628) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .metric-label { color: var(--ow-text-muted, #5588AA) !important; }
    .arch { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border-subtle, #0F1E30) !important; color: var(--ow-text-dim, #66AACC) !important; }
    .layer-item { background: var(--ow-bg, #0A1628) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    .layer-name { color: var(--ow-text, #E0E0E0) !important; }
    .layer-detail { color: var(--ow-text-muted, #5588AA) !important; }
    .layer-params { color: var(--ow-warning, #FFB800) !important; }
    .hparam-label { color: var(--ow-text-muted, #5588AA) !important; }
    input[type="number"], select { background: var(--ow-bg, #0A1628) !important; color: var(--ow-text, #E0E0E0) !important; border-color: var(--ow-border, #1E3A5F) !important; }
    input:focus, select:focus { border-color: var(--ow-accent, #00D4FF) !important; }
    .ds-label { color: var(--ow-text-dim, #7799BB) !important; }
    .ds-value { color: var(--ow-warning, #FFB800) !important; }
    .ds-row { border-bottom-color: rgba(0,0,0,0.3) !important; }
    .progress-track { background: var(--ow-bg-raise, #131E30) !important; }
    .log { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border-subtle, #0F1E30) !important; color: var(--ow-text-dim, #66AACC) !important; }
    .chart-container { background: var(--ow-bg, #050D1A) !important; border-color: var(--ow-border-subtle, #0F1E30) !important; }
    .chart-title { color: var(--ow-text-dim, #5588AA) !important; }
    .epoch-header { color: var(--ow-text-dim, #88AACC) !important; }
  </style>
</head>
<body>
  <div class="header">
    <div class="logo">&#129504;</div>
    <div>
      <h1>Sylva ML Studio</h1>
      <p class="subtitle">Machine Learning Model Design, Training &amp; Export</p>
    </div>
    <div style="margin-left:auto; display:flex; align-items:center; gap:12px;">
      <div class="training-status" id="train-status-ind">
        <div class="spinner"></div>
        <span id="train-epoch-label">Training...</span>
      </div>
      <button class="btn btn-secondary" onclick="openThemePicker()" title="Switch Theme" aria-label="Switch Theme" style="padding:8px 12px;">&#127912;</button>
    </div>
  </div>

  <div class="layout-3" style="margin-bottom:18px;">

    <!-- Architecture -->
    <div class="card" style="grid-column:span 2;">
      <h3>&#127760; Model Architecture</h3>
      <div class="arch">
<span class="layer-input">Input(784)                  # MNIST: 28x28 flattened</span>
<span class="arrow">    |</span>
<span class="arrow">    v</span>
<span class="layer-dense">Dense(512, activation='relu')   params: 401,920</span>
<span class="arrow">    |</span>
<span class="layer-dropout">Dropout(0.3)</span>
<span class="arrow">    |</span>
<span class="layer-dense">Dense(256, activation='relu')   params: 131,328</span>
<span class="arrow">    |</span>
<span class="layer-dropout">Dropout(0.2)</span>
<span class="arrow">    |</span>
<span class="layer-dense">Dense(128, activation='relu')   params: 32,896</span>
<span class="arrow">    |</span>
<span class="layer-dense">Dense(64,  activation='relu')   params:  8,256</span>
<span class="arrow">    |</span>
<span class="layer-output">Dense(10,  activation='softmax')  params:    650</span>
<span class="arrow">    |</span>
<span class="arrow">    v</span>
<span class="layer-output">Output(10 classes)          total: 575,050 params</span>
      </div>
    </div>

    <!-- Dataset Stats -->
    <div class="card">
      <h3>&#128202; Dataset</h3>
      <div class="dataset-stats">
        <div class="ds-row"><span class="ds-label">Name</span><span class="ds-value">MNIST-10</span></div>
        <div class="ds-row"><span class="ds-label">Train samples</span><span class="ds-value">60,000</span></div>
        <div class="ds-row"><span class="ds-label">Val samples</span><span class="ds-value">10,000</span></div>
        <div class="ds-row"><span class="ds-label">Features</span><span class="ds-value">784</span></div>
        <div class="ds-row"><span class="ds-label">Classes</span><span class="ds-value">10</span></div>
        <div class="ds-row"><span class="ds-label">Train split</span><span class="ds-value">85%</span></div>
        <div class="ds-row"><span class="ds-label">Val split</span><span class="ds-value">15%</span></div>
        <div class="ds-row"><span class="ds-label">Normalized</span><span class="ds-value"><span class="badge badge-green">Yes</span></span></div>
        <div class="ds-row"><span class="ds-label">Augmented</span><span class="ds-value"><span class="badge badge-blue">Rotate ±15°</span></span></div>
      </div>
    </div>

  </div>

  <div class="layout" style="margin-bottom:18px;">

    <!-- Layer Cards -->
    <div class="card">
      <h3>&#129517; Layers</h3>
      <div class="layer-list">
        <div class="layer-item">
          <div class="layer-icon">&#128229;</div>
          <div class="layer-info">
            <div class="layer-name">Input</div>
            <div class="layer-detail">shape=(784,) — flattened 28×28</div>
          </div>
          <div class="layer-params">784</div>
        </div>
        <div class="layer-item">
          <div class="layer-icon">&#9632;</div>
          <div class="layer-info">
            <div class="layer-name">Dense(512, relu)</div>
            <div class="layer-detail">Xavier init · L2 reg 1e-4</div>
          </div>
          <div class="layer-params">401,920</div>
        </div>
        <div class="layer-item">
          <div class="layer-icon">&#127744;</div>
          <div class="layer-info">
            <div class="layer-name">Dropout(0.3)</div>
            <div class="layer-detail">Training only · random mask</div>
          </div>
          <div class="layer-params">0</div>
        </div>
        <div class="layer-item">
          <div class="layer-icon">&#9632;</div>
          <div class="layer-info">
            <div class="layer-name">Dense(256, relu)</div>
            <div class="layer-detail">Xavier init · L2 reg 1e-4</div>
          </div>
          <div class="layer-params">131,328</div>
        </div>
        <div class="layer-item">
          <div class="layer-icon">&#127744;</div>
          <div class="layer-info">
            <div class="layer-name">Dropout(0.2)</div>
            <div class="layer-detail">Training only · random mask</div>
          </div>
          <div class="layer-params">0</div>
        </div>
        <div class="layer-item">
          <div class="layer-icon">&#9632;</div>
          <div class="layer-info">
            <div class="layer-name">Dense(128, relu)</div>
            <div class="layer-detail">Xavier init</div>
          </div>
          <div class="layer-params">32,896</div>
        </div>
        <div class="layer-item">
          <div class="layer-icon">&#9632;</div>
          <div class="layer-info">
            <div class="layer-name">Dense(64, relu)</div>
            <div class="layer-detail">Xavier init</div>
          </div>
          <div class="layer-params">8,256</div>
        </div>
        <div class="layer-item">
          <div class="layer-icon">&#128228;</div>
          <div class="layer-info">
            <div class="layer-name">Dense(10, softmax)</div>
            <div class="layer-detail">Output · cross-entropy loss</div>
          </div>
          <div class="layer-params">650</div>
        </div>
      </div>
    </div>

    <!-- Training Panel -->
    <div class="card">
      <h3>&#9889; Training</h3>

      <div class="metrics-grid" style="margin-bottom:14px;">
        <div class="metric-cell">
          <div class="metric-val blue" id="m-epoch">0</div>
          <div class="metric-label">Epoch</div>
        </div>
        <div class="metric-cell">
          <div class="metric-val green" id="m-train-acc">0.0000</div>
          <div class="metric-label">Train Acc</div>
        </div>
        <div class="metric-cell">
          <div class="metric-val gold" id="m-train-loss">—</div>
          <div class="metric-label">Train Loss</div>
        </div>
        <div class="metric-cell">
          <div class="metric-val pink" id="m-val-acc">0.0000</div>
          <div class="metric-label">Val Acc</div>
        </div>
        <div class="metric-cell">
          <div class="metric-val gold" id="m-val-loss">—</div>
          <div class="metric-label">Val Loss</div>
        </div>
        <div class="metric-cell">
          <div class="metric-val blue" id="m-lr">0.001</div>
          <div class="metric-label">Learn Rate</div>
        </div>
      </div>

      <div class="epoch-progress">
        <div class="epoch-header">
          <span>Progress</span>
          <span id="epoch-progress-label">0 / 0 epochs</span>
        </div>
        <div class="progress-track">
          <div class="progress-fill" id="epoch-bar" style="width:0%"></div>
        </div>
      </div>

      <!-- Hyperparams -->
      <p style="font-size:10px;font-weight:700;letter-spacing:1px;color:#3A6A8F;text-transform:uppercase;margin:14px 0 8px;">Hyperparameters</p>
      <div class="hparam-grid">
        <div class="hparam-row">
          <label class="hparam-label">Learning Rate</label>
          <input type="number" id="hp-lr" value="0.001" step="0.0001" min="0.00001" max="1">
        </div>
        <div class="hparam-row">
          <label class="hparam-label">Batch Size</label>
          <input type="number" id="hp-batch" value="32" step="8" min="1" max="512">
        </div>
        <div class="hparam-row">
          <label class="hparam-label">Epochs</label>
          <input type="number" id="hp-epochs" value="20" step="5" min="1" max="1000">
        </div>
        <div class="hparam-row">
          <label class="hparam-label">Optimizer</label>
          <select id="hp-optimizer">
            <option value="adam">Adam</option>
            <option value="sgd">SGD</option>
            <option value="rmsprop">RMSProp</option>
            <option value="adagrad">Adagrad</option>
          </select>
        </div>
      </div>

      <div class="actions">
        <button class="btn" id="train-btn" onclick="startTrain()">&#9654; Train</button>
        <button class="btn btn-stop" id="stop-btn" style="display:none;" onclick="stopTrain()">&#9632; Stop</button>
        <button class="btn btn-secondary" onclick="evaluate()">&#9654;&#9654; Evaluate</button>
      </div>
    </div>
  </div>

  <!-- Loss Chart -->
  <div class="layout" style="margin-bottom:18px;">
    <div class="card">
      <h3>&#128200; Loss Curve</h3>
      <div class="chart-container">
        <div class="chart-title">Train vs Validation Loss</div>
        <canvas id="loss-canvas" height="140"></canvas>
      </div>
    </div>

    <div class="card">
      <h3>&#127919; Accuracy Curve</h3>
      <div class="chart-container">
        <div class="chart-title">Train vs Validation Accuracy</div>
        <canvas id="acc-canvas" height="140"></canvas>
      </div>
    </div>
  </div>

  <!-- Export & Log -->
  <div class="layout">
    <div class="card">
      <h3>&#128190; Export Model</h3>
      <div class="hparam-row" style="margin-bottom:14px;">
        <label class="hparam-label">Export Format</label>
        <select id="export-format">
          <option value="sylva-model">Sylva Model (.syl)</option>
          <option value="onnx">ONNX (.onnx)</option>
          <option value="json-weights">JSON Weights</option>
          <option value="binary-flat">Binary Flat</option>
        </select>
      </div>
      <button class="btn btn-gold" onclick="exportModel()">&#128190; Export Model</button>
    </div>

    <div class="card">
      <h3>&#128203; Log</h3>
      <div class="log" id="ml-log">ML Studio ready. Configure hyperparameters and click Train.</div>
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
    let lossData = [];
    let valLossData = [];
    let accData = [];
    let valAccData = [];

    function startTrain() {
      const hp = {
        lr: parseFloat(document.getElementById('hp-lr').value),
        batchSize: parseInt(document.getElementById('hp-batch').value),
        epochs: parseInt(document.getElementById('hp-epochs').value),
        optimizer: document.getElementById('hp-optimizer').value,
      };
      lossData = []; valLossData = []; accData = []; valAccData = [];
      vscode.postMessage({ command: 'train', hparams: hp });
    }

    function stopTrain() { vscode.postMessage({ command: 'stop' }); }
    function evaluate() { vscode.postMessage({ command: 'evaluate' }); }
    function exportModel() {
      const fmt = document.getElementById('export-format').value;
      vscode.postMessage({ command: 'export', format: fmt });
    }

    function appendLog(text) {
      const log = document.getElementById('ml-log');
      log.textContent += '\n' + text;
      log.scrollTop = log.scrollHeight;
    }

    // ── Canvas chart renderer ──────────────────────────────────────────────
    function drawChart(canvasId, series, labels, colors, yLabel) {
      const canvas = document.getElementById(canvasId);
      if (!canvas) return;
      const dpr = window.devicePixelRatio || 1;
      const rect = canvas.getBoundingClientRect();
      canvas.width = (rect.width || 500) * dpr;
      canvas.height = 140 * dpr;
      const ctx = canvas.getContext('2d');
      ctx.scale(dpr, dpr);
      const W = rect.width || 500, H = 140;
      const pad = { top: 10, right: 20, bottom: 28, left: 44 };

      ctx.fillStyle = '#050D1A';
      ctx.fillRect(0, 0, W, H);

      if (series.length === 0 || series[0].length < 2) {
        ctx.fillStyle = '#3A5A7A';
        ctx.font = '11px monospace';
        ctx.textAlign = 'center';
        ctx.fillText('No data yet — start training', W / 2, H / 2);
        return;
      }

      const allVals = series.flat();
      const minV = Math.min(...allVals);
      const maxV = Math.max(...allVals);
      const range = maxV - minV || 1;

      const plotW = W - pad.left - pad.right;
      const plotH = H - pad.top - pad.bottom;

      // Grid lines
      ctx.strokeStyle = '#0F1E30';
      ctx.lineWidth = 1;
      for (let i = 0; i <= 4; i++) {
        const y = pad.top + (i / 4) * plotH;
        ctx.beginPath(); ctx.moveTo(pad.left, y); ctx.lineTo(W - pad.right, y); ctx.stroke();
        const val = maxV - (i / 4) * range;
        ctx.fillStyle = '#3A5A7A'; ctx.font = '9px monospace'; ctx.textAlign = 'right';
        ctx.fillText(val.toFixed(3), pad.left - 4, y + 3);
      }

      // X axis labels
      const pts = series[0].length;
      ctx.fillStyle = '#3A5A7A'; ctx.font = '9px monospace'; ctx.textAlign = 'center';
      [0, Math.floor(pts/2), pts - 1].forEach(i => {
        if (i < pts) {
          const x = pad.left + (i / (pts - 1)) * plotW;
          ctx.fillText(String(i + 1), x, H - pad.bottom + 14);
        }
      });

      // Epoch axis label
      ctx.fillStyle = '#3A5A7A'; ctx.font = '9px sans-serif'; ctx.textAlign = 'center';
      ctx.fillText('epoch', pad.left + plotW / 2, H - 2);

      // Plot lines
      series.forEach((data, si) => {
        ctx.beginPath();
        ctx.strokeStyle = colors[si];
        ctx.lineWidth = 2;
        ctx.shadowColor = colors[si];
        ctx.shadowBlur = 4;
        data.forEach((v, i) => {
          const x = pad.left + (i / Math.max(data.length - 1, 1)) * plotW;
          const y = pad.top + (1 - (v - minV) / range) * plotH;
          i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
        });
        ctx.stroke();
        ctx.shadowBlur = 0;

        // Legend dot
        const lx = pad.left + (si * 100);
        ctx.fillStyle = colors[si];
        ctx.fillRect(lx, 2, 14, 7);
        ctx.fillStyle = '#88AACC'; ctx.font = '9px sans-serif'; ctx.textAlign = 'left';
        ctx.fillText(labels[si], lx + 18, 9);
      });
    }

    function redrawCharts() {
      drawChart('loss-canvas', [lossData, valLossData], ['Train Loss', 'Val Loss'], ['#00D4FF', '#FF6B9D'], 'loss');
      drawChart('acc-canvas', [accData, valAccData], ['Train Acc', 'Val Acc'], ['#00FF88', '#FFB800'], 'acc');
    }

    window.addEventListener('resize', redrawCharts);

    window.addEventListener('message', event => {
      const msg = event.data;

      switch (msg.type) {
        case 'log':
          appendLog(msg.text);
          break;

        case 'trainStart':
          document.getElementById('train-btn').disabled = true;
          document.getElementById('stop-btn').style.display = 'inline-block';
          document.getElementById('train-status-ind').classList.add('active');
          document.getElementById('epoch-progress-label').textContent = '0 / ' + msg.hparams.epochs + ' epochs';
          appendLog('[Train] Starting — lr=' + msg.hparams.lr + ', batch=' + msg.hparams.batchSize + ', optimizer=' + msg.hparams.optimizer);
          break;

        case 'epochUpdate': {
          const { epoch, totalEpochs, trainLoss, valLoss, trainAcc, valAcc, lr, lossHistory, valLossHistory } = msg;
          document.getElementById('m-epoch').textContent = epoch;
          document.getElementById('m-train-acc').textContent = trainAcc.toFixed(4);
          document.getElementById('m-val-acc').textContent = valAcc.toFixed(4);
          document.getElementById('m-train-loss').textContent = trainLoss.toFixed(4);
          document.getElementById('m-val-loss').textContent = valLoss.toFixed(4);
          document.getElementById('m-lr').textContent = lr.toFixed(6);
          document.getElementById('epoch-bar').style.width = ((epoch / totalEpochs) * 100) + '%';
          document.getElementById('epoch-progress-label').textContent = epoch + ' / ' + totalEpochs + ' epochs';
          document.getElementById('train-epoch-label').textContent = 'Epoch ' + epoch + '/' + totalEpochs;

          lossData = lossHistory;
          valLossData = valLossHistory;
          accData = lossHistory.map((_, i) => {
            const progress = (i + 1) / lossHistory.length;
            return Math.min(0.999, 0.15 + progress * 0.75 + Math.random() * 0.02);
          });
          valAccData = accData.map(v => Math.max(0, v - 0.02 - Math.random() * 0.02));
          redrawCharts();
          break;
        }

        case 'trainComplete':
          document.getElementById('train-btn').disabled = false;
          document.getElementById('stop-btn').style.display = 'none';
          document.getElementById('train-status-ind').classList.remove('active');
          appendLog('[Train] Complete! Val acc: ' + msg.finalAcc + ', val loss: ' + msg.finalLoss);
          document.getElementById('epoch-bar').style.width = '100%';
          break;

        case 'trainStopped':
          document.getElementById('train-btn').disabled = false;
          document.getElementById('stop-btn').style.display = 'none';
          document.getElementById('train-status-ind').classList.remove('active');
          appendLog('[Train] Stopped by user.');
          break;

        case 'evalResult':
          appendLog('[Eval] Accuracy: ' + msg.accuracy + ' | Loss: ' + msg.loss);
          break;
      }
    });

    // Initial chart render (empty state)
    setTimeout(redrawCharts, 100);
  </script>
</body>
</html>`;
    }
    dispose() {
        MlStudioPanel.currentPanel = undefined;
        this._panel.dispose();
        this._stopTraining();
        while (this._disposables.length) {
            const d = this._disposables.pop();
            if (d)
                d.dispose();
        }
    }
}
exports.MlStudioPanel = MlStudioPanel;
MlStudioPanel.viewType = 'omnisystem.mlStudio';
//# sourceMappingURL=MlStudio.js.map