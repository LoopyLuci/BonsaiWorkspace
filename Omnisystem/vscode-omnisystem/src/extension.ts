import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { spawn } from 'child_process';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    State,
} from 'vscode-languageclient/node';
import { OmnisystemExplorerProvider } from './providers/OmnisystemExplorer';
import { DesktopExplorerProvider } from './providers/DesktopExplorer';
import { OmniPMExplorerProvider } from './providers/OmniPMExplorer';
import { OmniOSExplorerProvider } from './providers/OmniOSExplorer';
import { OmnisystemTaskProvider } from './providers/TaskProvider';
import { OmnisystemDashboardPanel } from './webviews/OmnisystemDashboard';
import { BuildDashboardPanel } from './webviews/BuildDashboard';
import { MlStudioPanel } from './webviews/MlStudio';
import { ShaderPreviewPanel } from './webviews/ShaderPreview';
import { LayoutPreviewPanel } from './webviews/LayoutPreview';
import { OmniOSDesktopPanel } from './webviews/OmniOSDesktop';
import { WidgetGalleryPanel } from './webviews/WidgetGallery';
import { WelcomePanel } from './webviews/WelcomePanel';
import { WidgetConverterPanel } from './webviews/WidgetConverter';
import { OmniCCDashboardPanel } from './webviews/OmniCCDashboard';
import { BuildEditorProvider } from './editors/BuildEditor';
import { SylvaNotebookSerializer, SylvaNotebookKernel } from './editors/SylvaNotebook';
import { OmniHarnessViewProvider } from './harness/OmniHarnessViewProvider';
import { HarnessStore } from './harness/HarnessStore';

// ─── Constants ────────────────────────────────────────────────────────────────

const EXTENSION_ID = 'omnisystem';
const LSP_CLIENT_ID = 'omnisystem-lsp';
const LSP_CLIENT_NAME = 'Omnisystem Language Server';
const FIRST_INSTALL_KEY = 'omnisystem.firstInstall';

const OMNI_LANGUAGES = [
    'titan', 'vera', 'helix', 'aether', 'axiom', 'sylva', 'nexus',
] as const;
type OmniLanguage = typeof OMNI_LANGUAGES[number];

const OMNI_DOC_SELECTORS: vscode.DocumentSelector = OMNI_LANGUAGES.map((lang) => ({
    scheme: 'file',
    language: lang,
}));

// ─── Workspace type detection ─────────────────────────────────────────────────

type WorkspaceKind = 'omnisystem' | 'desktop' | 'omnios' | 'titan' | 'unknown';

interface WorkspaceInfo {
    kind: WorkspaceKind;
    root: vscode.Uri | undefined;
    hasDesktop: boolean;
    hasOmniOS: boolean;
    desktopRoot: vscode.Uri | undefined;
}

// App manifest filename: primary (de-branded) name first, then legacy fallback.
const APP_MANIFEST_NAMES = ['app.omnisystem.toml', 'app.bonsai.toml'] as const;

// ─── Module-level state ───────────────────────────────────────────────────────

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let statusBarMain: vscode.StatusBarItem;
let statusBarLsp: vscode.StatusBarItem;
let statusBarPlatform: vscode.StatusBarItem;
let statusBarBuild: vscode.StatusBarItem;
let extensionContext: vscode.ExtensionContext;

let explorerProvider: OmnisystemExplorerProvider;
let desktopProvider: DesktopExplorerProvider;
let omnipmProvider: OmniPMExplorerProvider;
let omniosProvider: OmniOSExplorerProvider;
let harnessProvider: OmniHarnessViewProvider;

// ─── Helpers ──────────────────────────────────────────────────────────────────

function omniccPath(): string {
    return resolvedOmniccPath();
}

function workspaceRoot(): string | undefined {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

// ─── Activation ───────────────────────────────────────────────────────────────

export async function activate(ctx: vscode.ExtensionContext): Promise<void> {
    extensionContext = ctx;

    outputChannel = vscode.window.createOutputChannel('Omnisystem');
    ctx.subscriptions.push(outputChannel);
    outputChannel.appendLine('Omnisystem extension activating...');
    outputChannel.show(false);

    // Instantiate real providers immediately (constructors are sync and safe)
    explorerProvider = new OmnisystemExplorerProvider();
    desktopProvider   = new DesktopExplorerProvider(undefined);
    omnipmProvider   = new OmniPMExplorerProvider();
    omniosProvider   = new OmniOSExplorerProvider(undefined);

    // Register once — VS Code only accepts the first registration per view ID
    ctx.subscriptions.push(
        vscode.window.registerTreeDataProvider('omnisystemExplorer', explorerProvider),
        vscode.window.registerTreeDataProvider('desktopExplorer',     desktopProvider),
        vscode.window.registerTreeDataProvider('omnipmExplorer',     omnipmProvider),
        vscode.window.registerTreeDataProvider('omniOsExplorer',     omniosProvider),
    );

    // OmniHarness AI panel — registered synchronously and FIRST among the risky
    // pieces, so the webview resolves even if later async activation fails.
    // (Previously this lived at the end of activateCore(); any earlier throw
    // left the view showing "no data provider registered".)
    try {
        const harnessStore = new HarnessStore(ctx);
        harnessProvider = new OmniHarnessViewProvider(ctx, harnessStore, outputChannel);
        ctx.subscriptions.push(
            vscode.window.registerWebviewViewProvider(
                OmniHarnessViewProvider.viewType,
                harnessProvider,
                { webviewOptions: { retainContextWhenHidden: true } }
            ),
            harnessProvider
        );
        outputChannel.appendLine('OmniHarness panel registered.');
    } catch (err) {
        outputChannel.appendLine(`OmniHarness registration error: ${err}`);
    }

    outputChannel.appendLine('Providers registered. Starting async activation...');

    // Async work (workspace detection, LSP, commands) — providers already live
    try {
        await activateCore(ctx);
        outputChannel.appendLine('Core activation complete.');
    } catch (err) {
        outputChannel.appendLine(`ACTIVATION ERROR: ${err}`);
        outputChannel.show(true);
        vscode.window.showErrorMessage(`Omnisystem extension failed to activate: ${err}`);
    }
}

async function activateCore(ctx: vscode.ExtensionContext): Promise<void> {

    const wsInfo = await detectWorkspace();
    outputChannel.appendLine(`Workspace kind: ${wsInfo.kind}`);
    if (wsInfo.hasDesktop) {
        outputChannel.appendLine(`Desktop root: ${wsInfo.desktopRoot?.fsPath ?? 'unknown'}`);
    }
    if (wsInfo.hasOmniOS) {
        outputChannel.appendLine('OmniOS detected in workspace.');
    }

    // Auto-apply workspace settings when BUILD.omnisystem present
    if (wsInfo.kind === 'omnisystem') {
        applyOmnisystemWorkspaceSettings().catch((err) =>
            outputChannel.appendLine(`Workspace settings error (non-fatal): ${err}`)
        );
    }

    registerStatusBar(ctx);

    // Update providers with resolved workspace root (they were pre-registered in activate())
    desktopProvider.refresh(wsInfo.desktopRoot);
    omniosProvider.refresh(wsInfo.root);

    // Task provider
    ctx.subscriptions.push(
        vscode.tasks.registerTaskProvider('omnisystem', new OmnisystemTaskProvider())
    );

    // Custom editors
    ctx.subscriptions.push(
        vscode.window.registerCustomEditorProvider(
            'omnisystem.buildEditor',
            new BuildEditorProvider(ctx),
            { supportsMultipleEditorsPerDocument: false }
        )
    );

    // Sylva notebook
    const sylvaSerializer = new SylvaNotebookSerializer();
    ctx.subscriptions.push(
        vscode.workspace.registerNotebookSerializer('sylva-notebook', sylvaSerializer),
        new SylvaNotebookKernel()
    );

    // (OmniHarness AI panel is registered synchronously in activate() so the
    // webview resolves even if this async path fails.)

    // Code actions
    ctx.subscriptions.push(
        vscode.languages.registerCodeActionsProvider(
            OMNI_DOC_SELECTORS,
            new OmniCodeActionsProvider(),
            { providedCodeActionKinds: [vscode.CodeActionKind.QuickFix] }
        )
    );

    // Semantic tokens
    const tokenTypes      = ['function', 'variable', 'type', 'parameter', 'keyword', 'operator'];
    const tokenModifiers  = ['declaration', 'definition', 'readonly', 'static', 'async'];
    const semanticLegend  = new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);
    ctx.subscriptions.push(
        vscode.languages.registerDocumentSemanticTokensProvider(
            OMNI_DOC_SELECTORS,
            new OmniSemanticTokensProvider(semanticLegend),
            semanticLegend
        )
    );

    registerCommands(ctx);
    registerFileWatchers(ctx);
    registerThemeIpcListener();

    ctx.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(async (e) => {
            if (
                e.affectsConfiguration('omnisystem.lspServerPath') ||
                e.affectsConfiguration('omnisystem.enableLsp')     ||
                e.affectsConfiguration('omnisystem.omniccPath')
            ) {
                outputChannel.appendLine('Configuration changed — restarting language server...');
                await restartLanguageClient();
            }
            if (e.affectsConfiguration('omnisystem.buildTarget')) {
                const target = vscode.workspace.getConfiguration(EXTENSION_ID).get<string>('buildTarget', 'x86_64-linux');
                statusBarPlatform.text = `$(cpu) ${target}`;
            }
            refreshAllTreeViews();
        })
    );

    // Start LSP asynchronously — never block activation on it
    const config = vscode.workspace.getConfiguration(EXTENSION_ID);
    if (config.get<boolean>('enableLsp', true)) {
        startLanguageClient().catch((err) => {
            outputChannel.appendLine(`LSP startup error (non-fatal): ${err}`);
            updateLspStatusBar('error');
        });
    }

    const isFirstInstall = !ctx.globalState.get<boolean>(FIRST_INSTALL_KEY, false);
    if (isFirstInstall) {
        await ctx.globalState.update(FIRST_INSTALL_KEY, true);
        showWelcome();
    }

    outputChannel.appendLine('Omnisystem extension activated.');
}

// ─── Deactivation ─────────────────────────────────────────────────────────────

export async function deactivate(): Promise<void> {
    await stopLanguageClient();
}

// ─── Workspace settings ───────────────────────────────────────────────────────

