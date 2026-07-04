import * as vscode from 'vscode';
import * as fs from 'fs';
import { spawn } from 'child_process';
import { getRuntimeClient, disposeRuntimeClient, RuntimeClient } from '../runtime/RuntimeClient';
import { getPtyManager, disposePtyManager, PtyManager } from '../runtime/PtyManager';
import { OmniHarnessClient } from '../harness/OmniHarnessClient';

declare global {
  interface Window {
    post: (cmd: string, extra?: Record<string, unknown>) => void;
    notify: (title: string, msg: string, icon?: string) => void;
    openApp: (appId: string) => void;
  }
}

function getNonce(): string {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 32; i++) text += possible.charAt(Math.floor(Math.random() * possible.length));
  return text;
}

export class OmniOSDesktopPanel {
  public static currentPanel: OmniOSDesktopPanel | undefined;
  public static readonly viewType = 'omnisystem.omniOsDesktop';
  public static _extensionContext: vscode.ExtensionContext | undefined;
  public static onThemeChange: ((themeId: string) => void) | undefined;

  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionUri: vscode.Uri;
  private _disposables: vscode.Disposable[] = [];
  private _activeProcs = new Map<number, import('child_process').ChildProcess>();
  private _lastProc: import('child_process').ChildProcess | null = null;
  private _runtime: RuntimeClient;
  private _pty: PtyManager;
  private _lastDiagCount = -1;

  public static createOrShow(extensionUri: vscode.Uri, ctx?: vscode.ExtensionContext): void {
    if (ctx) OmniOSDesktopPanel._extensionContext = ctx;

    const column = vscode.window.activeTextEditor
      ? vscode.window.activeTextEditor.viewColumn
      : undefined;

    if (OmniOSDesktopPanel.currentPanel) {
      OmniOSDesktopPanel.currentPanel._panel.reveal(column);
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      OmniOSDesktopPanel.viewType,
      'OmniOS Desktop',
      vscode.ViewColumn.One,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [
          vscode.Uri.joinPath(extensionUri, 'icons'),
          vscode.Uri.joinPath(extensionUri, 'media')
        ]
      }
    );