async function applyOmnisystemWorkspaceSettings(): Promise<void> {
    const wsConfig = vscode.workspace.getConfiguration();
    await wsConfig.update('editor.tabSize',                       4,    vscode.ConfigurationTarget.Workspace);
    await wsConfig.update('editor.formatOnSave',                  true, vscode.ConfigurationTarget.Workspace);
    await wsConfig.update('editor.bracketPairColorization.enabled', true, vscode.ConfigurationTarget.Workspace);
    await wsConfig.update('files.trimTrailingWhitespace',         true, vscode.ConfigurationTarget.Workspace);
}

// ─── Workspace detection ──────────────────────────────────────────────────────

async function detectWorkspace(): Promise<WorkspaceInfo> {
    const folders = vscode.workspace.workspaceFolders;
    const root    = folders?.[0]?.uri;

    let kind: WorkspaceKind    = 'unknown';
    let hasDesktop              = false;
    let hasOmniOS              = false;
    let desktopRoot: vscode.Uri | undefined;

    if (!root) {
        return { kind, root, hasDesktop, hasOmniOS, desktopRoot };
    }

    if (
        fs.existsSync(vscode.Uri.joinPath(root, 'BUILD.omnisystem').fsPath) ||
        fs.existsSync(vscode.Uri.joinPath(root, 'Omnisystem', 'BUILD.omnisystem').fsPath)
    ) {
        kind = 'omnisystem';
    }

    const omniOsCandidates = [
        vscode.Uri.joinPath(root, 'OmniOS_Bootstrap_Launcher.titan').fsPath,
        vscode.Uri.joinPath(root, 'src', 'OmniOS_Bootstrap_Launcher.titan').fsPath,
        vscode.Uri.joinPath(root, 'src', 'systems', 'OmniOS_Bootstrap_Launcher.titan').fsPath,
        vscode.Uri.joinPath(root, 'Omnisystem', 'OmniOS_Bootstrap_Launcher.titan').fsPath,
        vscode.Uri.joinPath(root, 'Omnisystem', 'src', 'OmniOS_Bootstrap_Launcher.titan').fsPath,
        vscode.Uri.joinPath(root, 'Omnisystem', 'src', 'systems', 'OmniOS_Bootstrap_Launcher.titan').fsPath,
    ];
    for (const c of omniOsCandidates) {
        if (fs.existsSync(c)) {
            hasOmniOS = true;
            if (kind === 'unknown') { kind = 'omnios'; }
            break;
        }
    }

    // 1. Honour explicit setting
    const settingPath = vscode.workspace.getConfiguration('omnisystem').get<string>('desktopPath');
    if (settingPath && fs.existsSync(settingPath)) {
        hasDesktop  = true;
        desktopRoot = vscode.Uri.file(settingPath);
        if (kind === 'unknown') { kind = 'desktop'; }
    }

    // 2. Well-known deep path inside an Omnisystem monorepo
    if (!hasDesktop) {
        // The desktop ecosystem was absorbed into Omnisystem at src/systems/desktop
        // (formerly src/systems/modules/base-modules/applications/desktop-ecosystem).
        const ecoRelative = path.join('src', 'systems', 'desktop');
        const legacyRelative = path.join('src', 'systems', 'modules', 'base-modules', 'applications', 'desktop-ecosystem');
        const candidates = [
            path.join(root.fsPath, ecoRelative),
            path.join(root.fsPath, 'Omnisystem', ecoRelative),
            path.join(root.fsPath, legacyRelative),
            path.join(root.fsPath, 'Omnisystem', legacyRelative),
        ];
        for (const knownEcoPath of candidates) {
            if (fs.existsSync(knownEcoPath)) {
                hasDesktop  = true;
                desktopRoot = vscode.Uri.file(knownEcoPath);
                if (kind === 'unknown') { kind = 'desktop'; }
                break;
            }
        }
    }

    // 3. Shallow scan (workspace root and one level deep) for the app manifest.
    //    Primary name is app.omnisystem.toml; app.bonsai.toml is the legacy name.
    if (!hasDesktop) {
        const desktopCandidates: string[] = [];
        if (folders) {
            for (const f of folders) {
                for (const name of APP_MANIFEST_NAMES) {
                    desktopCandidates.push(vscode.Uri.joinPath(f.uri, name).fsPath);
                }
            }
        }
        try {
            const entries = fs.readdirSync(root.fsPath, { withFileTypes: true });
            for (const entry of entries) {
                if (entry.isDirectory()) {
                    for (const name of APP_MANIFEST_NAMES) {
                        desktopCandidates.push(path.join(root.fsPath, entry.name, name));
                    }
                }
            }
        } catch { /* ignore */ }

        for (const c of desktopCandidates) {
            if (fs.existsSync(c)) {
                hasDesktop  = true;
                // Use the parent (ecosystem root), not the launcher subdirectory
                const dir = path.dirname(c);
                const parent = path.dirname(dir);
                const parentHasEco = fs.existsSync(path.join(parent, 'control-panel'));
                desktopRoot = vscode.Uri.file(parentHasEco ? parent : dir);
                if (kind === 'unknown') { kind = 'desktop'; }
                break;
            }
        }
    }

    if (kind === 'unknown') {
        try {
            const files = await vscode.workspace.findFiles('**/*.titan', '**/node_modules/**', 1);
            if (files.length > 0) { kind = 'titan'; }
        } catch { /* ignore */ }
    }

    return { kind, root, hasDesktop, hasOmniOS, desktopRoot };
}

// ─── Status bar ───────────────────────────────────────────────────────────────

function registerStatusBar(ctx: vscode.ExtensionContext): void {
    const target = vscode.workspace.getConfiguration(EXTENSION_ID).get<string>('buildTarget', 'x86_64-linux');

    statusBarMain = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarMain.text    = '$(gear) Omnisystem';
    statusBarMain.tooltip = 'Omnisystem — Open Build Dashboard';
    statusBarMain.command = 'omnisystem.openBuildDashboard';
    statusBarMain.show();

    statusBarLsp = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 99);
    statusBarLsp.text    = '$(loading~spin) LSP: Starting...';
    statusBarLsp.tooltip = 'Omnisystem Language Server — Click to restart';
    statusBarLsp.command = 'omnisystem.restartLsp';
    statusBarLsp.show();

    statusBarPlatform = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 51);
    statusBarPlatform.text    = `$(cpu) ${target}`;
    statusBarPlatform.tooltip = 'Omnisystem build target — Click to change';
    statusBarPlatform.command = 'omnisystem.omniOsSelectPlatform';
    statusBarPlatform.show();

    statusBarBuild = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 50);
    statusBarBuild.text    = '$(check) Build OK';
    statusBarBuild.tooltip = 'Last build result';
    statusBarBuild.command = 'omnisystem.openBuildDashboard';
    statusBarBuild.show();

    ctx.subscriptions.push(statusBarMain, statusBarLsp, statusBarPlatform, statusBarBuild);
}

type LspStatus = 'starting' | 'running' | 'stopped' | 'error' | 'warning';

function updateLspStatusBar(status: LspStatus): void {
    const labels: Record<LspStatus, string> = {
        starting: '$(loading~spin) LSP: Starting...',
        running:  '$(server) LSP: Active',
        stopped:  '$(circle-slash) LSP: Stopped',
        error:    '$(error) LSP: Error',
        warning:  '$(warning) LSP: Not Found',
    };
    statusBarLsp.text = labels[status];
}

function setBuildStatus(status: 'running' | 'ok' | 'failed'): void {
    const labels = {
        running: '$(gear~spin) Building...',
        ok:      '$(check) Build OK',
        failed:  '$(error) Build Failed',
    };
    statusBarBuild.text = labels[status];
}

// ─── LSP lifecycle ────────────────────────────────────────────────────────────

async function startLanguageClient(): Promise<void> {
    if (!omniccAvailable()) {
        outputChannel.appendLine('LSP: omnicc binary not found — language server disabled. Configure omnisystem.omniccPath to enable.');
        updateLspStatusBar('warning');
        return;
    }

    const config  = vscode.workspace.getConfiguration(EXTENSION_ID);
    const lspPath = resolvedOmniccPath();

    outputChannel.appendLine(`LSP: starting via ${lspPath}`);
    updateLspStatusBar('starting');

    const serverOptions: ServerOptions = buildServerOptions(lspPath, config);

    const clientOptions: LanguageClientOptions = {
        documentSelector: OMNI_LANGUAGES.map(lang => ({ scheme: 'file', language: lang })),
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher(
                '**/*.{titan,vera,helix,aether,axiom,sylva,nexus}'
            ),
            configurationSection: EXTENSION_ID,
        },
        outputChannel,
        traceOutputChannel: outputChannel,
        initializationOptions: {
            enableInlayHints:  config.get<boolean>('enableInlayHints', true),
            buildTarget:       config.get<string>('buildTarget', 'x86_64-linux'),
            optimizationLevel: config.get<string>('optimizationLevel', 'O0'),
        },
    };

    client = new LanguageClient(LSP_CLIENT_ID, LSP_CLIENT_NAME, serverOptions, clientOptions);

    client.onDidChangeState((e) => {
        if      (e.newState === State.Running)  { updateLspStatusBar('running');  outputChannel.appendLine('Language server is running.'); }
        else if (e.newState === State.Stopped)  { updateLspStatusBar('stopped');  outputChannel.appendLine('Language server stopped.'); }
        else if (e.newState === State.Starting) { updateLspStatusBar('starting'); outputChannel.appendLine('Language server starting...'); }
    });

    try {
        await client.start();
        extensionContext.subscriptions.push(client);
    } catch (err) {
        outputChannel.appendLine(`Failed to start language server: ${err}`);
        updateLspStatusBar('error');
        vscode.window.showErrorMessage(
            'Omnisystem: Failed to start language server. Check the Output panel for details.'
        );
    }
}

async function stopLanguageClient(): Promise<void> {
    if (client) {
        await client.stop();
        client = undefined;
    }
}

async function restartLanguageClient(): Promise<void> {
    await stopLanguageClient();
    const config = vscode.workspace.getConfiguration(EXTENSION_ID);
    if (config.get<boolean>('enableLsp', true)) {
        await startLanguageClient();
    }
}

// ─── LSP path resolution ──────────────────────────────────────────────────────

function resolveLspServerPath(config: vscode.WorkspaceConfiguration): string | null {
    const configured = config.get<string>('lspServerPath', '').trim();
    if (configured && fs.existsSync(configured)) { return configured; }

    const cc     = config.get<string>('omniccPath', 'omnicc').trim();
    const ccDir  = path.dirname(cc);
    const bins   = ['omnicc-lsp', 'omnicc-lsp.exe', 'LspServer', 'LspServer.exe'];
    for (const b of bins) {
        const p = path.join(ccDir, b);
        if (fs.existsSync(p)) { return p; }
    }

    const wsRoot = workspaceRoot();
    if (wsRoot) {
        const wsBins = ['build/LspServer', 'build/LspServer.exe', 'build/omnicc-lsp', 'build/omnicc-lsp.exe'];
        for (const b of wsBins) {
            const p = path.join(wsRoot, b);
            if (fs.existsSync(p)) { return p; }
        }
    }

    return cc || null;
}

function buildServerOptions(lspPath: string, _config: vscode.WorkspaceConfiguration): ServerOptions {
    // lspPath is either:
    //   "Z:\...\bin\omnicc.cmd"   → run as shell command with args
    //   "Z:\...\bin\omnicc.js"    → run via node
    // In both cases we pass "lsp --stdio" as arguments.
    if (lspPath.endsWith('.js')) {
        return {
            run:   { command: 'node', args: [lspPath, 'lsp', '--stdio'],            transport: TransportKind.stdio },
            debug: { command: 'node', args: [lspPath, 'lsp', '--stdio', '--debug'], transport: TransportKind.stdio },
        };
    }
    // .cmd / .ps1 / bare binary — pass through shell
    return {
        run:   { command: lspPath, args: ['lsp', '--stdio'],            transport: TransportKind.stdio, options: { shell: true } },
        debug: { command: lspPath, args: ['lsp', '--stdio', '--debug'], transport: TransportKind.stdio, options: { shell: true } },
    };
}

// ─── File watchers ────────────────────────────────────────────────────────────

function registerFileWatchers(ctx: vscode.ExtensionContext): void {
    const omniWatcher   = vscode.workspace.createFileSystemWatcher('**/*.{titan,vera,helix,aether,axiom,sylva,nexus}');
    const buildWatcher  = vscode.workspace.createFileSystemWatcher('**/BUILD.omnisystem');
    const desktopWatcher = vscode.workspace.createFileSystemWatcher('**/app.{omnisystem,desktop}.toml');

    omniWatcher.onDidCreate(() => { explorerProvider.refresh(); });
    omniWatcher.onDidDelete(() => { explorerProvider.refresh(); });

    buildWatcher.onDidChange(() => { omnipmProvider.refresh(); explorerProvider.refresh(); });
    buildWatcher.onDidCreate(() => refreshAllTreeViews());
    buildWatcher.onDidDelete(() => refreshAllTreeViews());

    desktopWatcher.onDidCreate(() => refreshAllTreeViews());
    desktopWatcher.onDidDelete(() => refreshAllTreeViews());
    desktopWatcher.onDidChange(() => desktopProvider.refresh());

    ctx.subscriptions.push(omniWatcher, buildWatcher, desktopWatcher);
}

function refreshAllTreeViews(): void {
    explorerProvider.refresh();
    desktopProvider.refresh();
    omnipmProvider.refresh();
    omniosProvider.refresh();
}

// ─── Core build helper ────────────────────────────────────────────────────────

async function runCommandWithProgress(label: string, args: string[], cwd?: string): Promise<void> {
    setBuildStatus('running');
    outputChannel.show(true);
    outputChannel.appendLine(`\n[${label}] ${omniccPath()} ${args.join(' ')}`);
    outputChannel.appendLine('─'.repeat(60));

    await vscode.window.withProgress(
        { location: vscode.ProgressLocation.Notification, title: label, cancellable: true },
        (progress, token) => new Promise<void>((resolve, reject) => {
            const proc = spawn(omniccPath(), args, { cwd: cwd ?? workspaceRoot(), shell: true });

            proc.stdout.on('data', (d: Buffer) => {
                const lines = d.toString().split('\n').filter(Boolean);
                for (const line of lines) {
                    outputChannel.appendLine(line);
                    progress.report({ message: line.slice(0, 80) });
                }
            });
            proc.stderr.on('data', (d: Buffer) => {
                outputChannel.appendLine('[stderr] ' + d.toString().trimEnd());
            });
            token.onCancellationRequested(() => proc.kill());
            proc.on('close', (code) => {
                if (code === 0) {
                    outputChannel.appendLine(`[${label}] Completed successfully.`);
                    setBuildStatus('ok');
                    resolve();
                } else {
                    outputChannel.appendLine(`[${label}] Failed with exit code ${code}.`);
                    setBuildStatus('failed');
                    reject(new Error(`${label} failed with exit code ${code}`));
                }
            });
            proc.on('error', (err) => {
                outputChannel.appendLine(`[${label}] Process error: ${err.message}`);
                setBuildStatus('failed');
                reject(err);
            });
        })
    ).then(
        () => { /* already resolved */ },
        (err: Error) => vscode.window.showErrorMessage(`Omnisystem: ${err.message}`)
    );
}

// ─── Terminal runners ─────────────────────────────────────────────────────────

/** Run an omnicc subcommand in a terminal (prepends omnicc path). */
function runInTerminal(subArgs: string[], terminalName?: string): void {
    const config   = vscode.workspace.getConfiguration(EXTENSION_ID);
    const target   = config.get<string>('buildTarget', 'x86_64-linux');
    const optLevel = config.get<string>('optimizationLevel', 'O0');
    const cwd      = workspaceRoot();

    const fullArgs = [...subArgs];
    if ((subArgs[0] === 'build' || subArgs[0] === 'run') && !subArgs.includes('--target')) {
        fullArgs.push('--target', target, '--opt', optLevel);
    }
    const command = `${omniccPath()} ${fullArgs.join(' ')}`;
    const name    = terminalName ?? `Omnisystem: ${subArgs[0]}`;

    let terminal = vscode.window.terminals.find((t) => t.name === name);
    if (!terminal || terminal.exitStatus !== undefined) {
        terminal = vscode.window.createTerminal({ name, cwd });
    }
    terminal.show(true);
    terminal.sendText(command);
}

/** Run an arbitrary command (does NOT prepend omnicc). */
function runRawInTerminal(command: string, terminalName: string): void {
    const cwd = workspaceRoot();
    let terminal = vscode.window.terminals.find((t) => t.name === terminalName);
    if (!terminal || terminal.exitStatus !== undefined) {
        terminal = vscode.window.createTerminal({ name: terminalName, cwd });
    }
    terminal.show(true);
    terminal.sendText(command);
}

// ─── omnicc availability guard ────────────────────────────────────────────────

function omniccAvailable(): boolean {
    const p = resolvedOmniccPath();
    if (!p || p === 'omnicc') { return false; }
    if (p.startsWith('node ')) {
        // node "<path>" — check the js file exists
        const jsPath = p.replace(/^node\s+"?/, '').replace(/"$/, '');
        return fs.existsSync(jsPath);
    }
    return fs.existsSync(p);
}

function resolvedOmniccPath(): string {
    const configured = vscode.workspace.getConfiguration(EXTENSION_ID).get<string>('omniccPath', '').trim();
    // VS Code does not expand ${workspaceFolder} in API reads — do it ourselves
    const root = workspaceRoot() ?? '';
    const expanded = configured.replace(/\$\{workspaceFolder\}/g, root);
    if (expanded && expanded !== 'omnicc') { return expanded; }

    // Auto-detect relative to every workspace folder
    const roots = [
        root,
        path.join(root, 'Omnisystem'),
        path.join(root, '..'),
        path.join(root, '..', 'Omnisystem'),
    ];
    for (const r of roots) {
        for (const name of ['omnicc.cmd', 'omnicc.ps1', 'omnicc.js']) {
            const candidate = path.join(r, 'bin', name);
            if (fs.existsSync(candidate)) { return candidate; }
        }
    }
    return 'omnicc';
}

async function requireOmnicc(): Promise<boolean> {
    if (omniccAvailable()) { return true; }
    const action = await vscode.window.showWarningMessage(
        'Omnisystem: omnicc compiler not found. Set the path to the omnicc binary to use build commands.',
        'Configure Path',
        'Dismiss'
    );
    if (action === 'Configure Path') {
        await vscode.commands.executeCommand('workbench.action.openSettings', 'omnisystem.omniccPath');
    }
    return false;
}

// ─── OmniHarness Substrate commands ─────────────────────────────────────────────

function harnessServerUrl(): string {
    return vscode.workspace.getConfiguration(EXTENSION_ID).get<string>('harness.serverUrl', 'http://localhost:8080');
}

async function cmdSubstrateSwarm(): Promise<void> {
    const { OmniHarnessClient } = await import('./harness/OmniHarnessClient');
    const task = await vscode.window.showInputBox({ prompt: 'Swarm task / objective', placeHolder: 'e.g. Design and critique a caching layer for the API' });
    if (!task) { return; }
    const topology = await vscode.window.showQuickPick(
        ['orchestrator', 'debate', 'parallel', 'pipeline'],
        { placeHolder: 'Swarm topology' },
    );
    if (!topology) { return; }
    const model = vscode.workspace.getConfiguration(EXTENSION_ID).get<string>('harness.defaultModel', 'anthropic/claude-sonnet-4-6');
    const agents = [
        { id: 'lead', name: 'Lead', role: 'orchestrator', model, system: 'You are the lead architect. Decompose, coordinate, and synthesize.' },
        { id: 'impl', name: 'Implementer', role: 'worker', model, system: 'You are a senior engineer focused on correct, concrete implementation.' },
        { id: 'critic', name: 'Critic', role: 'critic', model, system: 'You are a rigorous reviewer who finds flaws, edge cases, and risks.' },
    ];
    const client = new OmniHarnessClient(harnessServerUrl());
    outputChannel.show(true);
    outputChannel.appendLine(`\n[Substrate Swarm] topology=${topology} task=${task}`);
    await vscode.window.withProgress(
        { location: vscode.ProgressLocation.Notification, title: `Running ${topology} swarm…`, cancellable: false },
        async () => {
            try {
                const res = await client.swarm({ topology, task, agents, rounds: 2 });
                outputChannel.appendLine('─'.repeat(60));
                outputChannel.appendLine(res.output);
                outputChannel.appendLine('─'.repeat(60));
                outputChannel.appendLine(`[governance] ${JSON.stringify(res.governance)}`);
            } catch (err) {
                outputChannel.appendLine(`[Substrate Swarm] error: ${err instanceof Error ? err.message : err}`);
                vscode.window.showErrorMessage('Swarm failed — is the OmniHarness orchestrator running? See Output.');
            }
        },
    );
}