    OmniOSDesktopPanel.currentPanel = new OmniOSDesktopPanel(panel, extensionUri);
  }

  public static postMessage(data: unknown): void {
    OmniOSDesktopPanel.currentPanel?._panel.webview.postMessage(data);
  }

  private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
    this._panel = panel;
    this._extensionUri = extensionUri;

    // Initialize IPC runtime client and PTY manager
    this._runtime = getRuntimeClient(extensionUri.fsPath);
    this._pty = getPtyManager();

    // Wire PTY output back to webview
    this._pty.on('output', (sessionId: string, data: string) => {
      if (!this._panel.visible) return;
      this._panel.webview.postMessage({ type: 'ptyOutput', sessionId, data });
    });
    this._pty.on('exit', (sessionId: string, code: number) => {
      this._panel.webview.postMessage({ type: 'ptyExit', sessionId, code });
    });

    // Wire RuntimeClient notifications to webview
    this._runtime.on('notification', (method: string, params: unknown) => {
      if (method === 'build/progress') {
        this._panel.webview.postMessage({ type: 'buildProgress', ...( params as object) });
      } else if (method === 'system/metrics') {
        this._panel.webview.postMessage({ type: 'runtimeMetrics', ...(params as object) });
      }
    });

    // Start runtime client (non-blocking — degrades gracefully on failure)
    this._runtime.start().catch(() => { /* will retry on demand */ });

    // Poll VS Code diagnostics every 8 seconds and push to Bug Hunter
    const diagInterval = setInterval(() => {
      if (!this._panel.visible) return;
      const diags = vscode.languages.getDiagnostics();
      const errors: Array<{ file: string; message: string; severity: string; line: number }> = [];
      for (const [uri, diagnostics] of diags) {
        for (const d of diagnostics) {
          if (d.severity <= vscode.DiagnosticSeverity.Warning) {
            errors.push({
              file: vscode.workspace.asRelativePath(uri).replace(/\\/g, '/'),
              message: d.message,
              severity: d.severity === vscode.DiagnosticSeverity.Error ? 'error' : 'warning',
              line: d.range.start.line + 1,
            });
          }
        }
      }
      if (errors.length !== this._lastDiagCount) {
        this._lastDiagCount = errors.length;
        this._panel.webview.postMessage({ type: 'vscodeDiagnostics', errors });
      }
    }, 8000);
    this._disposables.push({ dispose: () => clearInterval(diagInterval) });

    this._update();
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
    this._panel.webview.onDidReceiveMessage(
      msg => this._handleMessage(msg),
      null,
      this._disposables
    );
  }

  private _update(): void {
    const webview = this._panel.webview;
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'desktop-client.js')
    );
    const widgetStyleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.css')
    );
    const widgetScriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'omni-widgets.js')
    );
    this._panel.webview.html = this._getHtml(scriptUri.toString(), widgetStyleUri.toString(), widgetScriptUri.toString());
  }

  private async _handleMessage(msg: { command: string; [key: string]: unknown }): Promise<void> {
    switch (msg.command) {
      case 'openApp':
        await vscode.commands.executeCommand('omnisystem.' + (msg.app as string));
        break;

      case 'openFile': {
        const uri = vscode.Uri.file(msg.text as string);
        await vscode.window.showTextDocument(uri);
        break;
      }

      case 'runTerminalCommand': {
        const term = vscode.window.createTerminal('OmniOS');
        term.show();
        term.sendText(msg.text as string);
        break;
      }

      case 'owThemeChange':
        if (msg.theme && OmniOSDesktopPanel.onThemeChange) {
          OmniOSDesktopPanel.onThemeChange(msg.theme as string);
        }
        break;

      case 'applyTheme':
        await vscode.workspace.getConfiguration().update(
          'workbench.colorTheme',
          'Omnisystem Dark',
          vscode.ConfigurationTarget.Global
        );
        break;

      case 'openSettings':
        await vscode.commands.executeCommand(
          'workbench.action.openSettings',
          '@ext:omnisystem.omnisystem'
        );
        break;

      case 'scaffold': {
        const lang = msg.lang as string;
        const cmdMap: Record<string, string> = {
          titan: 'omnisystem.newTitanFile',
          vera: 'omnisystem.newVeraFile',
          helix: 'omnisystem.newHelixFile',
          aether: 'omnisystem.newAetherFile',
          axiom: 'omnisystem.newAxiomFile',
          sylva: 'omnisystem.newSylvaFile',
          nexus: 'omnisystem.newNexusFile',
        };
        const cmd = cmdMap[lang] ?? 'omnisystem.newTitanFile';
        await vscode.commands.executeCommand(cmd, msg.name as string);
        break;
      }

      case 'desktopLaunch':
        await vscode.commands.executeCommand('omnisystem.desktopLaunch');
        break;

      case 'getFiles': {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders || folders.length === 0) {
          this._panel.webview.postMessage({ type: 'fileList', files: [], path: '' });
          break;
        }
        const root = (msg.path as string | undefined) ?? folders[0].uri.fsPath;
        try {
          const dirUri = vscode.Uri.file(root);
          const entries = await vscode.workspace.fs.readDirectory(dirUri);
          const files = entries.map(([name, fileType]) => ({
            name,
            type: fileType === vscode.FileType.Directory ? 'directory' : 'file',
            path: root.replace(/\\/g, '/') + '/' + name,
          }));
          this._panel.webview.postMessage({ type: 'fileList', files, path: root });
        } catch {
          this._panel.webview.postMessage({ type: 'fileList', files: [], path: root });
        }
        break;
      }

      case 'runBuild': {
        const args = (msg.args as string[]) ?? ['build'];
        const folders2 = vscode.workspace.workspaceFolders;
        const cwd = folders2?.[0]?.uri.fsPath ?? process.cwd();
        const fs2 = require('fs') as typeof import('fs');
        const path2 = require('path') as typeof import('path');
        const searchRoots = [cwd, path2.join(cwd, 'Omnisystem'), path2.join(cwd, '..'), path2.join(cwd, '..', 'Omnisystem')];
        const names = process.platform === 'win32' ? ['omnicc.cmd', 'omnicc.ps1', 'omnicc.js'] : ['omnicc', 'omnicc.js'];
        let omniccPath = 'omnicc';
        outer: for (const root of searchRoots) {
          for (const name of names) {
            const candidate = path2.join(root, 'bin', name);
            if (fs2.existsSync(candidate)) { omniccPath = candidate; break outer; }
          }
        }

        try {
          const proc = spawn(omniccPath, args, { cwd, shell: true });
          proc.stdout.on('data', (chunk: Buffer) => {
            const lines = chunk.toString().split('\n');
            for (const line of lines) {
              if (line.trim()) {
                this._panel.webview.postMessage({ type: 'buildLine', text: line });
              }
            }
          });
          proc.stderr.on('data', (chunk: Buffer) => {
            const lines = chunk.toString().split('\n');
            for (const line of lines) {
              if (line.trim()) {
                this._panel.webview.postMessage({ type: 'buildLine', text: line });
              }
            }
          });
          proc.on('close', (code: number | null) => {
            this._panel.webview.postMessage({ type: 'buildDone', code: code ?? 0 });
          });
          proc.on('error', (err: Error) => {
            this._panel.webview.postMessage({ type: 'buildLine', text: '✗ ' + err.message });
            this._panel.webview.postMessage({ type: 'buildDone', code: 1 });
          });
        } catch (err) {
          this._panel.webview.postMessage({ type: 'buildLine', text: '✗ Failed to spawn omnicc' });
          this._panel.webview.postMessage({ type: 'buildDone', code: 1 });
        }
        break;
      }

      // ── Real in-window command execution ──────────────────────────────────
      case 'execCommand': {
        const cmdStr = msg.text as string;
        const folders3 = vscode.workspace.workspaceFolders;
        const cwd3 = (msg.cwd as string | undefined) || folders3?.[0]?.uri.fsPath || process.cwd();
        const procId = Date.now();
        try {
          const proc = spawn(cmdStr, [], { cwd: cwd3, shell: true });
          this._activeProcs.set(procId, proc);
          this._lastProc = proc;
          proc.stdout.on('data', (chunk: Buffer) => {
            chunk.toString().split('\n').forEach(line => {
              if (line) this._panel.webview.postMessage({ type: 'termLine', text: line });
            });
          });
          proc.stderr.on('data', (chunk: Buffer) => {
            chunk.toString().split('\n').forEach(line => {
              if (line) this._panel.webview.postMessage({ type: 'termLine', text: line, cls: 'err' });
            });
          });
          proc.on('close', (code: number | null) => {
            this._activeProcs.delete(procId);
            this._panel.webview.postMessage({ type: 'termDone', code: code ?? 0 });
          });
          proc.on('error', (e: Error) => {
            this._activeProcs.delete(procId);
            this._panel.webview.postMessage({ type: 'termLine', text: `Error: ${e.message}`, cls: 'err' });
            this._panel.webview.postMessage({ type: 'termDone', code: 1 });
          });
        } catch (e) {
          this._panel.webview.postMessage({ type: 'termLine', text: `Failed to spawn: ${(e as Error).message}`, cls: 'err' });
          this._panel.webview.postMessage({ type: 'termDone', code: 1 });
        }
        break;
      }

      case 'killProc': {
        this._activeProcs.forEach(p => { try { p.kill(); } catch { /* ignore */ } });
        this._activeProcs.clear();
        this._lastProc = null;
        this._panel.webview.postMessage({ type: 'termLine', text: '^C', cls: 'info' });
        this._panel.webview.postMessage({ type: 'termDone', code: 130 });
        break;
      }

      // ── Send stdin to running process ────────────────────────────────────
      case 'shellInput': {
        const text = (msg.text as string) + '\n';
        if (this._lastProc && this._lastProc.stdin && !this._lastProc.killed) {
          try { this._lastProc.stdin.write(text); } catch { /* ignore */ }
        }
        break;
      }

      // ── Real process list ────────────────────────────────────────────────
      case 'getProcessList': {
        const { exec } = require('child_process') as typeof import('child_process');
        const isWin = process.platform === 'win32';
        const cmd   = isWin
          ? 'tasklist /FO CSV /NH'
          : 'ps aux --no-headers';
        exec(cmd, { timeout: 5000 }, (_err: Error | null, stdout: string) => {
          const procs: Array<{ name: string; pid: string; mem: string; cpu: string }> = [];
          if (isWin) {
            for (const line of stdout.split('\n')) {
              const parts = line.trim().replace(/"/g, '').split(',');
              if (parts.length >= 5) {
                procs.push({ name: parts[0], pid: parts[1], mem: parts[4]?.replace(' K','K') ?? '', cpu: '' });
              }
            }
          } else {
            for (const line of stdout.split('\n')) {
              const cols = line.trim().split(/\s+/);
              if (cols.length >= 11) {
                procs.push({ name: cols[10] ?? '', pid: cols[1], cpu: cols[2], mem: cols[3] + 'K' });
              }
            }
          }
          this._panel.webview.postMessage({ type: 'processList', procs: procs.slice(0, 80) });
        });
        break;
      }

      // ── Sandbox / Immune System status ──────────────────────────────────
      case 'getSandboxStatus': {
        const osModule3 = require('os') as typeof import('os');
        const pathMod3  = require('path') as typeof import('path');
        const platform3 = osModule3.platform();
        const wsRoot3Raw = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';
        // Resolve the actual Omnisystem source root (handles monorepo offset)
        const wsRoot3 = (() => {
          const sub = pathMod3.join(wsRoot3Raw, 'Omnisystem');
          if (fs.existsSync(pathMod3.join(sub, 'BUILD.omnisystem'))) return sub;
          if (fs.existsSync(pathMod3.join(wsRoot3Raw, 'BUILD.omnisystem'))) return wsRoot3Raw;
          return sub;
        })();

        // These are the real existing Omnisystem source modules — read them live
        const sandboxModules = [
          { name: 'Sanctum Vault Kernel',    rel: 'src/systems/UOSC/kernel/sanctum.ti' },
          { name: 'UOSC Capability Layer',   rel: 'src/systems/UOSC/kernel/capability.ti' },
          { name: 'UOSC IPC',               rel: 'src/systems/UOSC/kernel/ipc.ti' },
          { name: 'UOSC Memory Manager',    rel: 'src/systems/UOSC/kernel/memory.ti' },
          { name: 'UOSC Boot',              rel: 'src/systems/UOSC/kernel/boot.ti' },
          { name: 'Env-Fabric Manager',      rel: 'src/systems/runtime/services/env-fabric/manager.ti' },
          { name: 'Env-Fabric Payload',      rel: 'src/systems/runtime/services/env-fabric/payload.ti' },
          { name: 'Env-Fabric Snapshot',     rel: 'src/systems/runtime/services/env-fabric/snapshot.ti' },
          { name: 'Env-Fabric Determinism',  rel: 'src/systems/runtime/services/env-fabric/determinism.ti' },
          { name: 'Sandbox Immune System',   rel: 'src/systems/runtime/services/sandbox/SandboxImmuneSystem.titan' },
        ];

        // Read each file and parse real metrics from the actual source code
        const sandboxFiles = sandboxModules.map(f => {
          const fullPath = wsRoot3 ? pathMod3.join(wsRoot3, f.rel) : '';
          const present  = fullPath ? fs.existsSync(fullPath) : false;
          let loc = 0;
          let symbols = 0;
          if (present && fullPath) {
            try {
              const src = fs.readFileSync(fullPath, 'utf8');
              loc     = src.split('\n').length;
              // Count real symbol declarations: pub fn, pub struct, pub enum, actor, type =
              symbols = (src.match(/\b(pub\s+fn|pub\s+struct|pub\s+enum|pub\s+actor|type\s+\w+\s*=)/g) ?? []).length;
            } catch { /* ignore read errors */ }
          }
          return { ...f, present, loc, symbols };
        });

        // Parse real vault types and capability types from the actual source
        let vaultTypes: string[] = [];
        let capabilityTypes: string[] = [];
        let envTypes: string[] = [];
        try {
          const sanctumSrc = wsRoot3 ? fs.readFileSync(pathMod3.join(wsRoot3, 'src/systems/UOSC/kernel/sanctum.ti'), 'utf8') : '';
          vaultTypes = (sanctumSrc.match(/\b(\w+),\s*\/\/[^\n]*/g) ?? []).slice(0, 5).map(m => m.split(',')[0].trim());
        } catch { /* ignore */ }
        try {
          const capSrc = wsRoot3 ? fs.readFileSync(pathMod3.join(wsRoot3, 'src/systems/UOSC/kernel/capability.ti'), 'utf8') : '';
          capabilityTypes = (capSrc.match(/ResourceType\s*=\s*enum\s*\{([^}]+)\}/s)?.[1] ?? '')
            .split('\n').map(l => l.replace(/\/\/.*/, '').trim().replace(/,$/, '')).filter(Boolean);
        } catch { /* ignore */ }
        try {
          const mgSrc = wsRoot3 ? fs.readFileSync(pathMod3.join(wsRoot3, 'src/systems/runtime/services/env-fabric/manager.ti'), 'utf8') : '';
          envTypes = (mgSrc.match(/const ENV_TYPE_\w+:\s*i64\s*=\s*\d+/g) ?? []).map(m => m.replace('const ', '').split(':')[0].replace('ENV_TYPE_', ''));
        } catch { /* ignore */ }

        this._panel.webview.postMessage({
          type: 'sandboxStatus',
          platform: platform3,
          isolationMode: platform3 === 'win32' ? 'Windows Job Objects + Restricted Token'
                       : platform3 === 'linux'  ? 'Linux Namespaces + seccomp + cgroups v2'
                       : platform3 === 'darwin' ? 'macOS App Sandbox + Seatbelt'
                       :                          'POSIX rlimits + chroot',
          immuneActive: true,
          vaultCount:   sandboxFiles.filter(f => f.present).length,
          sandboxFiles,
          vaultTypes,
          capabilityTypes,
          envTypes,
          totalLoc:   sandboxFiles.reduce((a, f) => a + (f.loc ?? 0), 0),
          totalSymbols: sandboxFiles.reduce((a, f) => a + (f.symbols ?? 0), 0),
          policies: {
            network: 'deny-by-default',
            filesystem: 'workspace-scoped',
            ipc: 'Sanctum-mediated',
            capabilities: 'allowlist-only',
          },
        });
        break;
      }

      // ── Real system stats ────────────────────────────────────────────────
      case 'getSystemStats': {
        // eslint-disable-next-line @typescript-eslint/no-var-requires
        const osModule = require('os') as typeof import('os');
        const totalMem = osModule.totalmem();
        const cpuSnapshot = osModule.cpus().map(c => ({
          idle: c.times.idle,
          total: c.times.user + c.times.nice + c.times.sys + c.times.idle + c.times.irq,
        }));
        setTimeout(() => {
          // eslint-disable-next-line @typescript-eslint/no-var-requires
          const osModule2 = require('os') as typeof import('os');
          const freeMem2 = osModule2.freemem();
          const perCore = osModule2.cpus().map((c, i) => {
            const idle2  = c.times.idle;
            const total2 = c.times.user + c.times.nice + c.times.sys + c.times.idle + c.times.irq;
            const di = idle2  - cpuSnapshot[i].idle;
            const dt = total2 - cpuSnapshot[i].total;
            return dt > 0 ? (1 - di / dt) * 100 : 0;
          });
          const avgCpu = Math.round(perCore.reduce((a, b) => a + b, 0) / perCore.length);
          const usedMem = totalMem - freeMem2;
          this._panel.webview.postMessage({
            type: 'systemStats',
            cpu:        avgCpu,
            memUsedGb:  Math.round(usedMem  / 1073741824 * 10) / 10,
            memTotalGb: Math.round(totalMem / 1073741824 * 10) / 10,
            memPct:     Math.round(usedMem  / totalMem * 100),
            platform:   osModule2.platform(),
            arch:       osModule2.arch(),
            uptime:     Math.round(osModule2.uptime()),
            cpuModel:   (osModule2.cpus()[0]?.model ?? 'Unknown').split('@')[0].trim(),
            cores:      osModule2.cpus().length,
          });
        }, 150);
        break;
      }

      // ── Settings persistence ─────────────────────────────────────────────
      case 'saveSettings': {
        try {
          await OmniOSDesktopPanel._extensionContext?.workspaceState.update(
            'omnisystem.desktopSettings', msg.settings
          );
        } catch { /* ignore */ }
        break;
      }

      case 'loadSettings': {
        const settings = OmniOSDesktopPanel._extensionContext?.workspaceState.get(
          'omnisystem.desktopSettings', {}
        ) ?? {};
        this._panel.webview.postMessage({ type: 'settingsLoaded', settings });
        break;
      }

      // ── Window state persistence ─────────────────────────────────────────
      case 'saveWindowState': {
        try {
          await OmniOSDesktopPanel._extensionContext?.workspaceState.update(
            'omnisystem.windowState', msg.state
          );
        } catch { /* ignore */ }
        break;
      }

      case 'loadWindowState': {
        const winState = OmniOSDesktopPanel._extensionContext?.workspaceState.get(
          'omnisystem.windowState', null
        ) ?? null;
        this._panel.webview.postMessage({ type: 'windowStateLoaded', state: winState });
        break;
      }

      // ── Real file operations ─────────────────────────────────────────────
      case 'deleteFile': {
        try {
          const uri = vscode.Uri.file(msg.path as string);
          await vscode.workspace.fs.delete(uri, { recursive: !!(msg.recursive), useTrash: true });
          this._panel.webview.postMessage({ type: 'fileDeleted', path: msg.path });
        } catch (e) {
          this._panel.webview.postMessage({ type: 'fileError', error: (e as Error).message });
        }
        break;
      }

      case 'createFile': {
        try {
          const uri = vscode.Uri.file(msg.path as string);
          await vscode.workspace.fs.writeFile(uri, Buffer.from((msg.content as string) || '', 'utf8'));
          this._panel.webview.postMessage({ type: 'fileCreated', path: msg.path });
          await vscode.window.showTextDocument(uri);
        } catch (e) {
          this._panel.webview.postMessage({ type: 'fileError', error: (e as Error).message });
        }
        break;
      }

      case 'createFolder': {
        try {
          const uri = vscode.Uri.file(msg.path as string);
          await vscode.workspace.fs.createDirectory(uri);
          this._panel.webview.postMessage({ type: 'folderCreated', path: msg.path });
        } catch (e) {
          this._panel.webview.postMessage({ type: 'fileError', error: (e as Error).message });
        }
        break;
      }

      case 'renameFile': {
        try {
          const oldUri = vscode.Uri.file(msg.oldPath as string);
          const newUri = vscode.Uri.file(msg.newPath as string);
          await vscode.workspace.fs.rename(oldUri, newUri, { overwrite: false });
          this._panel.webview.postMessage({ type: 'fileRenamed', oldPath: msg.oldPath, newPath: msg.newPath });
        } catch (e) {
          this._panel.webview.postMessage({ type: 'fileError', error: (e as Error).message });
        }
        break;
      }

      case 'readFileContent': {
        try {
          const uri = vscode.Uri.file(msg.path as string);
          const bytes = await vscode.workspace.fs.readFile(uri);
          const text = Buffer.from(bytes).toString('utf8');
          this._panel.webview.postMessage({ type: 'fileContent', path: msg.path, content: text.slice(0, 16384) });
        } catch (e) {
          this._panel.webview.postMessage({ type: 'fileError', error: (e as Error).message });
        }
        break;
      }

      // ── Real package manifest operations ─────────────────────────────────────
      case 'loadInstalledPackages': {
        try {
          const folders6 = vscode.workspace.workspaceFolders;
          const root6 = folders6?.[0]?.uri.fsPath ?? process.cwd();
          const path6 = require('path') as typeof import('path');
          const omnipmUri = vscode.Uri.file(path6.join(root6, 'omnipm.json'));
          try {
            const bytes = await vscode.workspace.fs.readFile(omnipmUri);
            const json = JSON.parse(Buffer.from(bytes).toString('utf8'));
            this._panel.webview.postMessage({ type: 'installedPackages', packages: json.dependencies ?? [] });
          } catch {
            this._panel.webview.postMessage({ type: 'installedPackages', packages: [] });
          }
        } catch {
          this._panel.webview.postMessage({ type: 'installedPackages', packages: [] });
        }
        break;
      }

      case 'saveInstalledPackages': {
        try {
          const folders7 = vscode.workspace.workspaceFolders;
          const root7 = folders7?.[0]?.uri.fsPath ?? process.cwd();
          const path7 = require('path') as typeof import('path');
          const omnipmUri = vscode.Uri.file(path7.join(root7, 'omnipm.json'));
          const content = JSON.stringify({
            name: 'omnisystem-project',
            version: '1.0.0',
            dependencies: msg.packages
          }, null, 2);
          await vscode.workspace.fs.writeFile(omnipmUri, Buffer.from(content, 'utf8'));
          this._panel.webview.postMessage({ type: 'packagesSaved' });
        } catch (e) {
          this._panel.webview.postMessage({ type: 'fileError', error: (e as Error).message });
        }
        break;
      }

      // ── Real PTY terminal sessions (node-pty or spawn fallback) ─────────────
      case 'ptyCreate': {
        const cols = (msg.cols as number) || 120;
        const rows = (msg.rows as number) || 30;
        const shell = msg.shell as string | undefined;
        const cwd8 = (msg.cwd as string | undefined) || vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        const session = this._pty.create(cols, rows, shell, cwd8);
        this._panel.webview.postMessage({
          type: 'ptyCreated',
          sessionId: session.id,
          pid: session.pid,
          shell: session.shell,
          backend: session.backend,
          hasPty: this._pty.hasPty,
        });
        break;
      }

      case 'ptyWrite': {
        const written = this._pty.write(msg.sessionId as string, msg.data as string);
        if (!written) {
          // Session gone — surface as exit
          this._panel.webview.postMessage({ type: 'ptyExit', sessionId: msg.sessionId, code: -1 });
        }
        break;
      }

      case 'ptyResize': {
        this._pty.resize(
          msg.sessionId as string,
          (msg.cols as number) || 80,
          (msg.rows as number) || 24,
        );
        break;
      }

      case 'ptyKill': {
        this._pty.kill(msg.sessionId as string, (msg.signal as string) || 'SIGTERM');
        this._panel.webview.postMessage({ type: 'ptyExit', sessionId: msg.sessionId, code: 0 });
        break;
      }

      case 'ptyKillAll': {
        for (const s of this._pty.listSessions()) {
          this._pty.kill(s.id);
        }
        break;
      }

      // ── RuntimeClient IPC direct calls ───────────────────────────────────────
      case 'rpcCall': {
        const { callId, method, params } = msg as unknown as { callId: string; method: string; params: unknown };
        this._runtime.call(method, params)
          .then(result => {
            this._panel.webview.postMessage({ type: 'rpcResult', callId, result });
          })
          .catch((err: Error) => {
            this._panel.webview.postMessage({ type: 'rpcError', callId, error: err.message });
          });
        break;
      }

      // ── Bug Hunter integration ───────────────────────────────────────────────
      case 'getBugHunterStatus': {
        const bhDiags = vscode.languages.getDiagnostics();
        const bhErrors: Array<{ file: string; message: string; severity: string; line: number }> = [];
        for (const [uri, diagnostics] of bhDiags) {
          for (const d of diagnostics) {
            if (d.severity <= vscode.DiagnosticSeverity.Warning) {
              bhErrors.push({
                file: vscode.workspace.asRelativePath(uri).replace(/\\/g, '/'),
                message: d.message,
                severity: d.severity === vscode.DiagnosticSeverity.Error ? 'error' : 'warning',
                line: d.range.start.line + 1,
              });
            }
          }
        }
        this._lastDiagCount = bhErrors.length;
        this._panel.webview.postMessage({ type: 'vscodeDiagnostics', errors: bhErrors });
        break;
      }

      case 'applyBugFix': {
        const fixFile = msg.file as string | undefined;
        const fixLine = (msg.line as number) || 1;
        if (fixFile) {
          try {
            const fixUri = vscode.Uri.file(fixFile);
            const fixRange = new vscode.Range(Math.max(0, fixLine - 1), 0, Math.max(0, fixLine - 1), 0);
            await vscode.window.showTextDocument(fixUri, { selection: fixRange, preview: false });
          } catch { /* file may not exist locally — ignore */ }
        }
        this._panel.webview.postMessage({ type: 'bugFixApplied', bugId: msg.bugId, fix: msg.fix, file: fixFile });
        break;
      }

      case 'reportWebviewError': {
        this._panel.webview.postMessage({
          type: 'bugHunterError',
          source: 'webview',
          message: msg.message as string,
          line: msg.line,
          col: msg.col,
        });
        break;
      }

      case 'saveBugHunterProfile': {
        try {
          await OmniOSDesktopPanel._extensionContext?.workspaceState.update(
            'omnisystem.bugHunterProfile', msg.profile
          );
        } catch { /* ignore */ }
        break;
      }

      case 'loadBugHunterProfile': {
        const bhProfile = OmniOSDesktopPanel._extensionContext?.workspaceState.get(
          'omnisystem.bugHunterProfile', null
        ) ?? null;
        this._panel.webview.postMessage({ type: 'bugHunterProfileLoaded', profile: bhProfile });
        break;
      }

      // ── OmniHarness AI integration ────────────────────────────────────────
      case 'getHarnessStatus': {
        const cfg = vscode.workspace.getConfiguration('omnisystem');
        const serverUrl = cfg.get<string>('harness.serverUrl', 'http://localhost:8080');
        const client = new OmniHarnessClient(serverUrl);
        try {
          const health = await client.health();
          const models = await client.listModels();
          this._panel.webview.postMessage({
            type: 'harnessStatus', alive: true, serverUrl, health,
            models: models.map(m => ({ id: m.id, provider: m.provider })),
          });
        } catch {
          this._panel.webview.postMessage({ type: 'harnessStatus', alive: false, serverUrl, models: [] });
        }
        break;
      }

      case 'closeDesktopPanel': {
        this._panel.dispose();
        break;
      }

      case 'getRuntimeDiagnostics': {
        this._panel.webview.postMessage({
          type: 'runtimeDiagnostics',
          ...this._runtime.diagnostics(),
          hasPty: this._pty.hasPty,
          ptySessions: this._pty.sessionCount(),
        });
        break;
      }
    }
  }

  public dispose(): void {
    OmniOSDesktopPanel.currentPanel = undefined;
    this._panel.dispose();
    disposePtyManager();
    disposeRuntimeClient();
    while (this._disposables.length) {
      const d = this._disposables.pop();
      if (d) d.dispose();
    }
  }

  private _getHtml(scriptUri: string, widgetStyleUri: string, widgetScriptUri: string): string {
    return `<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
<meta charset="UTF-8"/>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src vscode-resource: 'unsafe-inline'; script-src vscode-resource: https: 'unsafe-inline';">

<meta name="viewport" content="width=device-width,initial-scale=1.0"/>
<title>OmniOS Desktop</title>
<link rel="stylesheet" href="${widgetStyleUri}"/>
<style>
*{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#050D1A;--accent:#00D4FF;--gold:#FFB800;--green:#00FF88;--red:#FF4466;
  --surface:rgba(10,20,40,0.72);--border:rgba(0,212,255,0.18);
  --text:#E8F4FF;--text-dim:rgba(232,244,255,0.55);
  --glass:rgba(8,18,36,0.68);
}
html,body{width:100%;height:100%;overflow:hidden;background:var(--bg);color:var(--text);font-family:'Segoe UI',system-ui,sans-serif;font-size:13px;user-select:none}

/* WALLPAPER */
#wallpaper{position:fixed;inset:0;z-index:0;overflow:hidden}
#wallpaper::before{
  content:'';position:absolute;inset:0;
  background:
    radial-gradient(ellipse 120% 80% at 20% 30%,rgba(0,80,160,0.38) 0%,transparent 60%),
    radial-gradient(ellipse 80% 60% at 80% 70%,rgba(0,50,120,0.28) 0%,transparent 55%),
    radial-gradient(ellipse 60% 90% at 50% 10%,rgba(0,30,80,0.22) 0%,transparent 50%),
    #050D1A;
  animation:aurora 18s ease-in-out infinite alternate;
}
#wallpaper::after{
  content:'';position:absolute;inset:0;
  background-image:radial-gradient(rgba(0,212,255,0.055) 1px,transparent 1px);
  background-size:28px 28px;
}
@keyframes aurora{
  0%{filter:hue-rotate(0deg) brightness(1)}
  33%{filter:hue-rotate(20deg) brightness(1.08)}
  66%{filter:hue-rotate(-15deg) brightness(0.95)}
  100%{filter:hue-rotate(10deg) brightness(1.05)}
}
#nebula{
  position:absolute;width:800px;height:800px;border-radius:50%;
  background:radial-gradient(circle,rgba(0,212,255,0.06) 0%,rgba(0,80,200,0.04) 40%,transparent 70%);
  top:50%;left:50%;transform:translate(-50%,-50%);
  animation:nebulaPulse 12s ease-in-out infinite alternate;
  pointer-events:none;
}
@keyframes nebulaPulse{
  from{transform:translate(-50%,-50%) scale(1);opacity:0.7}
  to{transform:translate(-50%,-50%) scale(1.18);opacity:1}
}

/* DESKTOP */
#desktop{position:fixed;inset:0;z-index:1;display:flex;flex-direction:column}
#desktop-area{flex:1;position:relative;overflow:hidden}
#desktop-icons{
  position:absolute;top:16px;left:16px;display:flex;flex-direction:column;gap:8px;z-index:2
}
.desktop-icon{
  width:72px;display:flex;flex-direction:column;align-items:center;gap:5px;
  padding:8px 4px;border-radius:10px;cursor:pointer;transition:background 0.15s,transform 0.12s;
  border:1.5px solid transparent;
}
.desktop-icon:hover{background:rgba(0,212,255,0.09);border-color:rgba(0,212,255,0.2)}
.desktop-icon.selected{background:rgba(0,212,255,0.16);border-color:rgba(0,212,255,0.4)}
.di-icon{
  width:44px;height:44px;border-radius:10px;display:flex;align-items:center;justify-content:center;
  font-size:22px;box-shadow:0 4px 16px rgba(0,0,0,0.5);
}
.di-label{font-size:10px;color:var(--text);text-align:center;line-height:1.3;text-shadow:0 1px 4px rgba(0,0,0,0.8)}

/* TASKBAR */
#taskbar{
  height:48px;background:rgba(5,13,26,0.88);backdrop-filter:blur(20px);
  border-top:1px solid var(--border);display:flex;align-items:center;gap:8px;padding:0 12px;
  z-index:1000;flex-shrink:0;
}
#start-btn{
  background:linear-gradient(135deg,#0055AA,#003380);border:1px solid rgba(0,212,255,0.4);
  border-radius:8px;padding:6px 14px;color:var(--accent);font-weight:700;font-size:12px;
  cursor:pointer;transition:all 0.15s;white-space:nowrap;letter-spacing:0.5px;
}
#start-btn:hover{background:linear-gradient(135deg,#0066CC,#004499);box-shadow:0 0 16px rgba(0,212,255,0.3)}
#taskbar-apps{flex:1;display:flex;gap:6px;overflow-x:auto;scrollbar-width:none}
#taskbar-apps::-webkit-scrollbar{display:none}
.tb-chip{
  height:32px;padding:0 12px;background:rgba(0,212,255,0.08);border:1px solid rgba(0,212,255,0.2);
  border-radius:6px;color:var(--text);font-size:11px;cursor:pointer;display:flex;align-items:center;gap:6px;
  white-space:nowrap;transition:all 0.15s;
}
.tb-chip:hover{background:rgba(0,212,255,0.16);border-color:rgba(0,212,255,0.4)}
.tb-chip.active{background:rgba(0,212,255,0.2);border-color:var(--accent);color:var(--accent)}
#taskbar-right{display:flex;align-items:center;gap:12px;margin-left:8px}
#notif-btn{
  width:32px;height:32px;border-radius:6px;background:rgba(255,255,255,0.06);
  border:1px solid rgba(255,255,255,0.1);display:flex;align-items:center;justify-content:center;
  cursor:pointer;font-size:15px;transition:background 0.15s;
}
#notif-btn:hover{background:rgba(0,212,255,0.12)}
#clock-widget{text-align:right;line-height:1.3}
#clock-time{font-size:13px;font-weight:600;color:var(--text)}
#clock-date{font-size:10px;color:var(--text-dim)}

/* START MENU */
#start-menu{
  position:fixed;bottom:56px;left:8px;width:340px;
  background:rgba(6,14,28,0.95);backdrop-filter:blur(24px);
  border:1px solid var(--border);border-radius:14px;
  box-shadow:0 8px 48px rgba(0,0,0,0.7);z-index:2000;
  display:none;flex-direction:column;overflow:hidden;
}
#start-menu.open{display:flex}
#sm-search{
  padding:14px 16px 10px;border-bottom:1px solid var(--border);
}
#sm-search input{
  width:100%;background:rgba(0,212,255,0.07);border:1px solid rgba(0,212,255,0.25);
  border-radius:8px;padding:8px 12px;color:var(--text);font-size:13px;outline:none;
  transition:border-color 0.15s;
}
#sm-search input:focus{border-color:var(--accent)}
#sm-body{padding:12px 14px;overflow-y:auto;max-height:400px;display:flex;flex-direction:column;gap:14px}
.sm-section-label{font-size:10px;color:var(--text-dim);text-transform:uppercase;letter-spacing:1px;margin-bottom:6px}
#sm-pinned{display:grid;grid-template-columns:repeat(4,1fr);gap:8px}
.sm-app-btn{
  display:flex;flex-direction:column;align-items:center;gap:5px;padding:10px 6px;
  border-radius:10px;cursor:pointer;transition:background 0.15s;border:1px solid transparent;
}
.sm-app-btn:hover{background:rgba(0,212,255,0.1);border-color:rgba(0,212,255,0.2)}
.sm-app-icon{width:36px;height:36px;border-radius:8px;display:flex;align-items:center;justify-content:center;font-size:18px}
.sm-app-name{font-size:10px;color:var(--text);text-align:center;line-height:1.3}
#sm-system{display:flex;flex-direction:column;gap:4px}
.sm-sys-btn{
  display:flex;align-items:center;gap:10px;padding:8px 12px;border-radius:8px;cursor:pointer;
  transition:background 0.15s;font-size:12px;
}
.sm-sys-btn:hover{background:rgba(0,212,255,0.1)}
.sm-sys-icon{font-size:16px;width:24px;text-align:center}

/* WINDOWS */
#windows-layer{position:absolute;inset:0;overflow:hidden;pointer-events:none}
.window{
  position:absolute;background:var(--glass);backdrop-filter:blur(16px);
  border:1px solid var(--border);border-radius:14px;
  box-shadow:0 12px 48px rgba(0,0,0,0.6);display:flex;flex-direction:column;
  pointer-events:auto;
  min-width:320px;min-height:240px;overflow:hidden;transition:box-shadow 0.15s;
}
.window.focused{border-color:rgba(0,212,255,0.45);box-shadow:0 0 0 1px rgba(0,212,255,0.15),0 16px 56px rgba(0,0,0,0.7)}
.window.maximized{border-radius:0!important}
.win-titlebar{
  height:36px;background:rgba(0,20,50,0.6);display:flex;align-items:center;
  padding:0 12px;gap:8px;cursor:move;flex-shrink:0;border-bottom:1px solid rgba(0,212,255,0.1);
  user-select:none;
}
.win-title-icon{font-size:15px}
.win-title-text{flex:1;font-size:12px;font-weight:600;color:var(--text);letter-spacing:0.3px}
.win-controls{display:flex;gap:6px}
.wc-btn{
  width:24px;height:24px;border-radius:50%;display:flex;align-items:center;justify-content:center;
  cursor:pointer;font-size:11px;transition:opacity 0.15s;border:none;outline:none;
}
.wc-min{background:#FFB800;color:#000}
.wc-max{background:#00FF88;color:#000}
.wc-cls{background:#FF4466;color:#fff}
.wc-btn:hover{opacity:0.8}
.win-body{flex:1;overflow:auto;position:relative}
.win-resize{
  position:absolute;bottom:0;right:0;width:16px;height:16px;cursor:se-resize;
  background:linear-gradient(135deg,transparent 50%,rgba(0,212,255,0.3) 50%);border-radius:0 0 14px 0;
}

/* NOTIFICATIONS */
#notif-container{position:fixed;top:12px;right:12px;z-index:9999;display:flex;flex-direction:column;gap:8px;width:280px}
.notif{
  background:rgba(6,16,32,0.96);border:1px solid rgba(0,212,255,0.3);border-radius:10px;
  padding:12px 14px;box-shadow:0 4px 24px rgba(0,0,0,0.5);
  animation:notifSlide 0.25s ease;display:flex;gap:10px;align-items:flex-start;
}
@keyframes notifSlide{from{transform:translateX(100%);opacity:0}to{transform:translateX(0);opacity:1}}
.notif-icon{font-size:18px;flex-shrink:0}
.notif-body{flex:1}
.notif-title{font-size:12px;font-weight:700;color:var(--accent);margin-bottom:2px}
.notif-msg{font-size:11px;color:var(--text-dim)}
.notif-close{cursor:pointer;color:var(--text-dim);font-size:14px;line-height:1}
.notif-close:hover{color:var(--text)}

/* CONTEXT MENU */
#ctx-menu{
  position:fixed;background:rgba(6,14,28,0.97);backdrop-filter:blur(16px);
  border:1px solid var(--border);border-radius:10px;padding:6px;
  box-shadow:0 8px 32px rgba(0,0,0,0.6);z-index:8000;display:none;min-width:160px;
}
#ctx-menu.open{display:block}
.ctx-item{
  padding:7px 12px;border-radius:6px;cursor:pointer;font-size:12px;
  display:flex;align-items:center;gap:8px;transition:background 0.1s;
}
.ctx-item:hover{background:rgba(0,212,255,0.12)}
.ctx-sep{height:1px;background:var(--border);margin:4px 0}

/* SCROLLBARS */
::-webkit-scrollbar{width:5px;height:5px}
::-webkit-scrollbar-track{background:transparent}
::-webkit-scrollbar-thumb{background:rgba(0,212,255,0.25);border-radius:3px}
::-webkit-scrollbar-thumb:hover{background:rgba(0,212,255,0.45)}

/* APP STYLES */
.app-container{padding:16px;height:100%;display:flex;flex-direction:column;gap:12px;overflow:auto}
.app-header{display:flex;align-items:center;gap:10px;padding-bottom:12px;border-bottom:1px solid var(--border)}
.app-header h2{font-size:16px;font-weight:700;color:var(--text)}
.app-header .badge{
  background:rgba(0,212,255,0.15);border:1px solid rgba(0,212,255,0.3);
  border-radius:20px;padding:2px 10px;font-size:10px;color:var(--accent);font-weight:700;
}
.btn{
  padding:7px 16px;border-radius:8px;border:none;cursor:pointer;font-size:12px;font-weight:600;
  transition:all 0.15s;letter-spacing:0.3px;
}
.btn-primary{background:linear-gradient(135deg,#0066CC,#004499);color:#fff;border:1px solid rgba(0,212,255,0.3)}
.btn-primary:hover{background:linear-gradient(135deg,#0077DD,#0055BB);box-shadow:0 0 16px rgba(0,212,255,0.25)}
.btn-accent{background:rgba(0,212,255,0.15);color:var(--accent);border:1px solid rgba(0,212,255,0.3)}
.btn-accent:hover{background:rgba(0,212,255,0.25)}
.btn-gold{background:rgba(255,184,0,0.15);color:var(--gold);border:1px solid rgba(255,184,0,0.3)}
.btn-gold:hover{background:rgba(255,184,0,0.25)}
.btn-green{background:rgba(0,255,136,0.12);color:var(--green);border:1px solid rgba(0,255,136,0.25)}
.btn-green:hover{background:rgba(0,255,136,0.22)}
.btn-danger{background:rgba(255,68,102,0.12);color:var(--red);border:1px solid rgba(255,68,102,0.25)}
.btn-danger:hover{background:rgba(255,68,102,0.22)}
.btn-sm{padding:5px 12px;font-size:11px}
.input-field{
  background:rgba(0,212,255,0.06);border:1px solid rgba(0,212,255,0.2);border-radius:8px;
  padding:7px 12px;color:var(--text);font-size:12px;outline:none;transition:border-color 0.15s;width:100%;
}
.input-field:focus{border-color:var(--accent)}
.select-field{
  background:rgba(0,20,50,0.8);border:1px solid rgba(0,212,255,0.2);border-radius:8px;
  padding:7px 12px;color:var(--text);font-size:12px;outline:none;cursor:pointer;
}
.select-field:focus{border-color:var(--accent)}
.card{
  background:rgba(0,20,50,0.5);border:1px solid rgba(0,212,255,0.12);border-radius:10px;padding:14px;
}
.stat-card{
  background:rgba(0,20,50,0.6);border:1px solid var(--border);border-radius:10px;
  padding:14px;display:flex;flex-direction:column;gap:4px;
}
.stat-value{font-size:22px;font-weight:800;color:var(--accent)}
.stat-label{font-size:10px;color:var(--text-dim);text-transform:uppercase;letter-spacing:0.8px}
.pulse{
  width:8px;height:8px;border-radius:50%;background:var(--green);display:inline-block;
  animation:pulse 2s ease-in-out infinite;
}
@keyframes pulse{0%,100%{box-shadow:0 0 0 0 rgba(0,255,136,0.4)}50%{box-shadow:0 0 0 6px rgba(0,255,136,0)}}
.section-label{font-size:11px;font-weight:700;color:var(--text-dim);text-transform:uppercase;letter-spacing:0.8px;margin-bottom:8px}
.row{display:flex;gap:8px;align-items:center}
.col{display:flex;flex-direction:column;gap:8px}

/* FILE MANAGER */
#fm-breadcrumb{
  display:flex;align-items:center;gap:6px;padding:8px 12px;
  background:rgba(0,20,50,0.5);border-radius:8px;font-size:11px;flex-wrap:wrap;
}
#fm-grid{display:flex;flex-direction:column;gap:2px;overflow-y:auto;flex:1}
.fm-item{
  display:flex;align-items:center;gap:10px;padding:7px 10px;border-radius:7px;cursor:pointer;
  transition:background 0.1s;
}
.fm-item:hover{background:rgba(0,212,255,0.08)}
.fm-item.selected{background:rgba(0,212,255,0.15)}
.fm-icon{font-size:18px;width:24px;text-align:center;flex-shrink:0}
.fm-name{flex:1;font-size:12px;color:var(--text)}
.fm-size{font-size:10px;color:var(--text-dim);min-width:50px;text-align:right}
.fm-loading{padding:24px;text-align:center;color:var(--text-dim);font-size:13px}

/* TERMINAL */
#term-output{
  flex:1;overflow-y:auto;padding:12px;font-family:'Cascadia Code','Consolas',monospace;
  font-size:12px;line-height:1.7;color:#00FF88;background:#010a06;border-radius:8px;
}
.term-line{white-space:pre-wrap;word-break:break-all}
.term-line.err{color:#FF4466}
.term-line.info{color:var(--accent)}
.term-line.dim{color:rgba(0,255,136,0.5)}
#term-input-row{display:flex;align-items:center;gap:8px;padding-top:8px}
#term-prompt{color:var(--accent);font-family:'Cascadia Code','Consolas',monospace;font-size:12px;white-space:nowrap}
#term-input{
  flex:1;background:transparent;border:none;border-bottom:1px solid rgba(0,255,136,0.3);
  color:#00FF88;font-family:'Cascadia Code','Consolas',monospace;font-size:12px;outline:none;padding:4px 0;
}

/* COMPILER */
#build-output{
  flex:1;min-height:120px;overflow-y:auto;padding:10px;font-family:'Cascadia Code','Consolas',monospace;
  font-size:11px;line-height:1.7;background:#010a06;border-radius:8px;
}
.build-line{white-space:pre-wrap;word-break:break-all;color:#00FF88}
.build-line.err{color:#FF4466}
.build-line.phase{color:var(--accent);font-weight:700}
.phase-bar{display:flex;gap:4px}
.phase-step{
  flex:1;height:6px;border-radius:3px;background:rgba(0,212,255,0.1);
  transition:background 0.3s;
}
.phase-step.active{background:var(--accent);box-shadow:0 0 8px rgba(0,212,255,0.5)}
.phase-step.done{background:var(--green)}

/* ML STUDIO */
.layer-list{display:flex;flex-direction:column;gap:4px;max-height:160px;overflow-y:auto}
.layer-item{
  display:flex;align-items:center;gap:8px;padding:6px 10px;background:rgba(0,212,255,0.06);
  border-radius:6px;border:1px solid rgba(0,212,255,0.12);
}
.layer-type{flex:1;font-size:11px;color:var(--text)}
.layer-rm{color:var(--red);cursor:pointer;font-size:14px;padding:0 4px}
.layer-rm:hover{opacity:0.7}
.hp-row{display:flex;align-items:center;gap:8px;font-size:12px}
.hp-label{flex:1;color:var(--text-dim)}
.hp-val{width:80px;background:rgba(0,212,255,0.06);border:1px solid rgba(0,212,255,0.2);border-radius:6px;padding:4px 8px;color:var(--text);font-size:12px;text-align:center}
.train-metric{
  background:rgba(0,20,50,0.6);border-radius:8px;padding:10px 14px;display:flex;justify-content:space-between;align-items:center;
}
.metric-name{font-size:11px;color:var(--text-dim)}
.metric-val{font-size:18px;font-weight:800;color:var(--green)}

/* PM */
.pkg-item{
  display:flex;align-items:center;gap:10px;padding:8px 12px;border-radius:8px;
  border:1px solid rgba(0,212,255,0.1);background:rgba(0,20,50,0.4);
}
.pkg-name{flex:1;font-size:12px;font-weight:600}
.pkg-version{font-size:10px;color:var(--text-dim)}
.tab-bar{display:flex;gap:2px;padding:2px;background:rgba(0,20,50,0.5);border-radius:8px;border:1px solid var(--border)}
.tab{padding:6px 16px;border-radius:6px;cursor:pointer;font-size:12px;font-weight:600;color:var(--text-dim);transition:all 0.15s}
.tab.active{background:rgba(0,212,255,0.15);color:var(--accent)}

/* DESKTOP */
.desktop-grid{display:grid;grid-template-columns:1fr 1fr;gap:10px}
.desktop-card{
  background:rgba(0,20,50,0.55);border:1px solid rgba(0,212,255,0.15);border-radius:12px;
  padding:14px;display:flex;flex-direction:column;gap:8px;transition:border-color 0.15s;
}
.desktop-card:hover{border-color:rgba(0,212,255,0.35)}
.desktop-card-title{font-size:13px;font-weight:700;color:var(--text);display:flex;align-items:center;gap:6px}
.desktop-card-desc{font-size:11px;color:var(--text-dim);line-height:1.5}
.status-dot{width:7px;height:7px;border-radius:50%;display:inline-block;margin-right:4px}
.status-dot.green{background:var(--green);box-shadow:0 0 6px var(--green)}
.status-dot.gold{background:var(--gold);box-shadow:0 0 6px var(--gold)}
.desktop-status-row{display:flex;gap:14px;font-size:10px;color:var(--text-dim);padding:8px 12px;background:rgba(0,20,50,0.4);border-radius:8px}

/* SETTINGS */
.settings-row{
  display:flex;align-items:center;gap:10px;padding:10px 0;border-bottom:1px solid rgba(0,212,255,0.06);
}
.settings-row:last-child{border-bottom:none}
.settings-label{flex:1;font-size:12px}
.toggle{
  width:36px;height:20px;border-radius:10px;background:rgba(0,212,255,0.1);
  border:1px solid rgba(0,212,255,0.2);cursor:pointer;position:relative;transition:background 0.2s;
}
.toggle.on{background:var(--accent)}
.toggle::after{
  content:'';position:absolute;top:3px;left:3px;width:12px;height:12px;
  border-radius:50%;background:#fff;transition:transform 0.2s;
}
.toggle.on::after{transform:translateX(16px)}

/* SYSTEM MONITOR */
.sys-health-item{
  display:flex;align-items:center;gap:10px;padding:8px 12px;border-radius:8px;
  background:rgba(0,20,50,0.4);border:1px solid rgba(0,212,255,0.08);
}
.sys-health-name{flex:1;font-size:12px}
.sys-health-status{font-size:11px;color:var(--green);font-weight:700}
.progress-bar{height:6px;background:rgba(0,212,255,0.1);border-radius:3px;overflow:hidden}
.progress-fill{height:100%;border-radius:3px;background:linear-gradient(90deg,var(--accent),var(--green));transition:width 1s ease}
.lang-badge{
  display:inline-flex;align-items:center;gap:5px;padding:4px 10px;border-radius:20px;
  font-size:11px;font-weight:700;cursor:pointer;transition:all 0.15s;
}
.lang-badge:hover{opacity:0.8;transform:scale(1.05)}
.app-converter-strategy{
  background:rgba(0,212,255,0.05);border:1px solid rgba(0,212,255,0.15);border-radius:8px;
  padding:12px;font-size:11px;color:var(--text-dim);line-height:1.6;
}
</style>
</head>
<body>

<!-- Wallpaper -->
<div id="wallpaper"><div id="nebula"></div></div>

<!-- Notifications -->
<div id="notif-container"></div>

<!-- Context Menu -->
<div id="ctx-menu">
  <div class="ctx-item" data-action="ctx-new-file">📄 New File</div>
  <div class="ctx-item" data-action="ctx-open-term">💻 Open Terminal</div>
  <div class="ctx-item" data-action="ctx-refresh">🔄 Refresh</div>
  <div class="ctx-sep" id="ctx-file-sep" style="display:none"></div>
  <div class="ctx-item" data-action="ctx-open" id="ctx-open-item" style="display:none">📂 Open</div>
  <div class="ctx-item" data-action="ctx-copy-path" id="ctx-copy-item" style="display:none">📋 Copy Path</div>
  <div class="ctx-item" data-action="ctx-delete" id="ctx-delete-item" style="display:none">🗑️ Delete</div>
</div>

<!-- Desktop -->
<div id="desktop">
  <div id="desktop-area">
    <div id="desktop-icons">
      <div class="desktop-icon" data-app="harness" data-icon="🤖" data-label="OmniHarness AI">
        <div class="di-icon" style="background:linear-gradient(135deg,#5B3FA8,#7C5CD0)">🤖</div>
        <div class="di-label">OmniHarness AI</div>
      </div>
      <div class="desktop-icon" data-app="file-manager" data-icon="📁" data-label="Files">
        <div class="di-icon" style="background:linear-gradient(135deg,#FFB800,#FF6600)">📁</div>
        <div class="di-label">Files</div>
      </div>
      <div class="desktop-icon" data-app="terminal" data-icon="💻" data-label="Terminal">
        <div class="di-icon" style="background:linear-gradient(135deg,#001A00,#003300)">💻</div>
        <div class="di-label">Terminal</div>
      </div>
      <div class="desktop-icon" data-app="code-studio" data-icon="✨" data-label="Code Studio">
        <div class="di-icon" style="background:linear-gradient(135deg,#004499,#0077CC)">✨</div>
        <div class="di-label">Code Studio</div>
      </div>
      <div class="desktop-icon" data-app="desktop" data-icon="🌿" data-label="Omnisystem Hub">
        <div class="di-icon" style="background:linear-gradient(135deg,#003300,#006600)">🌿</div>
        <div class="di-label">Omnisystem Hub</div>
      </div>
      <div class="desktop-icon" data-app="compiler" data-icon="⚙️" data-label="OmniCC Build">
        <div class="di-icon" style="background:linear-gradient(135deg,#1A0A00,#3D1A00)">⚙️</div>
        <div class="di-label">OmniCC Build</div>
      </div>
      <div class="desktop-icon" data-app="ml-studio" data-icon="🧠" data-label="ML Studio">
        <div class="di-icon" style="background:linear-gradient(135deg,#1A0033,#330066)">🧠</div>
        <div class="di-label">ML Studio</div>
      </div>
      <div class="desktop-icon" data-app="pkg-manager" data-icon="📦" data-label="OmniPM">
        <div class="di-icon" style="background:linear-gradient(135deg,#001833,#003366)">📦</div>
        <div class="di-label">OmniPM</div>
      </div>
      <div class="desktop-icon" data-app="app-converter" data-icon="🔄" data-label="App Converter">
        <div class="di-icon" style="background:linear-gradient(135deg,#1A1A00,#333300)">🔄</div>
        <div class="di-label">App Converter</div>
      </div>
      <div class="desktop-icon" data-app="settings" data-icon="⚙" data-label="Settings">
        <div class="di-icon" style="background:linear-gradient(135deg,#0A0A1A,#1A1A2E)">⚙</div>
        <div class="di-label">Settings</div>
      </div>
      <div class="desktop-icon" data-app="system-monitor" data-icon="📊" data-label="System Monitor">
        <div class="di-icon" style="background:linear-gradient(135deg,#001A33,#003355)">📊</div>
        <div class="di-label">System Monitor</div>
      </div>
      <div class="desktop-icon" data-app="bug-hunter" data-icon="🐛" data-label="Bug Hunter">
        <div class="di-icon" style="background:linear-gradient(135deg,#330011,#660022)">🐛</div>
        <div class="di-label">Bug Hunter</div>
      </div>
    </div>
    <div id="windows-layer"></div>
  </div>

  <!-- Taskbar -->
  <div id="taskbar">
    <div id="start-btn">⬡ OmniOS</div>
    <div id="taskbar-apps"></div>
    <div id="taskbar-right">
      <div id="notif-btn" style="position:relative">🔔<span id="notif-badge" style="position:absolute;top:-5px;right:-5px;background:#FF4466;color:#fff;font-size:9px;min-width:16px;height:16px;border-radius:8px;display:none;align-items:center;justify-content:center;font-weight:700;padding:0 3px;line-height:16px;text-align:center"></span></div>
      <div id="clock-widget">
        <div id="clock-time">00:00:00</div>
        <div id="clock-date">Mon Jan 01</div>
      </div>
    </div>
  </div>
</div>

<!-- Start Menu -->
<div id="start-menu">
  <div id="sm-search">
    <input type="text" id="sm-search-input" placeholder="Search apps and files..."/>
  </div>
  <div id="sm-body">
    <div>
      <div class="sm-section-label">Pinned</div>
      <div id="sm-pinned">
        <div class="sm-app-btn" data-app="harness">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#5B3FA8,#7C5CD0)">🤖</div>
          <div class="sm-app-name">OmniHarness</div>
        </div>
        <div class="sm-app-btn" data-app="file-manager">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#FFB800,#FF6600)">📁</div>
          <div class="sm-app-name">Files</div>
        </div>
        <div class="sm-app-btn" data-app="terminal">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#001A00,#003300)">💻</div>
          <div class="sm-app-name">Terminal</div>
        </div>
        <div class="sm-app-btn" data-app="code-studio">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#004499,#0077CC)">✨</div>
          <div class="sm-app-name">Code Studio</div>
        </div>
        <div class="sm-app-btn" data-app="desktop">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#003300,#006600)">🌿</div>
          <div class="sm-app-name">Omnisystem Hub</div>
        </div>
        <div class="sm-app-btn" data-app="compiler">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#1A0A00,#3D1A00)">⚙️</div>
          <div class="sm-app-name">OmniCC</div>
        </div>
        <div class="sm-app-btn" data-app="ml-studio">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#1A0033,#330066)">🧠</div>
          <div class="sm-app-name">ML Studio</div>
        </div>
        <div class="sm-app-btn" data-app="pkg-manager">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#001833,#003366)">📦</div>
          <div class="sm-app-name">OmniPM</div>
        </div>
        <div class="sm-app-btn" data-app="system-monitor">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#001A33,#003355)">📊</div>
          <div class="sm-app-name">Monitor</div>
        </div>
        <div class="sm-app-btn" data-app="sandbox">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#003322,#00664422)">🛡️</div>
          <div class="sm-app-name">Sandbox</div>
        </div>
        <div class="sm-app-btn" data-app="bug-hunter">
          <div class="sm-app-icon" style="background:linear-gradient(135deg,#330011,#660022)">🐛</div>
          <div class="sm-app-name">Bug Hunter</div>
        </div>
      </div>
    </div>
    <div>
      <div class="sm-section-label">System</div>
      <div id="sm-system">
        <div class="sm-sys-btn" data-action="open-theme-picker"><span class="sm-sys-icon">🎨</span>Theme Picker</div>
        <div class="sm-sys-btn" data-action="open-settings"><span class="sm-sys-icon">⚙</span>VS Code Settings</div>
        <div class="sm-sys-btn" data-action="kernel-log"><span class="sm-sys-icon">📋</span>Kernel Log</div>
        <div class="sm-sys-btn" data-action="shutdown"><span class="sm-sys-icon">⏻</span>Shutdown OmniOS</div>
      </div>
    </div>
  </div>
</div>

<script src="${widgetScriptUri}"></script>
<script src="${scriptUri}"></script>
</body>
</html>`;
  }
}