async function cmdSubstrateEnsemble(): Promise<void> {
    const { OmniHarnessClient } = await import('./harness/OmniHarnessClient');
    const prompt = await vscode.window.showInputBox({ prompt: 'Prompt to send to a blend of models', placeHolder: 'e.g. What are the tradeoffs of event sourcing?' });
    if (!prompt) { return; }
    const modelsRaw = await vscode.window.showInputBox({
        prompt: 'Models to blend (comma-separated)',
        value: 'anthropic/claude-sonnet-4-6, gpt-4o, gemini/gemini-2.0-flash',
    });
    if (!modelsRaw) { return; }
    const models = modelsRaw.split(',').map((m) => m.trim()).filter(Boolean);
    const strategy = await vscode.window.showQuickPick(['judge', 'moa', 'vote', 'concat'], { placeHolder: 'Aggregation strategy' });
    if (!strategy) { return; }
    const client = new OmniHarnessClient(harnessServerUrl());
    outputChannel.show(true);
    outputChannel.appendLine(`\n[Substrate Ensemble] strategy=${strategy} models=${models.join(', ')}`);
    await vscode.window.withProgress(
        { location: vscode.ProgressLocation.Notification, title: `Blending ${models.length} models…`, cancellable: false },
        async () => {
            try {
                const res = await client.ensemble({ prompt, models, strategy, judge_model: models[0] });
                outputChannel.appendLine('─'.repeat(60));
                outputChannel.appendLine(res.final);
                outputChannel.appendLine('─'.repeat(60));
                outputChannel.appendLine(`[governance] ${JSON.stringify(res.governance)}`);
            } catch (err) {
                outputChannel.appendLine(`[Substrate Ensemble] error: ${err instanceof Error ? err.message : err}`);
                vscode.window.showErrorMessage('Ensemble failed — is the OmniHarness orchestrator running? See Output.');
            }
        },
    );
}

// ─── Build commands ───────────────────────────────────────────────────────────

async function cmdBuild(): Promise<void> {
    if (!omniccAvailable()) {
        // omnicc not configured — open the visual Build Dashboard instead
        BuildDashboardPanel.createOrShow(extensionContext.extensionUri);
        return;
    }
    const config  = vscode.workspace.getConfiguration(EXTENSION_ID);
    const target  = config.get<string>('buildTarget', 'x86_64-linux');
    const optLevel = config.get<string>('optimizationLevel', 'O0');
    await runCommandWithProgress('Build', ['build', '--target', target, '--opt', optLevel]);
}

async function cmdBuildRelease(): Promise<void> {
    if (!omniccAvailable()) { BuildDashboardPanel.createOrShow(extensionContext.extensionUri); return; }
    const config = vscode.workspace.getConfiguration(EXTENSION_ID);
    const target = config.get<string>('buildTarget', 'x86_64-linux');
    await runCommandWithProgress('Build Release', ['build', '--release', '--target', target]);
}

async function cmdBuildWasm(): Promise<void> {
    if (!omniccAvailable()) { BuildDashboardPanel.createOrShow(extensionContext.extensionUri); return; }
    await runCommandWithProgress('Build WASM', ['build', '--target', 'wasm32']);
}

async function cmdBuildLinux(): Promise<void> {
    if (!omniccAvailable()) { BuildDashboardPanel.createOrShow(extensionContext.extensionUri); return; }
    await runCommandWithProgress('Build Linux', ['build', '--target', 'x86_64-linux']);
}

function cmdBuildWatch(): void {
    if (!omniccAvailable()) { BuildDashboardPanel.createOrShow(extensionContext.extensionUri); return; }
    runInTerminal(['build', '--watch'], 'Omnisystem: build --watch');
}

async function cmdBuildFile(): Promise<void> {
    if (!await requireOmnicc()) { return; }
    const file = vscode.window.activeTextEditor?.document.uri.fsPath;
    if (!file) {
        vscode.window.showWarningMessage('Omnisystem: No active file to build.');
        return;
    }
    await runCommandWithProgress(`Build ${path.basename(file)}`, ['build', file]);
}

async function cmdRunFile(): Promise<void> {
    if (!await requireOmnicc()) { return; }
    const file = vscode.window.activeTextEditor?.document.uri.fsPath;
    if (!file) {
        vscode.window.showWarningMessage('Omnisystem: No active file to run.');
        return;
    }
    runInTerminal(['run', file], `Omnisystem: run ${path.basename(file)}`);
}

// Resolve the OmniCC bootstrap launcher (bootstrap/omnicc.mjs) by searching the
// workspace root and this extension's neighbours. Returns null if not found.
function bootstrapLauncher(): string | null {
    const candidates: string[] = [];
    const root = workspaceRoot();
    if (root) {
        candidates.push(path.join(root, 'bootstrap', 'omnicc.mjs'));
        candidates.push(path.join(root, 'Omnisystem', 'bootstrap', 'omnicc.mjs'));
    }
    const cfg = vscode.workspace.getConfiguration(EXTENSION_ID).get<string>('bootstrapPath', '').trim();
    if (cfg) { candidates.unshift(cfg); }
    for (const c of candidates) {
        try { if (fs.existsSync(c)) { return c; } } catch { /* ignore */ }
    }
    return null;
}

// Run the active .titan file through the bootstrap Titan runtime (Node-hosted,
// no external toolchain required). This is the seed interpreter in ../bootstrap.
async function cmdRunTitanBootstrap(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    const file = editor?.document.uri.fsPath;
    if (!file || !file.endsWith('.titan')) {
        vscode.window.showWarningMessage('Omnisystem: open a .titan file to run it with the bootstrap runtime.');
        return;
    }
    await editor!.document.save();
    const launcher = bootstrapLauncher();
    if (!launcher) {
        vscode.window.showErrorMessage('Omnisystem: could not find bootstrap/omnicc.mjs. Set "omnisystem.bootstrapPath" to its location.');
        return;
    }
    const name = `Titan (bootstrap): ${path.basename(file)}`;
    let terminal = vscode.window.terminals.find((t) => t.name === name);
    if (!terminal || terminal.exitStatus !== undefined) {
        terminal = vscode.window.createTerminal({ name, cwd: path.dirname(launcher) });
    }
    terminal.show(true);
    terminal.sendText(`node "${launcher}" run "${file}"`);
}

async function cmdClean(): Promise<void> {
    if (!await requireOmnicc()) { return; }
    await runCommandWithProgress('Clean', ['clean']);
}

async function cmdRun(): Promise<void> {
    if (!await requireOmnicc()) { return; }
    runInTerminal(['run'], 'Omnisystem: run');
}

async function cmdTest(): Promise<void> {
    if (!await requireOmnicc()) { return; }
    outputChannel.show(true);
    outputChannel.appendLine('\n[Test Suite]');
    outputChannel.appendLine('─'.repeat(60));
    await runCommandWithProgress('Test', ['test', '--verbose']);
}

function cmdBenchmark(): void {
    if (!omniccAvailable()) { void requireOmnicc(); return; }
    runInTerminal(['bench'], 'Omnisystem: bench');
}

async function cmdProfile(): Promise<void> {
    if (!await requireOmnicc()) { return; }
    const config = vscode.workspace.getConfiguration(EXTENSION_ID);
    const target = config.get<string>('buildTarget', 'x86_64-linux');
    await runCommandWithProgress('Profile Build', ['build', '--profile', '--target', target]);
    const root = workspaceRoot();
    if (root) {
        const profileOut = path.join(root, 'target', 'profile.json');
        if (fs.existsSync(profileOut)) {
            const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(profileOut));
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Two);
        } else {
            outputChannel.appendLine('[Profile] Profile data written to target/profile.json (run again to view).');
        }
    }
}

// ─── LSP / editor commands ────────────────────────────────────────────────────

async function cmdRestartLsp(): Promise<void> {
    vscode.window.showInformationMessage('Omnisystem: Restarting language server...');
    updateLspStatusBar('starting');
    await restartLanguageClient();
}

function cmdFormatDocument(): void {
    vscode.commands.executeCommand('editor.action.formatDocument');
}

function cmdOrganizeImports(): void {
    vscode.commands.executeCommand('editor.action.organizeImports');
}

function cmdGenerateDocs(): void {
    runInTerminal(['doc'], 'Omnisystem: doc');
}

function cmdRunAxiomVerification(): void {
    runInTerminal(['verify', '--axiom'], 'Omnisystem: axiom verify');
}

function cmdShowTypeHierarchy(): void {
    vscode.commands.executeCommand('editor.showTypeHierarchyView');
}

async function cmdInlayHintsToggle(): Promise<void> {
    const wsConfig  = vscode.workspace.getConfiguration();
    const current   = wsConfig.get<string>('editor.inlayHints.enabled', 'on');
    const next      = current === 'off' ? 'on' : 'off';
    await wsConfig.update('editor.inlayHints.enabled', next, vscode.ConfigurationTarget.Workspace);
    vscode.window.showInformationMessage(`Omnisystem: Inlay hints ${next}.`);
}

// ─── OmniOS commands ──────────────────────────────────────────────────────────

async function cmdOmniOsBoot(): Promise<void> {
    outputChannel.show(false);
    outputChannel.appendLine('\n════════════════════════════════════════════');
    outputChannel.appendLine('  OmniOS Sandboxing Immune System — Boot Sequence');
    outputChannel.appendLine('════════════════════════════════════════════');

    const osModule = require('os') as typeof import('os');
    const pathMod  = require('path') as typeof import('path');
    const platform = osModule.platform();
    const arch     = osModule.arch();
    outputChannel.appendLine(`[Sandbox] Platform  : ${platform} / ${arch}`);
    outputChannel.appendLine(`[Sandbox] Host CPUs : ${osModule.cpus().length} cores`);
    outputChannel.appendLine(`[Sandbox] Host RAM  : ${Math.round(osModule.totalmem() / 1073741824 * 10) / 10} GB`);
    outputChannel.appendLine('[Sandbox] Initializing Sanctum Vault subsystem...');

    const wsRootRaw = workspaceRoot();
    // Resolve the actual Omnisystem source root (handles monorepo offset)
    const wsRoot = (() => {
        if (!wsRootRaw) { return wsRootRaw; }
        const sub = pathMod.join(wsRootRaw, 'Omnisystem');
        if (fs.existsSync(pathMod.join(sub, 'BUILD.omnisystem'))) { return sub; }
        if (fs.existsSync(pathMod.join(wsRootRaw, 'BUILD.omnisystem'))) { return wsRootRaw; }
        return sub;
    })();
    const sandboxSources: Array<[string, string]> = [
        ['Sanctum Vault Kernel',  'src/systems/UOSC/kernel/sanctum.ti'],
        ['Env-Fabric Manager',    'src/systems/runtime/services/env-fabric/manager.ti'],
        ['Env-Fabric Payload',    'src/systems/runtime/services/env-fabric/payload.ti'],
        ['Sandbox Immune System', 'src/systems/runtime/services/sandbox/SandboxImmuneSystem.titan'],
        ['UOSC Capability Layer', 'src/systems/UOSC/kernel/capability.ti'],
    ];
    const vaultStatus: Array<{name: string; present: boolean; path: string}> = [];
    for (const [name, rel] of sandboxSources) {
        const fullPath = wsRoot ? pathMod.join(wsRoot, rel) : '';
        const present  = fullPath ? fs.existsSync(fullPath) : false;
        outputChannel.appendLine(`[Sandbox]   ${present ? '✓' : '○'} ${name}`);
        vaultStatus.push({ name, present, path: rel });
    }

    const isolationMode = platform === 'win32'  ? 'Windows Job Objects + Restricted Token'
                        : platform === 'linux'   ? 'Linux Namespaces + seccomp + cgroups v2'
                        : platform === 'darwin'  ? 'macOS App Sandbox + Seatbelt'
                        :                          'POSIX rlimits + chroot';
    outputChannel.appendLine(`[Sandbox] Isolation : ${isolationMode}`);

    const vaultId  = `svlt-${Date.now().toString(36).toUpperCase()}`;
    const immuneId = `imm-${process.pid.toString(16).toUpperCase()}`;
    outputChannel.appendLine(`[Sandbox] Vault ID  : ${vaultId}`);
    outputChannel.appendLine(`[Sandbox] Immune ID : ${immuneId}`);
    outputChannel.appendLine('[Sandbox] Network policy:   deny-by-default (allowlist only)');
    outputChannel.appendLine('[Sandbox] FS policy:        workspace-scoped read/write');
    outputChannel.appendLine('[Sandbox] IPC policy:       Sanctum-mediated channels only');
    outputChannel.appendLine('[Sandbox] Immune System:    ACTIVE — monitoring all vault events');
    outputChannel.appendLine('[Sandbox] ✓ OmniOS Sandbox ready — launching Desktop...\n');

    OmniOSDesktopPanel.createOrShow(extensionContext.extensionUri, extensionContext);

    setTimeout(() => {
        OmniOSDesktopPanel.postMessage({
            type: 'sandboxInit',
            vaultId,
            immuneId,
            platform,
            arch,
            isolationMode,
            immuneActive: true,
            vaultStatus,
        });
    }, 600);
}

function cmdOmniOsSystemServices(): void {
    OmniOSDesktopPanel.createOrShow(extensionContext.extensionUri, extensionContext);
}

function cmdOmniOsKernelLog(): void {
    outputChannel.show(true);
    outputChannel.appendLine('\n[OmniOS Kernel Log — streamed from OmniOS Desktop]');
    OmniOSDesktopPanel.createOrShow(extensionContext.extensionUri, extensionContext);
}

async function cmdOmniOsSelectPlatform(): Promise<void> {
    const platforms = [
        { label: 'x86_64-linux',   description: 'Linux x86-64' },
        { label: 'x86_64-windows', description: 'Windows x86-64' },
        { label: 'aarch64-linux',  description: 'Linux ARM64' },
        { label: 'aarch64-macos',  description: 'macOS Apple Silicon' },
        { label: 'wasm32',         description: 'WebAssembly' },
        { label: 'embedded',       description: 'Embedded / bare metal' },
    ];
    const chosen = await vscode.window.showQuickPick(platforms, {
        placeHolder: 'Select build target platform',
    });
    if (!chosen) { return; }
    const config = vscode.workspace.getConfiguration(EXTENSION_ID);
    await config.update('buildTarget', chosen.label, vscode.ConfigurationTarget.Workspace);
    statusBarPlatform.text = `$(cpu) ${chosen.label}`;
    vscode.window.showInformationMessage(`Omnisystem: Build target set to ${chosen.label}.`);
}

async function cmdOmniOsPackageOs(): Promise<void> {
    if (!await requireOmnicc()) { return; }
    runInTerminal(['omnios', 'package'], 'Omnisystem: omnios package');
}

function cmdOpenSystemsExplorer(): void {
    vscode.commands.executeCommand('omniOsExplorer.focus');
}

// ─── OmniPM commands ──────────────────────────────────────────────────────────

async function cmdOmniPMInstall(): Promise<void> {
    if (!omniccAvailable()) {
        // omnicc not configured — refresh the OmniPM tree view and focus it
        omnipmProvider.refresh();
        await vscode.commands.executeCommand('omnipmExplorer.focus');
        vscode.window.showInformationMessage(
            'OmniPM: Dependencies listed in BUILD.omnisystem. Configure omnicc to run install.',
            'Configure omnicc'
        ).then(action => {
            if (action === 'Configure omnicc') {
                vscode.commands.executeCommand('workbench.action.openSettings', 'omnisystem.omniccPath');
            }
        });
        return;
    }
    await runCommandWithProgress('OmniPM Install', ['pm', 'install']);
    omnipmProvider.refresh();
}

async function cmdOmniPMAdd(): Promise<void> {
    const pkg = await vscode.window.showInputBox({
        prompt: 'Package to add (e.g. omni-http@1.2.0)',
        placeHolder: 'package-name[@version]',
        validateInput: (v) => v.trim() ? null : 'Package name is required',
    });
    if (!pkg) { return; }
    await runCommandWithProgress(`Add ${pkg.trim()}`, ['pm', 'add', pkg.trim()]);
    omnipmProvider.refresh();
}

async function cmdOmniPMRemove(): Promise<void> {
    // Read current dependencies from BUILD.omnisystem if available
    const root    = workspaceRoot();
    let packages: string[] = [];
    if (root) {
        const buildFile = path.join(root, 'BUILD.omnisystem');
        if (fs.existsSync(buildFile)) {
            const text = fs.readFileSync(buildFile, 'utf8');
            const matches = text.matchAll(/dep\s+"([^"]+)"/g);
            for (const m of matches) { packages.push(m[1]); }
        }
    }

    let pkg: string | undefined;
    if (packages.length > 0) {
        pkg = await vscode.window.showQuickPick(packages, { placeHolder: 'Select package to remove' });
    } else {
        pkg = await vscode.window.showInputBox({
            prompt: 'Package name to remove',
            placeHolder: 'package-name',
            validateInput: (v) => v.trim() ? null : 'Package name is required',
        });
    }
    if (!pkg) { return; }
    await runCommandWithProgress(`Remove ${pkg}`, ['pm', 'remove', pkg]);
    omnipmProvider.refresh();
}

async function cmdOmniPMUpdate(): Promise<void> {
    await runCommandWithProgress('OmniPM Update', ['pm', 'update']);
    omnipmProvider.refresh();
}

function cmdOmniPMPublish(): void {
    runInTerminal(['pm', 'publish'], 'Omnisystem: pm publish');
}

async function cmdOmniPMSearch(): Promise<void> {
    const query = await vscode.window.showInputBox({
        prompt: 'Search the OmniPM registry',
        placeHolder: 'search term',
    });
    if (!query) { return; }
    runInTerminal(['pm', 'search', query.trim()], 'Omnisystem: pm search');
}

function cmdOmniPMAudit(): void {
    runInTerminal(['pm', 'audit'], 'Omnisystem: pm audit');
}

// ─── Desktop commands ──────────────────────────────────────────────────────────

// Desktop is a separate app ecosystem — it has its own CLI, not an omnicc subcommand.
// All Desktop commands use runRawInTerminal or open the Desktop Dashboard webview.

function cmdDesktopLaunch(): void {
    // Primary: open the Desktop Dashboard webview inside VS Code.
    // Secondary: also try launching via npm/desktop CLI if in a Desktop workspace.
    OmnisystemDashboardPanel.createOrShow(extensionContext.extensionUri);
    const root = workspaceRoot();
    if (root && APP_MANIFEST_NAMES.some((n) => fs.existsSync(path.join(root, n)))) {
        runRawInTerminal('npm run dev', 'Desktop: Dev Server');
    }
}

function cmdDesktopBuddyConnect(): void {
    runRawInTerminal('npx desktop-buddy connect', 'Desktop: Buddy Connect');
}

async function cmdDesktopBuddyBuild(): Promise<void> {
    OmnisystemDashboardPanel.createOrShow(extensionContext.extensionUri);
    runRawInTerminal('npm run build:android', 'Desktop: Android Build');
}

function cmdDesktopBrowserExtBuild(): void {
    runRawInTerminal('npm run build:extension', 'Desktop: Browser Extension Build');
}

function cmdDesktopBrowserExtInstall(): void {
    runRawInTerminal('npm run build:extension && npx web-ext run', 'Desktop: Browser Extension Dev');
}

function cmdDesktopControlPanel(): void {
    // Control Panel Titan server runs on port 12345 (see control-panel/api_server.ti)
    vscode.env.openExternal(vscode.Uri.parse('http://localhost:12345'));
}

function cmdDesktopNotifications(): void {
    OmnisystemDashboardPanel.createOrShow(extensionContext.extensionUri);
    outputChannel.appendLine('[Desktop] Opening Notification System panel...');
    outputChannel.show(true);
}

function cmdDesktopSystemTray(): void {
    outputChannel.appendLine('[Desktop] System Tray module: src/systems/desktop/system-tray/core.ti');
    outputChannel.show(true);
    // Open the system-tray source file
    const root = workspaceRoot();
    if (root) {
        const trayFile = path.join(root, 'src', 'systems', 'desktop', 'system-tray', 'core.ti');
        if (fs.existsSync(trayFile)) {
            vscode.window.showTextDocument(vscode.Uri.file(trayFile));
        }
    }
}

async function cmdDesktopInit(): Promise<void> {
    const pick = await vscode.window.showQuickPick(
        ['Full Initialization', 'Diagnostics Mode', 'Repair Mode', 'Graceful Shutdown'],
        { placeHolder: 'Select Desktop Ecosystem operation', title: 'Desktop Initialization' }
    );
    if (!pick) { return; }
    outputChannel.appendLine(`[Desktop Init] ${pick} → INITIALIZATION.ti`);
    outputChannel.show(true);
    vscode.window.showInformationMessage(`Desktop Ecosystem: ${pick} triggered.`);
}

async function cmdDesktopDeploy(): Promise<void> {
    runRawInTerminal('npm run deploy', 'Desktop: Deploy');
}

function cmdDesktopModelManager(): void {
    MlStudioPanel.createOrShow(extensionContext.extensionUri);
}

function cmdDesktopWorkspace(): void {
    OmnisystemDashboardPanel.createOrShow(extensionContext.extensionUri);
}

function cmdOpenDesktopDashboard(): void {
    OmnisystemDashboardPanel.createOrShow(extensionContext.extensionUri);
}

// ─── New file scaffolding ─────────────────────────────────────────────────────

const TEMPLATES: Record<OmniLanguage, (name: string) => string> = {
    titan: (name) =>
`// ${name}.titan
mod ${name} {
    pub fn main() {
        let message: String = "Hello from ${name}"
        println(message)
    }
}
`,
    vera: (name) =>
`component ${name} {
    props {
        title: String
        visible: Bool = true
    }
    state {
        count: u32 = 0
    }
    render {
        <div class="${name.toLowerCase()}">
            <h2>{self.title}</h2>
            <span>{self.count}</span>
        </div>
    }
    on_mount() {
        self.count = 0
    }
}
`,
    helix: (name) =>
`shader ${name} {
    @vertex
    fn vs_main(in: VertexInput) -> VertexOutput {
        var out: VertexOutput
        out.position = in.position * vec4<f32>(1.0, 1.0, 0.0, 1.0)
        out.color = in.color
        return out
    }
    @fragment
    fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
        return in.color
    }
}
`,
    aether: (name) =>
`actor ${name} {
    state {
        count: u32 = 0
        name: String = "${name}"
    }
    message Init { id: u32 }
    message Increment { amount: u32 }
    handler Init(msg: Init) {
        self.count = 0
        self.log("Actor initialized with id " + msg.id.to_string())
    }
    handler Increment(msg: Increment) {
        self.count = self.count + msg.amount
        self.emit("count_changed", self.count)
    }
}
`,
    axiom: (name) =>
`theorem ${name} {
    preconditions {
        input_valid: input != null
        input_bounded: input.len() > 0 && input.len() <= 1024
    }
    postconditions {
        result_valid: result != null
        result_deterministic: forall x: input. f(x) == f(x)
    }
    invariants {
        state_consistent: self.count >= 0
    }
    assertions {
        assert input_valid
        assert result_valid
        assert state_consistent
    }
}
`,
    sylva: (name) =>
`model ${name} {
    architecture: [
        Dense { units: 128, activation: relu }
        Dropout { rate: 0.2 }
        Dense { units: 64, activation: relu }
        Dense { units: 10, activation: softmax }
    ]
    loss: cross_entropy
    optimizer: adam(lr=0.001)
    metrics: [accuracy, f1_score]
}
`,
    nexus: (name) =>
`layout ${name} {
    breakpoints {
        sm: 640px
        md: 768px
        lg: 1024px
        xl: 1280px
    }
    container .main {
        max_width: 1200px
        padding: 16px
    }
    flex column gap-4 {
        header .top { height: 64px }
        main .content { flex: 1 }
        footer .bottom { height: 48px }
    }
}
`,
};

async function scaffoldFile(lang: OmniLanguage): Promise<void> {
    const name = await vscode.window.showInputBox({
        prompt: `Enter ${lang.toUpperCase()} file name (without extension)`,
        placeHolder: 'MyModule',
        validateInput: (v) => v.trim() ? null : 'Name is required',
    });
    if (!name) { return; }

    const folders = vscode.workspace.workspaceFolders;
    if (!folders) {
        vscode.window.showErrorMessage('Omnisystem: No workspace folder open.');
        return;
    }

    const targetUri = vscode.Uri.joinPath(folders[0].uri, `${name}.${lang}`);
    const template  = TEMPLATES[lang]?.(name) ?? `// ${name}.${lang}\n`;

    try {
        await vscode.workspace.fs.writeFile(targetUri, Buffer.from(template, 'utf8'));
        const doc = await vscode.workspace.openTextDocument(targetUri);
        await vscode.window.showTextDocument(doc);
        explorerProvider.refresh();
    } catch (err) {
        vscode.window.showErrorMessage(`Omnisystem: Failed to create file: ${err}`);
    }
}

async function scaffoldDesktopApp(): Promise<void> {
    const name = await vscode.window.showInputBox({
        prompt: 'Desktop app name',
        placeHolder: 'my-desktop-app',
        validateInput: (v) => v.trim() ? null : 'Name is required',
    });
    if (!name) { return; }

    const folders = vscode.workspace.workspaceFolders;
    if (!folders) {
        vscode.window.showErrorMessage('Omnisystem: No workspace folder open.');
        return;
    }

    const appDir = vscode.Uri.joinPath(folders[0].uri, name.trim());
    const toml   = `[app]\nname = "${name.trim()}"\nversion = "0.1.0"\ntargets = ["android", "browser"]\n`;
    const mainTs = TEMPLATES.titan(name.trim());

    try {
        await vscode.workspace.fs.createDirectory(appDir);
        await vscode.workspace.fs.writeFile(
            vscode.Uri.joinPath(appDir, APP_MANIFEST_NAMES[0]), Buffer.from(toml, 'utf8')
        );
        await vscode.workspace.fs.writeFile(
            vscode.Uri.joinPath(appDir, `${name.trim()}.titan`), Buffer.from(mainTs, 'utf8')
        );
        const doc = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(appDir, `${name.trim()}.titan`));
        await vscode.window.showTextDocument(doc);
        desktopProvider.refresh();
        explorerProvider.refresh();
        vscode.window.showInformationMessage(`Omnisystem: Desktop app "${name.trim()}" created.`);
    } catch (err) {
        vscode.window.showErrorMessage(`Omnisystem: Failed to scaffold Desktop app: ${err}`);
    }
}

async function scaffoldOmniOsService(): Promise<void> {
    const name = await vscode.window.showInputBox({
        prompt: 'OmniOS service name',
        placeHolder: 'MyService',
        validateInput: (v) => v.trim() ? null : 'Name is required',
    });
    if (!name) { return; }

    const folders = vscode.workspace.workspaceFolders;
    if (!folders) {
        vscode.window.showErrorMessage('Omnisystem: No workspace folder open.');
        return;
    }

    const template =
`// ${name}.titan — OmniOS Service
service ${name} {
    fn start() {
        // Service initialization
    }
    fn stop() {
        // Graceful shutdown
    }
    fn handle(event: OsEvent) {
        // Event dispatch
    }
}
`;
    const targetUri = vscode.Uri.joinPath(folders[0].uri, 'src', 'systems', `${name}.titan`);

    try {
        // Ensure directory exists
        await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(folders[0].uri, 'src', 'systems'));
        await vscode.workspace.fs.writeFile(targetUri, Buffer.from(template, 'utf8'));
        const doc = await vscode.workspace.openTextDocument(targetUri);
        await vscode.window.showTextDocument(doc);
        explorerProvider.refresh();
        omniosProvider.refresh();
    } catch (err) {
        vscode.window.showErrorMessage(`Omnisystem: Failed to create OmniOS service: ${err}`);
    }
}

// ─── Dashboard panel commands ─────────────────────────────────────────────────

function cmdOpenBuildDashboard(): void {
    BuildDashboardPanel.createOrShow(extensionContext.extensionUri);
}

function cmdOpenMlStudio(): void {
    MlStudioPanel.createOrShow(extensionContext.extensionUri);
}

function cmdOpenShaderPreview(): void {
    ShaderPreviewPanel.createOrShow(extensionContext.extensionUri);
}

function cmdOpenLayoutPreview(): void {
    LayoutPreviewPanel.createOrShow(extensionContext.extensionUri);
}

function cmdOpenOmniOsDesktop(): void {
    OmniOSDesktopPanel.createOrShow(extensionContext.extensionUri, extensionContext);
}

function cmdOpenWidgetGallery(): void {
    WidgetGalleryPanel.createOrShow(extensionContext.extensionUri);
}

function cmdOpenWelcome(): void {
    WelcomePanel.createOrShow(extensionContext.extensionUri);
}

// ─── OmniCC editor commands ───────────────────────────────────────────────────

async function cmdOmniccConvertSelection(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { vscode.window.showWarningMessage('OmniCC: No active editor.'); return; }
    const selection = editor.selection;
    const source = editor.document.getText(selection.isEmpty ? undefined : selection);
    if (!source.trim()) { vscode.window.showWarningMessage('OmniCC: Nothing to convert.'); return; }

    const { quickConvert } = await import('./omnicc/ConversionEngine');
    const { allLanguages } = await import('./omnicc/LanguageRegistry');
    const langs = allLanguages();
    const picks = langs.map(l => ({ label: l.name, description: l.id, detail: l.family }));
    const picked = await vscode.window.showQuickPick(picks, { title: 'OmniCC — Convert to…', matchOnDescription: true, matchOnDetail: true });
    if (!picked) { return; }

    const srcLangId = editor.document.languageId;
    const result = quickConvert(source, picked.description!, srcLangId, editor.document.fileName);
    if (!result.success) { vscode.window.showErrorMessage(`OmniCC conversion failed: ${result.error ?? 'unknown error'}`); return; }

    const doc = await vscode.workspace.openTextDocument({ content: result.output, language: picked.description! });
    await vscode.window.showTextDocument(doc, { viewColumn: vscode.ViewColumn.Beside, preview: false });
}

async function cmdOmniccConvertFile(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { vscode.window.showWarningMessage('OmniCC: No active editor.'); return; }
    await _omniccConvertDocument(editor.document);
}

async function cmdOmniccConvertFileAs(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { vscode.window.showWarningMessage('OmniCC: No active editor.'); return; }

    const { allLanguages } = await import('./omnicc/LanguageRegistry');
    const langs = allLanguages();
    const picks = langs.map(l => ({ label: l.name, description: l.id, detail: l.family }));
    const picked = await vscode.window.showQuickPick(picks, { title: 'OmniCC — Convert file to…', matchOnDescription: true, matchOnDetail: true });
    if (!picked) { return; }

    await _omniccConvertDocument(editor.document, picked.description!);
}

async function _omniccConvertDocument(doc: vscode.TextDocument, targetLangId?: string): Promise<void> {
    const { quickConvert } = await import('./omnicc/ConversionEngine');
    const { allLanguages } = await import('./omnicc/LanguageRegistry');

    let tgtLang = targetLangId;
    if (!tgtLang) {
        const langs = allLanguages();
        const picks = langs.map(l => ({ label: l.name, description: l.id, detail: l.family }));
        const picked = await vscode.window.showQuickPick(picks, { title: 'OmniCC — Convert to…', matchOnDescription: true, matchOnDetail: true });
        if (!picked) { return; }
        tgtLang = picked.description!;
    }

    const source = doc.getText();
    const result = quickConvert(source, tgtLang, doc.languageId, doc.fileName);
    if (!result.success) { vscode.window.showErrorMessage(`OmniCC conversion failed: ${result.error ?? 'unknown error'}`); return; }

    const outDoc = await vscode.workspace.openTextDocument({ content: result.output, language: tgtLang });
    await vscode.window.showTextDocument(outDoc, { viewColumn: vscode.ViewColumn.Beside, preview: false });
    vscode.window.showInformationMessage(`OmniCC: Converted ${doc.languageId} → ${tgtLang} (${result.output.split('\n').length} lines)`);
}

// ─── Theme IPC broadcast ──────────────────────────────────────────────────────

export function broadcastTheme(themeId: string): void {
    const msg = { type: 'owThemeSync', theme: themeId };
    const senders = [
        () => OmnisystemDashboardPanel.postMessage(msg),
        () => BuildDashboardPanel.postMessage(msg),
        () => MlStudioPanel.postMessage(msg),
        () => ShaderPreviewPanel.postMessage(msg),
        () => LayoutPreviewPanel.postMessage(msg),
        () => WidgetGalleryPanel.postMessage(msg),
        () => WelcomePanel.postMessage(msg),
        () => WidgetConverterPanel.postMessage(msg),
        () => OmniCCDashboardPanel.postMessage(msg),
    ];
    for (const send of senders) {
        try { send(); } catch { /* panel may have been disposed */ }
    }
    extensionContext.globalState.update('omnisystem.owTheme', themeId).then(
        undefined, (e) => outputChannel.appendLine(`[Theme IPC] Persist error: ${e}`)
    );
    outputChannel.appendLine(`[Theme IPC] Broadcast: ${themeId}`);
}

function registerThemeIpcListener(): void {
    OmniOSDesktopPanel.onThemeChange = broadcastTheme;
}

// ─── VS Code workspace control commands ──────────────────────────────────────

async function cmdApplyOmnisystemTheme(): Promise<void> {
    await vscode.workspace.getConfiguration('workbench').update(
        'colorTheme', 'Omnisystem Dark', true
    );
    vscode.window.showInformationMessage('Omnisystem: Theme applied.');
}

async function cmdOptimizeLayout(): Promise<void> {
    const wsConfig = vscode.workspace.getConfiguration();
    await wsConfig.update('editor.minimap.enabled',          true, vscode.ConfigurationTarget.Workspace);
    await wsConfig.update('editor.stickyScroll.enabled',     true, vscode.ConfigurationTarget.Workspace);
    await wsConfig.update('editor.guides.bracketPairs',      true, vscode.ConfigurationTarget.Workspace);
    await wsConfig.update('workbench.sideBar.location',      'left', vscode.ConfigurationTarget.Workspace);
    await wsConfig.update('editor.renderWhitespace',         'boundary', vscode.ConfigurationTarget.Workspace);
    // Split editor for dual-pane workflow
    await vscode.commands.executeCommand('workbench.action.splitEditorRight');
    await vscode.commands.executeCommand('workbench.action.focusSideBar');
    vscode.window.showInformationMessage('Omnisystem: Layout optimized for Omni-language development.');
}

function cmdToggleZenMode(): void {
    vscode.commands.executeCommand('workbench.action.toggleZenMode');
}

function cmdFocusExplorer(): void {
    vscode.commands.executeCommand('omnisystemExplorer.focus');
}

function cmdSplitEditorVertical(): void {
    vscode.commands.executeCommand('workbench.action.splitEditorRight');
}

function cmdShowAllSystems(): void {
    vscode.commands.executeCommand('omniOsExplorer.focus');
    omniosProvider.refresh();
}

// ─── Command registration ─────────────────────────────────────────────────────

function registerCommands(ctx: vscode.ExtensionContext): void {
    const reg = (id: string, handler: (...args: unknown[]) => unknown) =>
        ctx.subscriptions.push(vscode.commands.registerCommand(id, handler));

    // Build & Run
    reg('omnisystem.build',          () => cmdBuild());
    reg('omnisystem.buildRelease',   () => cmdBuildRelease());
    reg('omnisystem.buildWasm',      () => cmdBuildWasm());
    reg('omnisystem.buildLinux',     () => cmdBuildLinux());
    reg('omnisystem.buildWatch',     () => cmdBuildWatch());
    reg('omnisystem.buildFile',      () => cmdBuildFile());
    reg('omnisystem.runFile',        () => cmdRunFile());
    reg('omnisystem.runTitanBootstrap', () => cmdRunTitanBootstrap());
    reg('omnisystem.clean',          () => cmdClean());
    reg('omnisystem.run',            () => cmdRun());
    reg('omnisystem.test',           () => cmdTest());
    reg('omnisystem.benchmark',      () => cmdBenchmark());
    reg('omnisystem.profile',        () => cmdProfile());

    // LSP / Editor
    reg('omnisystem.restartLsp',           () => cmdRestartLsp());
    reg('omnisystem.showOutputChannel',    () => outputChannel.show(true));
    reg('omnisystem.formatDocument',       () => cmdFormatDocument());
    reg('omnisystem.organizeImports',      () => cmdOrganizeImports());
    reg('omnisystem.generateDocs',         () => cmdGenerateDocs());
    reg('omnisystem.runAxiomVerification', () => cmdRunAxiomVerification());
    reg('omnisystem.showTypeHierarchy',    () => cmdShowTypeHierarchy());
    reg('omnisystem.inlayHintsToggle',     () => cmdInlayHintsToggle());

    // OmniOS
    reg('omnisystem.omniOsBoot',            () => { cmdOmniOsBoot().catch(e => outputChannel.appendLine(`[Sandbox] Error: ${e}`)); });
    reg('omnisystem.omniOsSystemServices',  () => cmdOmniOsSystemServices());
    reg('omnisystem.omniOsKernelLog',       () => cmdOmniOsKernelLog());
    reg('omnisystem.omniOsSelectPlatform',  () => cmdOmniOsSelectPlatform());
    reg('omnisystem.omniOsPackageOs',       () => cmdOmniOsPackageOs());
    reg('omnisystem.openSystemsExplorer',   () => cmdOpenSystemsExplorer());

    // OmniPM
    reg('omnisystem.omnipmInstall',  () => cmdOmniPMInstall());
    reg('omnisystem.omnipmAdd',      () => cmdOmniPMAdd());
    reg('omnisystem.omnipmRemove',   () => cmdOmniPMRemove());
    reg('omnisystem.omnipmUpdate',   () => cmdOmniPMUpdate());
    reg('omnisystem.omnipmPublish',  () => cmdOmniPMPublish());
    reg('omnisystem.omnipmSearch',   () => cmdOmniPMSearch());
    reg('omnisystem.omnipmAudit',    () => cmdOmniPMAudit());

    // Desktop
    reg('omnisystem.desktopLaunch',           () => cmdDesktopLaunch());
    reg('omnisystem.desktopBuddyConnect',     () => cmdDesktopBuddyConnect());
    reg('omnisystem.desktopBuddyBuild',       () => cmdDesktopBuddyBuild());
    reg('omnisystem.desktopBrowserExtBuild',  () => cmdDesktopBrowserExtBuild());
    reg('omnisystem.desktopBrowserExtInstall',() => cmdDesktopBrowserExtInstall());
    reg('omnisystem.desktopControlPanel',     () => cmdDesktopControlPanel());
    reg('omnisystem.desktopDeploy',           () => cmdDesktopDeploy());
    reg('omnisystem.desktopModelManager',     () => cmdDesktopModelManager());
    reg('omnisystem.desktopWorkspace',        () => cmdDesktopWorkspace());
    reg('omnisystem.desktopNotifications',    () => cmdDesktopNotifications());
    reg('omnisystem.desktopSystemTray',       () => cmdDesktopSystemTray());
    reg('omnisystem.desktopInit',             () => { cmdDesktopInit().catch(e => outputChannel.appendLine(`[Desktop Init] ${e}`)); });
    reg('omnisystem.openDesktopDashboard',    () => cmdOpenDesktopDashboard());

    // New file scaffolding (all 9 types)
    reg('omnisystem.newTitanFile',       () => scaffoldFile('titan'));
    reg('omnisystem.newVeraComponent',   () => scaffoldFile('vera'));
    reg('omnisystem.newHelixShader',     () => scaffoldFile('helix'));
    reg('omnisystem.newAetherActor',     () => scaffoldFile('aether'));
    reg('omnisystem.newAxiomTheorem',    () => scaffoldFile('axiom'));
    reg('omnisystem.newSylvaModel',      () => scaffoldFile('sylva'));
    reg('omnisystem.newNexusLayout',     () => scaffoldFile('nexus'));
    reg('omnisystem.newDesktopApp',       () => scaffoldDesktopApp());
    reg('omnisystem.newOmniOsService',   () => scaffoldOmniOsService());

    // Dashboard panels
    reg('omnisystem.openBuildDashboard', () => cmdOpenBuildDashboard());
    reg('omnisystem.openMlStudio',       () => cmdOpenMlStudio());
    reg('omnisystem.openShaderPreview',  () => cmdOpenShaderPreview());
    reg('omnisystem.openLayoutPreview',  () => cmdOpenLayoutPreview());
    reg('omnisystem.openOmniOsDesktop',  () => cmdOpenOmniOsDesktop());
    reg('omnisystem.widgetGallery',      () => cmdOpenWidgetGallery());
    reg('omnisystem.widgetConverter',    () => WidgetConverterPanel.createOrShow(ctx.extensionUri));
    reg('omnisystem.omniCC',             () => OmniCCDashboardPanel.createOrShow(ctx.extensionUri));
    reg('omnisystem.openWelcome',        () => cmdOpenWelcome());
    // Internal IPC command — triggered by panels when user changes OW theme
    reg('omnisystem._broadcastTheme',    (themeId: unknown) => {
        if (typeof themeId === 'string') { broadcastTheme(themeId); }
    });

    // VS Code control
    reg('omnisystem.applyOmnisystemTheme', () => cmdApplyOmnisystemTheme());
    reg('omnisystem.optimizeLayout',       () => cmdOptimizeLayout());
    reg('omnisystem.toggleZenMode',        () => cmdToggleZenMode());
    reg('omnisystem.focusExplorer',        () => cmdFocusExplorer());
    reg('omnisystem.splitEditorVertical',  () => cmdSplitEditorVertical());
    reg('omnisystem.showAllSystems',       () => cmdShowAllSystems());

    // OmniHarness AI
    reg('omnisystem.substrateSwarm',     () => cmdSubstrateSwarm());
    reg('omnisystem.substrateEnsemble',  () => cmdSubstrateEnsemble());
    reg('omnisystem.harnessFocus',       () => vscode.commands.executeCommand('omniharnessChat.focus'));
    reg('omnisystem.harnessNewSession',  () => { vscode.commands.executeCommand('omniharnessChat.focus'); harnessProvider?.newSession(); });
    reg('omnisystem.harnessSettings',    () => vscode.commands.executeCommand('workbench.action.openSettings', 'omnisystem.harness'));
    reg('omnisystem.harnessStartServer', async () => { await harnessProvider?.startServerCommand(); desktopProvider.refresh(); });
    reg('omnisystem.harnessStopServer',  () => { harnessProvider?.stopServerCommand(); desktopProvider.refresh(); });
    reg('omnisystem.harnessAddSelection',() => { vscode.commands.executeCommand('omniharnessChat.focus'); harnessProvider?.addSelectionCommand(); });
    reg('omnisystem.harnessUndoLast',    () => harnessProvider?.undoLastCommand());
    reg('omnisystem.harnessExportConfig',() => harnessProvider?.exportConfigCommand());
    reg('omnisystem.harnessImportConfig',() => harnessProvider?.importConfigCommand());
    reg('omnisystem.harnessCompactNow',  () => harnessProvider?.compactNowCommand());

    // Tree view refresh
    reg('omnisystem.refreshExplorer', () => explorerProvider.refresh());
    reg('omnisystem.refreshDesktop',   () => desktopProvider.refresh());
    reg('omnisystem.refreshOmnipm',   () => omnipmProvider.refresh());
    reg('omnisystem.refreshOmniOs',   () => omniosProvider.refresh());

    // Context menu item commands
    reg('omnisystem.packageUpdate', (pkg: unknown) => {
        const name = typeof pkg === 'string' ? pkg : (pkg as { label?: string })?.label ?? '';
        if (name) { runInTerminal(['pm', 'update', name]); }
    });
    reg('omnisystem.packageRemove', (pkg: unknown) => {
        const name = typeof pkg === 'string' ? pkg : (pkg as { label?: string })?.label ?? '';
        if (name) { runInTerminal(['pm', 'remove', name]); }
    });

    // File open helper
    reg('omnisystem.openFile', async (uri: unknown) => {
        if (uri instanceof vscode.Uri) {
            const doc = await vscode.workspace.openTextDocument(uri);
            await vscode.window.showTextDocument(doc);
        }
    });

    // Legacy aliases (keep existing callers working)
    reg('omnisystem.buildDashboard',  () => cmdOpenBuildDashboard());
    reg('omnisystem.desktopDashboard', () => cmdOpenDesktopDashboard());
    reg('omnisystem.selectPlatform',  () => cmdOmniOsSelectPlatform());
    reg('omnisystem.check',           async () => { if (await requireOmnicc()) { runInTerminal(['check']); } });
    reg('omnisystem.format',          async () => { if (await requireOmnicc()) { runInTerminal(['fmt', '--all']); } });
    reg('omnisystem.desktopBuild',     () => runRawInTerminal('npm run build', 'Desktop: Build'));
    reg('omnisystem.desktopBuildApk',  () => runRawInTerminal('npm run build:android', 'Desktop: Android Build'));
    // Old stub names remapped to correct handlers
    reg('omnisystem.newAetherService',() => scaffoldFile('aether'));
    reg('omnisystem.newAxiomModule',  () => scaffoldFile('axiom'));

    // OmniCC editor commands
    reg('omnisystem.omniccConvertSelection', () => cmdOmniccConvertSelection());
    reg('omnisystem.omniccConvertFile',      () => cmdOmniccConvertFile());
    reg('omnisystem.omniccConvertFileAs',    () => cmdOmniccConvertFileAs());
}

// ─── Code actions provider ────────────────────────────────────────────────────

class OmniCodeActionsProvider implements vscode.CodeActionProvider {
    provideCodeActions(
        document: vscode.TextDocument,
        range: vscode.Range,
        context: vscode.CodeActionContext,
    ): vscode.CodeAction[] {
        const actions: vscode.CodeAction[] = [];

        for (const diag of context.diagnostics) {
            // "unknown symbol X" → offer import
            const unknownSymbol = diag.message.match(/unknown symbol ['`]?(\w+)['`]?/i);
            if (unknownSymbol) {
                const sym    = unknownSymbol[1];
                const action = new vscode.CodeAction(
                    `Add import for '${sym}'`,
                    vscode.CodeActionKind.QuickFix
                );
                action.diagnostics = [diag];
                action.edit        = new vscode.WorkspaceEdit();
                // Insert import at top of file
                action.edit.insert(
                    document.uri,
                    new vscode.Position(0, 0),
                    `import ${sym};\n`
                );
                action.isPreferred = true;
                actions.push(action);
            }

            // "unused variable X" → prefix with _
            const unusedVar = diag.message.match(/unused (?:variable|binding) ['`]?(\w+)['`]?/i);
            if (unusedVar) {
                const varName = unusedVar[1];
                const action  = new vscode.CodeAction(
                    `Prefix '${varName}' with _ to suppress warning`,
                    vscode.CodeActionKind.QuickFix
                );
                action.diagnostics = [diag];
                action.edit        = new vscode.WorkspaceEdit();
                const lineText     = document.lineAt(diag.range.start.line).text;
                const idx          = lineText.indexOf(varName);
                if (idx >= 0) {
                    action.edit.replace(
                        document.uri,
                        new vscode.Range(
                            diag.range.start.line, idx,
                            diag.range.start.line, idx + varName.length
                        ),
                        `_${varName}`
                    );
                }
                actions.push(action);
            }

            // "missing semicolon" → insert
            if (/missing semicolon/i.test(diag.message)) {
                const action = new vscode.CodeAction('Insert missing semicolon', vscode.CodeActionKind.QuickFix);
                action.diagnostics = [diag];
                action.edit        = new vscode.WorkspaceEdit();
                const pos          = diag.range.end;
                action.edit.insert(document.uri, pos, ';');
                action.isPreferred = true;
                actions.push(action);
            }
        }

        return actions;
    }
}

// ─── Semantic tokens provider ─────────────────────────────────────────────────

// Keywords per language (fallback when LSP is unavailable)
const TITAN_KEYWORDS  = /\b(fn|mod|pub|let|mut|if|else|while|for|return|struct|enum|impl|use|import|type|const|static|async|await|spawn)\b/g;
const VERA_KEYWORDS   = /\b(component|props|render|on|state|effect|ref|slot)\b/g;
const HELIX_KEYWORDS  = /\b(shader|vertex|fragment|uniform|varying|in|out|fn|let|var|return|if|else|for|while|struct)\b/g;
const AETHER_KEYWORDS = /\b(actor|message|handler|state|send|spawn|await|service|async|fn|let)\b/g;
const AXIOM_KEYWORDS  = /\b(theorem|proof|assert|assume|forall|exists|preconditions|postconditions|assertions|lemma)\b/g;
const SYLVA_KEYWORDS  = /\b(model|layer|loss|optimizer|train|predict|architecture|activation|dropout|batch)\b/g;
const NEXUS_KEYWORDS  = /\b(layout|breakpoints|flex|grid|column|row|gap|align|justify|responsive)\b/g;

const KEYWORD_PATTERNS: Record<string, RegExp> = {
    titan:  TITAN_KEYWORDS,
    vera:   VERA_KEYWORDS,
    helix:  HELIX_KEYWORDS,
    aether: AETHER_KEYWORDS,
    axiom:  AXIOM_KEYWORDS,
    sylva:  SYLVA_KEYWORDS,
    nexus:  NEXUS_KEYWORDS,
};

class OmniSemanticTokensProvider implements vscode.DocumentSemanticTokensProvider {
    constructor(private readonly legend: vscode.SemanticTokensLegend) {}

    provideDocumentSemanticTokens(document: vscode.TextDocument): vscode.SemanticTokens {
        const builder = new vscode.SemanticTokensBuilder(this.legend);
        const lang    = document.languageId;
        const pattern = KEYWORD_PATTERNS[lang];
        if (!pattern) { return builder.build(); }

        const kwIndex = this.legend.tokenTypes.indexOf('keyword');
        if (kwIndex < 0) { return builder.build(); }

        for (let i = 0; i < document.lineCount; i++) {
            const line = document.lineAt(i).text;
            // Reset regex lastIndex per line
            const rx   = new RegExp(pattern.source, pattern.flags.replace('g', '') + 'g');
            let match: RegExpExecArray | null;
            while ((match = rx.exec(line)) !== null) {
                builder.push(i, match.index, match[0].length, kwIndex, 0);
            }
        }

        return builder.build();
    }
}

// ─── Welcome notification ─────────────────────────────────────────────────────

function showWelcome(): void {
    WelcomePanel.createOrShow(extensionContext.extensionUri);
}
